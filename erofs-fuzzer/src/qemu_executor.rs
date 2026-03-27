//! QEMU Kernel Executor
//!
//! Executor that runs EROFS images inside a QEMU virtual machine with a test kernel.
//! This allows testing the kernel-space EROFS driver instead of user-space erofsfuse.

use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, trace, warn};

use libafl_bolts::Error;

use erofs_input::ErofsImageInput;
use crate::executor_trait::{ExecutionResult, Executor, ExecutorConfig};
use crate::kernel_monitor::KernelMonitor;

/// QEMU-based kernel test executor
#[derive(Debug)]
pub struct QemuKernelExecutor {
    /// Path to QEMU binary
    qemu_path: PathBuf,
    /// Path to kernel image (bzImage)
    kernel_path: PathBuf,
    /// Path to initramfs
    initramfs_path: PathBuf,
    /// Extra QEMU arguments
    qemu_args: Vec<String>,
    /// Memory for QEMU (in MB)
    memory_mb: usize,
    /// Timeout for each execution
    timeout: Duration,
    /// Maximum image size
    max_size: usize,
    /// Minimum image size
    min_size: usize,
    /// Number of executions
    executions: u64,
    /// Kernel crash monitor
    kernel_monitor: KernelMonitor,
    /// Keep temp files for debugging
    keep_temp: bool,
    /// Temporary directory for test images
    temp_dir: Option<PathBuf>,
}

impl QemuKernelExecutor {
    /// Create a new QEMU kernel executor
    pub fn new(config: &ExecutorConfig) -> Self {
        Self {
            qemu_path: config.qemu_path.clone(),
            kernel_path: config.kernel_path.clone(),
            initramfs_path: config.initramfs_path.clone(),
            qemu_args: config.qemu_args.clone(),
            memory_mb: config.qemu_memory,
            timeout: config.timeout,
            max_size: config.max_size,
            min_size: config.min_size,
            executions: 0,
            kernel_monitor: KernelMonitor::new(),
            keep_temp: false,
            temp_dir: None,
        }
    }

    /// Create from individual parameters
    pub fn with_params(
        qemu_path: PathBuf,
        kernel_path: PathBuf,
        initramfs_path: PathBuf,
        timeout: Duration,
    ) -> Self {
        Self {
            qemu_path,
            kernel_path,
            initramfs_path,
            qemu_args: Vec::new(),
            memory_mb: 512,
            timeout,
            max_size: 16 * 1024 * 1024,
            min_size: 4096,
            executions: 0,
            kernel_monitor: KernelMonitor::new(),
            keep_temp: false,
            temp_dir: None,
        }
    }

    /// Set whether to keep temp files
    pub fn with_keep_temp(mut self, keep: bool) -> Self {
        self.keep_temp = keep;
        self
    }

    /// Set QEMU memory
    pub fn with_memory(mut self, memory_mb: usize) -> Self {
        self.memory_mb = memory_mb;
        self
    }

    /// Add extra QEMU arguments
    pub fn with_qemu_args(mut self, args: Vec<String>) -> Self {
        self.qemu_args = args;
        self
    }

    /// Get the number of executions
    pub fn executions(&self) -> u64 {
        self.executions
    }

    /// Check if kernel and initramfs exist
    pub fn check_requirements(&self) -> Result<(), Error> {
        if !self.qemu_path.exists() {
            return Err(Error::illegal_state(format!(
                "QEMU not found at {:?}",
                self.qemu_path
            )));
        }
        if !self.kernel_path.exists() {
            return Err(Error::illegal_state(format!(
                "Kernel not found at {:?}. Run scripts/build_kernel.sh first.",
                self.kernel_path
            )));
        }
        if !self.initramfs_path.exists() {
            return Err(Error::illegal_state(format!(
                "Initramfs not found at {:?}. Run scripts/build_kernel.sh first.",
                self.initramfs_path
            )));
        }
        Ok(())
    }

