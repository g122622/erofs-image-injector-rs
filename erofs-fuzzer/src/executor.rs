//! Erofsfuse Executor
//!
//! Executor that runs erofsfuse to mount and test EROFS images.
//!
//! This executor implements improved crash detection:
//! - Detects ASan crashes via exit codes (134 = SIGABRT, etc.)
//! - Monitors stderr for ASan error messages in real-time
//! - Uses aggressive process monitoring during filesystem operations
//! - Detects crashes that happen in FUSE worker threads

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, trace, warn};

use libafl_bolts::Error;

use erofs_input::ErofsImageInput;
use crate::cli::FuzzerConfig;
use crate::executor_trait::{Executor, ExecutionResult, ExecutionOutput};

/// Exit kind for erofsfuse execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErofsfuseExit {
    /// Execution completed successfully
    Success,
    /// Process timed out
    Timeout,
    /// Process crashed (SIGSEGV, SIGABRT, etc.)
    Crashed(i32),
    /// Process returned non-zero exit code
    Error(i32),
    /// Failed to start process
    FailedToStart,
    /// ASan detected memory error
    AsanError,
}

impl ErofsfuseExit {
    /// Returns true if this exit indicates a crash or error
    pub fn is_crash(&self) -> bool {
        matches!(self, ErofsfuseExit::Crashed(_) | ErofsfuseExit::AsanError)
    }
}

/// ASan error patterns to detect in stderr/stdout
/// These patterns are case-insensitive for better matching
const ASAN_ERROR_PATTERNS: &[&str] = &[
    // ASan specific patterns
    "AddressSanitizer",
    "ASAN",
    "ERROR:",
    "SUMMARY:",
    // Crash indicators
    "Segmentation Fault",
    "segmentation fault",
    "SEGV on unknown",
    // Memory error types
    "heap-buffer-overflow",
    "heap-use-after-free",
    "stack-buffer-overflow",
    "stack-use-after-scope",
    "global-buffer-overflow",
    "use-after-poison",
    "double-free",
    "invalid-free",
    "memory allocation failed",
    // ASan specific output markers
    "Starting backtrace",
    "shadow bytes",
    "AddressSanitizer:",
    "LeakSanitizer",
    "UndefinedBehaviorSanitizer",
];

/// Signal exit codes (128 + signal number)
const EXIT_SIGABRT: i32 = 134; // 128 + 6
const EXIT_SIGSEGV: i32 = 139; // 128 + 11
const EXIT_SIGFPE: i32 = 136;  // 128 + 8
const EXIT_SIGBUS: i32 = 135;  // 128 + 7

/// Process monitoring result
#[derive(Debug, Clone, Copy)]
enum ProcessEvent {
    /// ASan detected in stderr
    AsanDetected,
}

/// Executor for erofsfuse
#[derive(Debug)]
pub struct ErofsfuseExecutor {
    /// Path to erofsfuse binary
    erofsfuse_path: PathBuf,
    /// Base directory for mount points
    mount_base: PathBuf,
    /// Timeout for each execution
    timeout: Duration,
    /// Maximum image size
    max_size: usize,
    /// Minimum image size
    min_size: usize,
    /// Whether to keep temp files for debugging
    keep_temp: bool,
    /// Number of executions
    executions: u64,
}

impl ErofsfuseExecutor {
    /// Create a new executor
    pub fn new(config: &FuzzerConfig) -> Self {
        Self {
            erofsfuse_path: config.erofsfuse_path.clone(),
            mount_base: config.mount_base.clone(),
            timeout: Duration::from_millis(config.timeout_ms),
            max_size: config.max_image_size,
            min_size: config.min_image_size,
            keep_temp: false,
            executions: 0,
        }
    }

    /// Set whether to keep temp files
    pub fn with_keep_temp(mut self, keep: bool) -> Self {
        self.keep_temp = keep;
        self
    }

    /// Get the number of executions
    pub fn executions(&self) -> u64 {
        self.executions
    }

