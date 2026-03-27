//! Executor Trait and Types
//!
//! This module defines the abstract executor interface for running EROFS tests.
//! It supports both user-space (erofsfuse) and kernel-space (QEMU) testing.

use std::path::PathBuf;
use std::time::Duration;

use libafl_bolts::Error;

use erofs_input::ErofsImageInput;

/// Exit kind for test execution
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionResult {
    /// Execution completed successfully
    Success,
    /// Process timed out
    Timeout,
    /// Process crashed with signal
    Crashed(i32),
    /// Process returned non-zero exit code
    Error(i32),
    /// ASan detected memory error
    AsanError,
    /// Kernel panic detected
    KernelPanic,
    /// Kernel oops detected
    KernelOops,
    /// Failed to start process
    FailedToStart,
}

impl ExecutionResult {
    /// Returns true if this result indicates a crash or error
    pub fn is_crash(&self) -> bool {
        matches!(
            self,
            ExecutionResult::Crashed(_)
                | ExecutionResult::AsanError
                | ExecutionResult::KernelPanic
                | ExecutionResult::KernelOops
        )
    }

    /// Returns true if this result indicates a kernel-space issue
    pub fn is_kernel_issue(&self) -> bool {
        matches!(
            self,
            ExecutionResult::KernelPanic | ExecutionResult::KernelOops
        )
    }

    /// Get a description of the result
    pub fn description(&self) -> &'static str {
        match self {
            ExecutionResult::Success => "Success",
            ExecutionResult::Timeout => "Timeout",
            ExecutionResult::Crashed(_) => "Crashed",
            ExecutionResult::Error(_) => "Error",
            ExecutionResult::AsanError => "ASan Error",
            ExecutionResult::KernelPanic => "Kernel Panic",
            ExecutionResult::KernelOops => "Kernel Oops",
            ExecutionResult::FailedToStart => "Failed to Start",
        }
    }
}

/// Executor type selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorType {
    /// User-space erofsfuse testing
    Erofsfuse,
    /// QEMU kernel testing
    QemuKernel,
}

/// Configuration for executor creation
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Executor type to use
    pub executor_type: ExecutorType,
    /// Timeout for each execution
    pub timeout: Duration,
    /// Maximum image size
    pub max_size: usize,
    /// Minimum image size
    pub min_size: usize,
    /// Path to erofsfuse binary (for Erofsfuse executor)
    pub erofsfuse_path: PathBuf,
    /// Mount base directory (for Erofsfuse executor)
    pub mount_base: PathBuf,
    /// Path to QEMU binary (for QEMU executor)
    pub qemu_path: PathBuf,
    /// Path to kernel image (for QEMU executor)
    pub kernel_path: PathBuf,
    /// Path to initramfs (for QEMU executor)
    pub initramfs_path: PathBuf,
    /// Extra QEMU arguments
    pub qemu_args: Vec<String>,
    /// Memory for QEMU (in MB)
    pub qemu_memory: usize,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            executor_type: ExecutorType::Erofsfuse,
            timeout: Duration::from_secs(60),
            max_size: 16 * 1024 * 1024,
            min_size: 4096,
            erofsfuse_path: PathBuf::from("erofsfuse"),
            mount_base: PathBuf::from("/tmp/erofs-fuzz"),
            qemu_path: PathBuf::from("qemu-system-x86_64"),
            kernel_path: PathBuf::from("./kernel_build/bzImage"),
            initramfs_path: PathBuf::from("./kernel_build/rootfs.cpio.gz"),
            qemu_args: Vec::new(),
            qemu_memory: 512,
        }
    }
}

impl ExecutorConfig {
    /// Create a new executor config for erofsfuse
    pub fn erofsfuse(erofsfuse_path: PathBuf, mount_base: PathBuf, timeout_ms: u64) -> Self {
        Self {
            executor_type: ExecutorType::Erofsfuse,
            timeout: Duration::from_millis(timeout_ms),
            erofsfuse_path,
            mount_base,
            ..Default::default()
        }
    }

    /// Create a new executor config for QEMU kernel testing
    pub fn qemu(
        kernel_path: PathBuf,
        initramfs_path: PathBuf,
        timeout_ms: u64,
    ) -> Self {
        Self {
            executor_type: ExecutorType::QemuKernel,
            timeout: Duration::from_millis(timeout_ms),
            kernel_path,
            initramfs_path,
            ..Default::default()
        }
    }

    /// Set QEMU path
    pub fn with_qemu_path(mut self, path: PathBuf) -> Self {
        self.qemu_path = path;
        self
    }

    /// Set QEMU memory
    pub fn with_qemu_memory(mut self, memory_mb: usize) -> Self {
        self.qemu_memory = memory_mb;
        self
    }

    /// Set max image size
    pub fn with_max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    /// Set min image size
    pub fn with_min_size(mut self, size: usize) -> Self {
        self.min_size = size;
        self
    }
}

/// Trait for EROFS test executors
pub trait Executor {
    /// Execute a test with the given EROFS image input
    fn execute(&mut self, input: &ErofsImageInput) -> Result<ExecutionResult, Error>;

    /// Get the number of executions performed
    fn executions(&self) -> u64;

    /// Check if a result indicates a crash
    fn is_crash(&self, result: &ExecutionResult) -> bool {
        result.is_crash()
    }

    /// Get executor name
    fn name(&self) -> &'static str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_result_is_crash() {
        assert!(ExecutionResult::Crashed(11).is_crash());
        assert!(ExecutionResult::AsanError.is_crash());
        assert!(ExecutionResult::KernelPanic.is_crash());
        assert!(ExecutionResult::KernelOops.is_crash());
        assert!(!ExecutionResult::Success.is_crash());
        assert!(!ExecutionResult::Timeout.is_crash());
    }

    #[test]
    fn test_execution_result_is_kernel_issue() {
        assert!(ExecutionResult::KernelPanic.is_kernel_issue());
        assert!(ExecutionResult::KernelOops.is_kernel_issue());
        assert!(!ExecutionResult::Crashed(11).is_kernel_issue());
        assert!(!ExecutionResult::AsanError.is_kernel_issue());
    }

    #[test]
    fn test_executor_config_default() {
        let config = ExecutorConfig::default();
        assert_eq!(config.executor_type, ExecutorType::Erofsfuse);
        assert_eq!(config.qemu_memory, 512);
    }

    #[test]
    fn test_executor_config_builder() {
        let config = ExecutorConfig::erofsfuse(
            PathBuf::from("/usr/bin/erofsfuse"),
            PathBuf::from("/tmp/test"),
            30000,
        );
        assert_eq!(config.executor_type, ExecutorType::Erofsfuse);
        assert_eq!(config.timeout, Duration::from_millis(30000));
    }
}