    /// Execute a test with the given EROFS image
    fn run_qemu(&self, image_path: &Path) -> Result<ExecutionResult, Error> {
        let start = Instant::now();

        // Build QEMU command
        let mut cmd = Command::new(&self.qemu_path);
        cmd.arg("-kernel")
            .arg(&self.kernel_path)
            .arg("-initrd")
            .arg(&self.initramfs_path)
            .arg("-drive")
            .arg(format!("file={},format=raw,if=virtio,readonly=on", image_path.display()))
            .arg("-append")
            .arg("console=ttyS0 panic=1 quiet")
            .arg("-nographic")
            .arg("-no-reboot")
            .arg("-m")
            .arg(format!("{}M", self.memory_mb))
            .arg("-smp")
            .arg("2");

        // Add extra QEMU arguments
        for arg in &self.qemu_args {
            cmd.arg(arg);
        }

        cmd.stdout(Stdio::piped())
            .stderr(Stdio::piped());

        debug!("Starting QEMU: {:?}", cmd);

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                error!("Failed to start QEMU: {}", e);
                return Ok(ExecutionResult::FailedToStart);
            }
        };

        // Capture stdout and stderr
        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");
        let (crash_tx, crash_rx) = mpsc::channel::<ExecutionResult>();
        let panic_detected = Arc::new(AtomicBool::new(false));
        let panic_detected_stdout = panic_detected.clone();
        let panic_detected_stderr = panic_detected.clone();
        let crash_tx_stdout = crash_tx.clone();

        // Monitor stdout thread
        let stdout_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        trace!("QEMU stdout: {}", line.trim());
                        // Check for kernel panic/oops
                        if let Some(issue) = KernelMonitor::check_line(&line) {
                            info!("Kernel issue detected in stdout: {:?}", issue);
                            panic_detected_stdout.store(true, Ordering::SeqCst);
                            let _ = crash_tx_stdout.send(issue);
                        }
                    }
                    Err(e) => {
                        debug!("QEMU stdout read error: {}", e);
                        break;
                    }
                }
            }
        });

        // Monitor stderr thread
        let stderr_thread = std::thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) => break,
                    Ok(_) => {
                        trace!("QEMU stderr: {}", line.trim());
                        if let Some(issue) = KernelMonitor::check_line(&line) {
                            info!("Kernel issue detected in stderr: {:?}", issue);
                            panic_detected_stderr.store(true, Ordering::SeqCst);
                            let _ = crash_tx.send(issue);
                        }
                    }
                    Err(e) => {
                        debug!("QEMU stderr read error: {}", e);
                        break;
                    }
                }
            }
        });

        // Wait for QEMU with timeout
        let timeout_remaining = self.timeout.saturating_sub(start.elapsed());
        let mut last_check = Instant::now();
        let check_interval = Duration::from_millis(50);

        loop {
            // Check for crash detection
            match crash_rx.try_recv() {
                Ok(result) => {
                    info!("Kernel issue detected: {:?}", result);
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    return Ok(result);
                }
                _ => {}
            }

            // Check process status
            if last_check.elapsed() > check_interval {
                last_check = Instant::now();
                match child.try_wait() {
                    Ok(Some(status)) => {
                        // Process exited
                        let _ = stdout_thread.join();
                        let _ = stderr_thread.join();

                        // Check if we detected a panic
                        if panic_detected.load(Ordering::SeqCst) {
                            return Ok(ExecutionResult::KernelPanic);
                        }

                        // Check exit status
                        #[cfg(unix)]
                        {
                            use std::os::unix::process::ExitStatusExt;
                            if let Some(signal) = status.signal() {
                                info!("QEMU terminated by signal {}", signal);
                                return Ok(ExecutionResult::Crashed(signal));
                            }
                        }

                        match status.code() {
                            Some(0) => {
                                debug!("QEMU exited successfully");
                                return Ok(ExecutionResult::Success);
                            }
                            Some(code) => {
                                debug!("QEMU exited with code {}", code);
                                return Ok(ExecutionResult::Error(code));
                            }
                            None => {
                                debug!("QEMU exited without status");
                                return Ok(ExecutionResult::Error(-1));
                            }
                        }
                    }
                    Ok(None) => {
                        // Still running
                    }
                    Err(e) => {
                        error!("Failed to check QEMU status: {}", e);
                    }
                }
            }

            // Check timeout
            if start.elapsed() > timeout_remaining {
                warn!("QEMU timeout after {:?}", start.elapsed());
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();

                if panic_detected.load(Ordering::SeqCst) {
                    return Ok(ExecutionResult::KernelPanic);
                }
                return Ok(ExecutionResult::Timeout);
            }

            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Executor for QemuKernelExecutor {
    fn execute(&mut self, input: &ErofsImageInput) -> Result<ExecutionResult, Error> {
        self.executions += 1;

        // Check requirements on first execution
        if self.executions == 1 {
            self.check_requirements()?;
        }

        let data = input.data();

        // Validate size
        if data.len() < self.min_size {
            trace!("Input too small: {} < {}", data.len(), self.min_size);
            return Ok(ExecutionResult::Error(0));
        }
        if data.len() > self.max_size {
            trace!("Input too large: {} > {}", data.len(), self.max_size);
            return Ok(ExecutionResult::Error(0));
        }

        // Create temporary file for the image
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::illegal_state(format!("Failed to create temp dir: {}", e)))?;

        let image_path = temp_dir.path().join("test.erofs");
        std::fs::write(&image_path, data)
            .map_err(|e| Error::illegal_state(format!("Failed to write image: {}", e)))?;

        debug!("Testing EROFS image: {:?}", image_path);

        let result = self.run_qemu(&image_path);

        // Clean up
        if !self.keep_temp {
            let _ = std::fs::remove_file(&image_path);
        } else {
            self.temp_dir = Some(temp_dir.into_path());
        }

        result
    }

    fn executions(&self) -> u64 {
        self.executions
    }

    fn name(&self) -> &'static str {
        "QemuKernelExecutor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qemu_executor_creation() {
        let config = ExecutorConfig::qemu(
            PathBuf::from("./kernel_build/bzImage"),
            PathBuf::from("./kernel_build/rootfs.cpio.gz"),
            60000,
        );
        let executor = QemuKernelExecutor::new(&config);
        assert_eq!(executor.name(), "QemuKernelExecutor");
        assert_eq!(executor.executions(), 0);
    }

    #[test]
    fn test_qemu_executor_check_requirements() {
        let config = ExecutorConfig {
            qemu_path: PathBuf::from("/nonexistent/qemu"),
            ..Default::default()
        };
        let executor = QemuKernelExecutor::new(&config);
        assert!(executor.check_requirements().is_err());
    }
}