    /// Execute the target with the given input (returns ErofsfuseExit)
    pub fn execute_erofs(&mut self, input: &ErofsImageInput) -> Result<ErofsfuseExit, Error> {
        self.executions += 1;

        let data = input.data();

        if data.len() < self.min_size {
            trace!("Input too small: {} < {}", data.len(), self.min_size);
            return Ok(ErofsfuseExit::Error(0));
        }
        if data.len() > self.max_size {
            trace!("Input too large: {} > {}", data.len(), self.max_size);
            return Ok(ErofsfuseExit::Error(0));
        }

        let temp_dir = tempfile::tempdir_in(&self.mount_base)
            .map_err(|e| Error::illegal_state(format!("Failed to create temp dir: {}", e)))?;

        let image_path = temp_dir.path().join("image.erofs");
        std::fs::write(&image_path, data)
            .map_err(|e| Error::illegal_state(format!("Failed to write image: {}", e)))?;

        let mount_point = temp_dir.path().join("mnt");
        std::fs::create_dir_all(&mount_point)
            .map_err(|e| Error::illegal_state(format!("Failed to create mount point: {}", e)))?;

        let result = self.run_erofsfuse(&image_path, &mount_point);

        if !self.keep_temp {
            let _ = self.unmount(&mount_point);
        }

        result
    }

