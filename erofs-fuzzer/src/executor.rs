//! Erofsfuse Executor
//!
//! Executor that runs erofsfuse to mount and test EROFS images.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tracing::{debug, error, info, trace, warn};

use libafl::executors::ExitKind;
use libafl::inputs::Input;
use libafl::state::HasMetadata;
use libafl_bolts::Error;
use libafl_bolts::rands::Rand;

use erofs_input::ErofsImageInput;

use crate::FuzzerConfig;

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
}

/// Executor for erofsfuse
///
/// This executor writes the image to a temp file, mounts it using erofsfuse,
/// performs file system operations, and detects crashes.
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

    /// Execute the target with the given input
    pub fn execute(&mut self, input: &ErofsImageInput) -> Result<ErofsfuseExit, Error> {
        self.executions += 1;

        let data = input.data();

        // Check size constraints
        if data.len() < self.min_size {
            trace!("Input too small: {} < {}", data.len(), self.min_size);
            return Ok(ErofsfuseExit::Error(0));
        }
        if data.len() > self.max_size {
            trace!("Input too large: {} > {}", data.len(), self.max_size);
            return Ok(ErofsfuseExit::Error(0));
        }

        // Create temp directory for this execution
        let temp_dir = tempfile::tempdir_in(&self.mount_base)
            .map_err(|e| Error::illegal_state(format!("Failed to create temp dir: {}", e)))?;

        // Write image to temp file
        let image_path = temp_dir.path().join("image.erofs");
        std::fs::write(&image_path, data)
            .map_err(|e| Error::illegal_state(format!("Failed to write image: {}", e)))?;

        // Create mount point
        let mount_point = temp_dir.path().join("mnt");
        std::fs::create_dir_all(&mount_point)
            .map_err(|e| Error::illegal_state(format!("Failed to create mount point: {}", e)))?;

        // Run erofsfuse
        let result = self.run_erofsfuse(&image_path, &mount_point);

        // Cleanup (unless keep_temp)
        if !self.keep_temp {
            // Unmount if still mounted
            let _ = self.unmount(&mount_point);
        }

        result
    }

    /// Run erofsfuse and perform file system operations
    fn run_erofsfuse(
        &self,
        image_path: &Path,
        mount_point: &Path,
    ) -> Result<ErofsfuseExit, Error> {
        let start = Instant::now();

        // Start erofsfuse process
        let mut child = match Command::new(&self.erofsfuse_path)
            .arg(image_path)
            .arg(mount_point)
            .arg("-o")
            .arg("ro")
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

        // Wait for mount to complete (or timeout)
        let mount_timeout = Duration::from_secs(5);
        let mut mounted = false;

        while start.elapsed() < mount_timeout {
            // Check if mount point has content
            if mount_point.exists() && mount_point.read_dir().map_or(false, |mut d| d.next().is_some()) {
                mounted = true;
                break;
            }

            // Check if process has exited
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process exited during mount
                    let exit_code = status.code().unwrap_or(-1);
                    debug!("erofsfuse exited during mount with status: {}", exit_code);

                    // Check for crash signals
                    #[cfg(unix)]
                    {
                        use std::os::unix::process::ExitStatusExt;
                        if let Some(signal) = status.signal() {
                            return Ok(ErofsfuseExit::Crashed(signal));
                        }
                    }

                    return Ok(ErofsfuseExit::Error(exit_code));
                }
                Ok(None) => {
                    // Still running
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    error!("Failed to check process status: {}", e);
                    break;
                }
            }
        }

        if !mounted {
            warn!("Mount timeout, killing process");
            let _ = child.kill();
            let _ = child.wait();
            return Ok(ErofsfuseExit::Timeout);
        }

        // Perform file system operations
        let ops_result = self.perform_filesystem_ops(mount_point);

        // Unmount
        let unmount_result = self.unmount(mount_point);

        // Wait for process to exit
        match child.wait_timeout(self.timeout.saturating_sub(start.elapsed())) {
            Ok(Some(status)) => {
                let exit_code = status.code().unwrap_or(-1);

                // Check for crash signals
                #[cfg(unix)]
                {
                    use std::os::unix::process::ExitStatusExt;
                    if let Some(signal) = status.signal() {
                        return Ok(ErofsfuseExit::Crashed(signal));
                    }
                }

                if exit_code != 0 {
                    Ok(ErofsfuseExit::Error(exit_code))
                } else {
                    Ok(ops_result.unwrap_or(ErofsfuseExit::Success))
                }
            }
            Ok(None) => {
                // Timeout
                warn!("Process timeout, killing");
                let _ = child.kill();
                let _ = child.wait();
                Ok(ErofsfuseExit::Timeout)
            }
            Err(e) => {
                error!("Failed to wait for process: {}", e);
                let _ = child.kill();
                let _ = child.wait();
                Ok(ErofsfuseExit::Error(-1))
            }
        }
    }

    /// Perform file system operations on the mount point
    fn perform_filesystem_ops(&self, mount_point: &Path) -> Result<ErofsfuseExit, Error> {
        use std::fs;

        // List root directory
        match fs::read_dir(mount_point) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();

                    // Try to read file metadata
                    let _ = fs::metadata(&path);

                    // If it's a directory, traverse
                    if path.is_dir() {
                        let _ = self.traverse_directory(&path);
                    } else {
                        // Try to read file contents
                        let _ = self.read_file(&path);
                    }

                    // Try to read extended attributes
                    let _ = self.read_xattrs(&path);
                }
            }
            Err(e) => {
                debug!("Failed to read mount point: {}", e);
            }
        }

        Ok(ErofsfuseExit::Success)
    }

    /// Traverse a directory recursively
    fn traverse_directory(&self, dir: &Path) -> Result<(), Error> {
        use std::fs;

        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();

                    // Read metadata
                    let _ = fs::metadata(&path);

                    if path.is_dir() {
                        // Recurse (limit depth)
                        if path.components().count() < 20 {
                            let _ = self.traverse_directory(&path);
                        }
                    } else {
                        let _ = self.read_file(&path);
                    }

                    let _ = self.read_xattrs(&path);
                }
            }
            Err(e) => {
                debug!("Failed to traverse directory: {}", e);
            }
        }

        Ok(())
    }

    /// Read a file's contents
    fn read_file(&self, path: &Path) -> Result<(), Error> {
        use std::fs;

        // Read with size limit
        match fs::File::open(path) {
            Ok(mut file) => {
                let mut buf = vec![0u8; 4096];
                let _ = std::io::Read::read(&mut file, &mut buf);
            }
            Err(e) => {
                debug!("Failed to read file {:?}: {}", path, e);
            }
        }

        Ok(())
    }

    /// Read extended attributes
    fn read_xattrs(&self, path: &Path) -> Result<(), Error> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            // Try to list xattrs
            // Note: This requires xattr support in Rust or external crate
            // For now, just try to get basic metadata
            let _ = std::fs::metadata(path);
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
            // Windows doesn't have FUSE in the same way
            warn!("Unmount not implemented for Windows");
        }

        Ok(())
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

    #[test]
    fn test_executor_creation() {
        let config = FuzzerConfig::from(CliArgs::parse_from(["test", "--seeds", "./seeds"]));
        let executor = ErofsfuseExecutor::new(&config);
        assert_eq!(executor.executions(), 0);
    }
}
