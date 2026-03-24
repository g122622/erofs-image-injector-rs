//! Command Line Interface
//!
//! Argument parsing for the EROFS fuzzer.

use std::path::PathBuf;

use clap::Parser;

/// EROFS Image Fuzzer - A structure-aware fuzzer for EROFS filesystem images
#[derive(Parser, Debug, Clone)]
#[command(name = "erofs-fuzzer")]
#[command(author = "GSoC 2026 EROFS Fuzzing Project")]
#[command(version = "0.1.0")]
#[command(about = "LibAFL-based fuzzer for EROFS filesystem images")]
pub struct CliArgs {
    /// Directory containing seed EROFS images
    #[arg(short, long, value_name = "DIR")]
    pub seeds: PathBuf,

    /// Directory to store crash outputs
    #[arg(short, long, value_name = "DIR", default_value = "./crashes")]
    pub output: PathBuf,

    /// Timeout per execution in seconds
    #[arg(short, long, default_value = "60")]
    pub timeout: u64,

    /// Maximum number of iterations (0 = unlimited)
    #[arg(short, long, default_value = "0")]
    pub iterations: u64,

    /// Path to erofsfuse binary
    #[arg(long, value_name = "PATH", default_value = "erofsfuse")]
    pub erofsfuse_path: PathBuf,

    /// Number of parallel workers
    #[arg(short, long, default_value = "1")]
    pub workers: usize,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    pub log_level: String,

    /// Corpus directory for interesting inputs
    #[arg(long, value_name = "DIR", default_value = "./corpus")]
    pub corpus: PathBuf,

    /// Enable AddressSanitizer detection
    #[arg(long, default_value = "false")]
    pub asan: bool,

    /// Mount point base (will create temp dirs under this)
    #[arg(long, value_name = "DIR", default_value = "/tmp/erofs-fuzz")]
    pub mount_base: PathBuf,

    /// Maximum image size in bytes
    #[arg(long, default_value = "16777216")]
    pub max_size: usize,

    /// Minimum image size in bytes
    #[arg(long, default_value = "4096")]
    pub min_size: usize,

    /// Number of mutations per input
    #[arg(long, default_value = "4")]
    pub mutations_per_input: usize,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

/// Configuration derived from CLI arguments
#[derive(Debug, Clone)]
pub struct FuzzerConfig {
    /// Seeds directory
    pub seeds_dir: PathBuf,
    /// Output directory for crashes
    pub output_dir: PathBuf,
    /// Corpus directory
    pub corpus_dir: PathBuf,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum iterations (0 = unlimited)
    pub max_iterations: u64,
    /// Path to erofsfuse
    pub erofsfuse_path: PathBuf,
    /// Number of workers
    pub num_workers: usize,
    /// Mount base directory
    pub mount_base: PathBuf,
    /// Maximum image size
    pub max_image_size: usize,
    /// Minimum image size
    pub min_image_size: usize,
    /// Mutations per input
    pub mutations_per_input: usize,
    /// ASan enabled
    pub asan_enabled: bool,
}

impl From<CliArgs> for FuzzerConfig {
    fn from(args: CliArgs) -> Self {
        Self {
            seeds_dir: args.seeds,
            output_dir: args.output,
            corpus_dir: args.corpus,
            timeout_ms: args.timeout * 1000,
            max_iterations: args.iterations,
            erofsfuse_path: args.erofsfuse_path,
            num_workers: args.workers,
            mount_base: args.mount_base,
            max_image_size: args.max_size,
            min_image_size: args.min_size,
            mutations_per_input: args.mutations_per_input,
            asan_enabled: args.asan,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_args_defaults() {
        let args = CliArgs::parse_from(["test", "--seeds", "./seeds"]);
        assert_eq!(args.output, PathBuf::from("./crashes"));
        assert_eq!(args.timeout, 60);
        assert_eq!(args.workers, 1);
    }
}