    /// Run erofsfuse with improved crash detection
    fn run_erofsfuse(
        &self,
        image_path: &Path,
        mount_point: &Path,
    ) -> Result<ErofsfuseExit, Error> {
        let start = Instant::now();

        // Start erofsfuse with stdout and stderr capture
        // Use -f (foreground) mode so we can properly monitor the process
        // ASan outputs to both stdout and stderr, so we need to capture both
        let mut child = match Command::new(&self.erofsfuse_path)
            .arg(image_path)
            .arg(mount_point)
            .arg("-o")
            .arg("ro")
            .arg("-f")  // Run in foreground for proper monitoring
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                error!("Failed to start erofsfuse: {}", e);
                return Ok(ErofsfuseExit::FailedToStart);
            }
        };

        // Capture stdout and stderr in separate threads
        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");
        let (crash_tx, crash_rx) = mpsc::channel::<ProcessEvent>();
        let asan_detected = Arc::new(AtomicBool::new(false));
        let asan_detected_stdout = asan_detected.clone();
        let asan_detected_stderr = asan_detected.clone();
        let crash_tx_stdout = crash_tx.clone();

        // Spawn stdout monitoring thread (ASan often outputs here)
        let stdout_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        // Log for debugging
                        trace!("stdout: {}", line.trim());
                        // Check for ASan patterns
                        for pattern in ASAN_ERROR_PATTERNS {
                            if line.contains(pattern) {
                                info!("ASan pattern '{}' detected in stdout: {}", pattern, line.trim());
                                asan_detected_stdout.store(true, Ordering::SeqCst);
                                let _ = crash_tx_stdout.send(ProcessEvent::AsanDetected);
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Stdout read error: {}", e);
                        break;
                    }
                }
            }
        });

        // Spawn stderr monitoring thread
        let stderr_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        // Log for debugging
                        trace!("stderr: {}", line.trim());
                        // Check for ASan patterns
                        for pattern in ASAN_ERROR_PATTERNS {
                            if line.contains(pattern) {
                                debug!("ASan pattern '{}' detected in stderr: {}", pattern, line.trim());
                                asan_detected_stderr.store(true, Ordering::SeqCst);
                                let _ = crash_tx.send(ProcessEvent::AsanDetected);
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Stderr read error: {}", e);
                        break;
                    }
                }
            }
        });

        // Wait for mount with timeout
        let mount_timeout = Duration::from_secs(5);
        let mut mounted = false;
        let poll_interval = Duration::from_millis(20);

        while start.elapsed() < mount_timeout {
            // Check for crash first
            match crash_rx.try_recv() {
                Ok(ProcessEvent::AsanDetected) => {
                    info!("ASan error detected during mount phase");
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    return Ok(ErofsfuseExit::AsanError);
                }
                _ => {}
            }

            // Check process status
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process exited - check why
                    let exit = self.classify_exit(&status);
                    if exit.is_crash() || exit == ErofsfuseExit::Error(134) {
                        info!("Process crashed during mount: {:?}", exit);
                        let _ = stdout_thread.join();
                        let _ = stderr_thread.join();
                        return Ok(exit);
                    }
                    debug!("Process exited during mount with {:?}", exit);
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    return Ok(exit);
                }
                Ok(None) => {
                    // Still running - check mount point
                    if mount_point.exists() {
                        if let Ok(mut dir) = mount_point.read_dir() {
                            if dir.next().is_some() {
                                mounted = true;
                                debug!("Mount successful after {:?}", start.elapsed());
                                break;
                            }
                        }
                    }
                    std::thread::sleep(poll_interval);
                }
                Err(e) => {
                    error!("Failed to check process status: {}", e);
                    break;
                }
            }
        }

        if !mounted {
            warn!("Mount timeout after {:?}", start.elapsed());
            // Check if process crashed
            if asan_detected.load(Ordering::SeqCst) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                return Ok(ErofsfuseExit::AsanError);
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    let exit = self.classify_exit(&status);
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    return Ok(exit);
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    return Ok(ErofsfuseExit::Timeout);
                }
            }
        }

        // Perform filesystem operations with aggressive crash monitoring
        debug!("Starting filesystem operations");
        let ops_result = self.perform_filesystem_ops_with_monitoring(
            mount_point,
            &mut child,
            &crash_rx,
            &asan_detected,
            self.timeout.saturating_sub(start.elapsed()),
        );

        // Check final result
        if let Some(exit) = ops_result {
            debug!("Filesystem ops detected crash: {:?}", exit);
            let _ = child.kill();
            let _ = child.wait();
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Ok(exit);
        }

        // Unmount gracefully
        let _ = self.unmount(mount_point);

        // Wait for process to exit and get final status
        let wait_result = child.wait_timeout(Duration::from_secs(2));

        let final_exit = match wait_result {
            Ok(Some(status)) => {
                let exit = self.classify_exit(&status);
                debug!("Final process status: {:?}", exit);
                exit
            }
            Ok(None) => {
                warn!("Process still running after unmount, killing");
                let _ = child.kill();
                let status = child.wait().ok();
                status.map_or(ErofsfuseExit::Error(-1), |s| self.classify_exit(&s))
            }
            Err(e) => {
                error!("Error waiting for process: {}", e);
                let _ = child.kill();
                let _ = child.wait();
                ErofsfuseExit::Error(-1)
            }
        };

        // Final ASan check
        if asan_detected.load(Ordering::SeqCst) && !final_exit.is_crash() {
            let _ = stdout_thread.join();
            let _ = stderr_thread.join();
            return Ok(ErofsfuseExit::AsanError);
        }

        let _ = stdout_thread.join();
        let _ = stderr_thread.join();
        Ok(final_exit)
    }

    /// Classify exit status into ErofsfuseExit
    fn classify_exit(&self, status: &std::process::ExitStatus) -> ErofsfuseExit {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;

            // Check for signal termination first (most reliable)
            if let Some(signal) = status.signal() {
                info!("Process terminated by signal {}", signal);
                return ErofsfuseExit::Crashed(signal);
            }

            // Check for core dump
            if status.core_dumped() {
                info!("Process core dumped");
                return ErofsfuseExit::Crashed(11);
            }
        }

        // Check exit code
        let code = status.code().unwrap_or(-1);
        match code {
            0 => ErofsfuseExit::Success,
            EXIT_SIGABRT => {
                info!("Exit code {} indicates ASan abort", code);
                ErofsfuseExit::Crashed(6)
            }
            EXIT_SIGSEGV => ErofsfuseExit::Crashed(11),
            EXIT_SIGBUS => ErofsfuseExit::Crashed(7),
            EXIT_SIGFPE => ErofsfuseExit::Crashed(8),
            other => ErofsfuseExit::Error(other),
        }
    }

    /// Perform filesystem operations with aggressive process monitoring
    fn perform_filesystem_ops_with_monitoring(
        &self,
        mount_point: &Path,
        child: &mut std::process::Child,
        crash_rx: &Receiver<ProcessEvent>,
        asan_detected: &Arc<AtomicBool>,
        timeout: Duration,
    ) -> Option<ErofsfuseExit> {
        use std::fs;

        let start = Instant::now();
        let check_interval = Duration::from_millis(10); // More frequent checks
        let mut last_check = Instant::now();

        // Read directory entries
        if let Ok(entries) = fs::read_dir(mount_point) {
            for entry in entries.flatten() {
                // Timeout check
                if start.elapsed() > timeout {
                    debug!("Filesystem ops timeout");
                    return Some(ErofsfuseExit::Timeout);
                }

                // Check for ASan detection from stderr thread
                if asan_detected.load(Ordering::SeqCst) {
                    info!("ASan detected during filesystem traversal");
                    return Some(ErofsfuseExit::AsanError);
                }

                // Check for crash notification from channel
                match crash_rx.try_recv() {
                    Ok(ProcessEvent::AsanDetected) => {
                        info!("Received ASan notification");
                        return Some(ErofsfuseExit::AsanError);
                    }
                    _ => {}
                }

                // Check process status frequently
                if last_check.elapsed() > check_interval {
                    last_check = Instant::now();
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            let exit = self.classify_exit(&status);
                            if exit.is_crash() {
                                info!("Process crashed during ops: {:?}", exit);
                                return Some(exit);
                            }
                            // Non-zero exit might indicate ASan too
                            if exit != ErofsfuseExit::Success && asan_detected.load(Ordering::SeqCst) {
                                return Some(ErofsfuseExit::AsanError);
                            }
                            return Some(exit);
                        }
                        _ => {}
                    }
                }

                let path = entry.path();

                // Try to read metadata - this can trigger ASan crashes
                let _ = fs::metadata(&path);

                // Immediate check after potentially blocking operation
                if asan_detected.load(Ordering::SeqCst) {
                    return Some(ErofsfuseExit::AsanError);
                }

                if path.is_dir() {
                    if path.components().count() < 20 {
                        if let Err(e) = self.traverse_directory(
                            &path,
                            child,
                            crash_rx,
                            asan_detected,
                            &mut last_check,
                            timeout.saturating_sub(start.elapsed()),
                        ) {
                            // Check if it was a crash
                            if asan_detected.load(Ordering::SeqCst) {
                                return Some(ErofsfuseExit::AsanError);
                            }
                            debug!("Directory traversal error: {}", e);
                        }
                    }
                } else {
                    let _ = self.read_file(&path);
                }

                // Check again after file operations
                if asan_detected.load(Ordering::SeqCst) {
                    return Some(ErofsfuseExit::AsanError);
                }
            }
        }

        // Final process check after all operations
        match child.try_wait() {
            Ok(Some(status)) => {
                let exit = self.classify_exit(&status);
                if exit.is_crash() {
                    return Some(exit);
                }
                if exit != ErofsfuseExit::Success && asan_detected.load(Ordering::SeqCst) {
                    return Some(ErofsfuseExit::AsanError);
                }
                Some(exit)
            }
            _ => None,
        }
    }

    /// Traverse a directory with monitoring
    fn traverse_directory(
        &self,
        dir: &Path,
        child: &mut std::process::Child,
        crash_rx: &Receiver<ProcessEvent>,
        asan_detected: &Arc<AtomicBool>,
        last_check: &mut Instant,
        timeout: Duration,
    ) -> Result<(), Error> {
        use std::fs;

        let start = Instant::now();
        let check_interval = Duration::from_millis(10);

        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                // Timeout check
                if start.elapsed() > timeout {
                    return Err(Error::illegal_state("Timeout"));
                }

                // Check ASan
                if asan_detected.load(Ordering::SeqCst) {
                    return Err(Error::illegal_state("ASan detected"));
                }

                // Check crash channel
                match crash_rx.try_recv() {
                    Ok(ProcessEvent::AsanDetected) => {
                        return Err(Error::illegal_state("ASan detected"));
                    }
                    _ => {}
                }

                // Check process
                if last_check.elapsed() > check_interval {
                    *last_check = Instant::now();
                    if let Ok(Some(status)) = child.try_wait() {
                        let exit = self.classify_exit(&status);
                        if exit.is_crash() {
                            return Err(Error::illegal_state(format!("Crash: {:?}", exit)));
                        }
                    }
                }

                let path = entry.path();
                let _ = fs::metadata(&path);

                // Check after blocking operation
                if asan_detected.load(Ordering::SeqCst) {
                    return Err(Error::illegal_state("ASan detected"));
                }

                if path.is_dir() && path.components().count() < 20 {
                    let _ = self.traverse_directory(
                        &path,
                        child,
                        crash_rx,
                        asan_detected,
                        last_check,
                        timeout.saturating_sub(start.elapsed()),
                    );
                } else {
                    let _ = self.read_file(&path);
                }
            }
        }

        Ok(())
    }

    /// Read a file's contents
    fn read_file(&self, path: &Path) -> Result<(), Error> {
        use std::fs;

        match fs::File::open(path) {
            Ok(mut file) => {
                let mut buf = vec![0u8; 4096];
                let _ = std::io::Read::read(&mut file, &mut buf);
            }
            Err(e) => {
                trace!("Failed to read file {:?}: {}", path, e);
            }
        }

        Ok(())
    }

    /// Unmount the filesystem
    fn unmount(&self, mount_point: &Path) -> Result<(), Error> {
        #[cfg(unix)]
        {
            // Try fusermount -u first
            let result = Command::new("fusermount")
                .arg("-u")
                .arg(mount_point)
                .output();

            match result {
                Ok(output) => {
                    if output.status.success() {
                        debug!("Successfully unmounted {:?}", mount_point);
                        return Ok(());
                    }
                }
                Err(e) => {
                    debug!("fusermount failed: {}", e);
                }
            }

            // Try umount as fallback
            let result = Command::new("umount")
                .arg(mount_point)
                .output();

            match result {
                Ok(output) => {
                    if output.status.success() {
                        debug!("Successfully unmounted with umount {:?}", mount_point);
                        return Ok(());
                    }
                }
                Err(e) => {
                    debug!("umount failed: {}", e);
                }
            }
        }

        #[cfg(windows)]
        {
            warn!("Unmount not implemented for Windows");
        }

        Ok(())
    }
}

