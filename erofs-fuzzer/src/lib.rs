//! EROFS Image Fuzzer
//!
//! Main entry point for the EROFS image injection fuzzer.

#![deny(missing_docs)]

mod cli;
mod executor;
mod fuzzer;
mod harness;

pub use cli::*;
pub use executor::*;
pub use fuzzer::*;
pub use harness::*;

use std::path::PathBuf;

/// Re-exports for convenience
pub mod prelude {
    pub use crate::{CliArgs, ErofsfuseExecutor, FuzzerConfig, run_fuzzer};
}
