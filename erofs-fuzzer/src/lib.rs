//! EROFS Image Fuzzer
//!
//! Main entry point for the EROFS image injection fuzzer.

#![deny(missing_docs)]

mod cli;
mod executor;
mod executor_trait;
mod fuzzer;
mod harness;
mod kernel_monitor;
mod qemu_executor;

pub use cli::*;
pub use executor::*;
pub use executor_trait::*;
pub use fuzzer::*;
pub use harness::*;
pub use kernel_monitor::*;
pub use qemu_executor::*;

/// Re-exports for convenience
pub mod prelude {
    pub use crate::{CliArgs, ErofsfuseExecutor, FuzzerConfig, run_fuzzer};
    pub use crate::{Executor, ExecutorConfig, ExecutorType, ExecutionResult};
    pub use crate::{QemuKernelExecutor, KernelMonitor, KernelCrashInfo, KernelIssue};
}