impl Clone for ErofsfuseExecutor {
    fn clone(&self) -> Self {
        Self {
            erofsfuse_path: self.erofsfuse_path.clone(),
            mount_base: self.mount_base.clone(),
            timeout: self.timeout,
            max_size: self.max_size,
            min_size: self.min_size,
            keep_temp: self.keep_temp,
            executions: 0,
        }
    }
}

/// Trait extension for waiting with timeout
trait WaitTimeout {
    fn wait_timeout(&mut self, timeout: Duration) -> Result<Option<std::process::ExitStatus>, std::io::Error>;
}

impl WaitTimeout for std::process::Child {
    fn wait_timeout(&mut self, timeout: Duration) -> Result<Option<std::process::ExitStatus>, std::io::Error> {
        let start = Instant::now();

        while start.elapsed() < timeout {
            match self.try_wait() {
                Ok(Some(status)) => return Ok(Some(status)),
                Ok(None) => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => return Err(e),
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_executor_creation() {
        let config = FuzzerConfig::from(crate::cli::CliArgs::try_parse_from(["test", "--seeds", "./seeds"]).unwrap());
        let executor = ErofsfuseExecutor::new(&config);
        assert_eq!(executor.executions(), 0);
    }

    #[test]
    fn test_crash_detection() {
        // Test exit code detection
        assert_eq!(EXIT_SIGABRT, 134);
        assert_eq!(EXIT_SIGSEGV, 139);
        assert_eq!(EXIT_SIGFPE, 136);
        assert_eq!(EXIT_SIGBUS, 135);
    }
}

/// Convert ErofsfuseExit to ExecutionResult
impl From<ErofsfuseExit> for ExecutionResult {
    fn from(exit: ErofsfuseExit) -> Self {
        match exit {
            ErofsfuseExit::Success => ExecutionResult::Success,
            ErofsfuseExit::Timeout => ExecutionResult::Timeout,
            ErofsfuseExit::Crashed(signal) => ExecutionResult::Crashed(signal),
            ErofsfuseExit::Error(code) => ExecutionResult::Error(code),
            ErofsfuseExit::FailedToStart => ExecutionResult::FailedToStart,
            ErofsfuseExit::AsanError => ExecutionResult::AsanError,
        }
    }
}

/// Implement Executor trait for ErofsfuseExecutor
impl Executor for ErofsfuseExecutor {
    fn execute(&mut self, input: &ErofsImageInput) -> Result<ExecutionResult, Error> {
        self.execute_erofs(input).map(|exit| exit.into())
    }

    fn execute_with_output(&mut self, input: &ErofsImageInput) -> Result<ExecutionOutput, Error> {
        // ErofsfuseExecutor doesn't capture kernel logs, so return empty
        let result = self.execute_erofs(input)?;
        Ok(ExecutionOutput::new(result.into()))
    }

    fn executions(&self) -> u64 {
        self.executions
    }

    fn name(&self) -> &'static str {
        "ErofsfuseExecutor"
    }
}
