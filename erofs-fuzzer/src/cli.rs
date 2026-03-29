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
    /// Directory containing seed EROFS images (not required for --web mode)
    #[arg(short, long, value_name = "DIR", default_value = "./seeds")]
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

    // ===== Targeted Mutation Arguments =====

    /// Target field for precise mutation (format: struct.field, e.g., superblock.checksum)
    #[arg(long, value_name = "FIELD")]
    pub target: Option<String>,

    /// Absolute byte range to mutate (format: start:length, e.g., 1024:8)
    #[arg(long, value_name = "RANGE")]
    pub range: Option<String>,

    /// Bytes before the target field to include in mutation
    #[arg(long, default_value = "0")]
    pub before: usize,

    /// Bytes after the target field to include in mutation
    #[arg(long, default_value = "0")]
    pub after: usize,

    /// Mutation strategy: bitflip, arithmetic, interesting, boundary, random, zero, max
    #[arg(long, default_value = "bitflip")]
    pub strategy: String,

    /// Number of mutations to apply in targeted mode
    #[arg(long, default_value = "1")]
    pub count: usize,

    /// Enable targeted-only mode (skip random mutations, only use targeted mutations)
    #[arg(long)]
    pub targeted: bool,

    // ===== QEMU Kernel Testing Arguments =====

    /// Executor type: erofsfuse (default) or qemu
    #[arg(long, value_enum, default_value = "erofsfuse")]
    pub executor: ExecutorTypeArg,

    /// Path to kernel bzImage (for QEMU executor)
    #[arg(long, value_name = "PATH", default_value = "./kernel_build/bzImage")]
    pub kernel: PathBuf,

    /// Path to initramfs (for QEMU executor)
    #[arg(long, value_name = "PATH", default_value = "./kernel_build/rootfs.cpio.gz")]
    pub initramfs: PathBuf,

    /// Path to QEMU binary (for QEMU executor)
    #[arg(long, value_name = "PATH", default_value = "qemu-system-x86_64")]
    pub qemu_path: PathBuf,

    /// Memory for QEMU in MB (for QEMU executor)
    #[arg(long, default_value = "512")]
    pub qemu_memory: usize,

    // ===== Web Console Arguments =====

    /// Start web console instead of running fuzzer directly
    #[arg(long)]
    pub web: bool,

    /// Port for web console (only used with --web)
    #[arg(long, default_value = "8080")]
    pub web_port: u16,

    /// Database path for web console (default: in-memory)
    #[arg(long, value_name = "PATH")]
    pub web_db: Option<PathBuf>,

    /// Maximum concurrent tasks in web console
    #[arg(long, default_value = "4")]
    pub web_max_tasks: usize,
}

/// Executor type argument
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ExecutorTypeArg {
    /// User-space erofsfuse testing
    Erofsfuse,
    /// QEMU kernel testing
    Qemu,
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
    /// Targeted mutation configuration
    pub targeted_config: Option<TargetedConfig>,
    /// Targeted-only mode (skip random mutations)
    pub targeted_only: bool,
    /// Executor type
    pub executor_type: ExecutorType,
    /// Path to kernel bzImage (for QEMU)
    pub kernel_path: PathBuf,
    /// Path to initramfs (for QEMU)
    pub initramfs_path: PathBuf,
    /// Path to QEMU binary
    pub qemu_path: PathBuf,
    /// Memory for QEMU in MB
    pub qemu_memory: usize,
}

/// Configuration for targeted mutation
#[derive(Debug, Clone)]
pub struct TargetedConfig {
    /// Target field specification (if using field targeting)
    pub target: Option<String>,
    /// Absolute range (if using range targeting)
    pub range: Option<String>,
    /// Bytes before target field
    pub before: usize,
    /// Bytes after target field
    pub after: usize,
    /// Mutation strategy
    pub strategy: String,
    /// Number of mutations
    pub count: usize,
}

/// Executor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorType {
    /// User-space erofsfuse testing
    Erofsfuse,
    /// QEMU kernel testing
    QemuKernel,
}

impl From<ExecutorTypeArg> for ExecutorType {
    fn from(arg: ExecutorTypeArg) -> Self {
        match arg {
            ExecutorTypeArg::Erofsfuse => ExecutorType::Erofsfuse,
            ExecutorTypeArg::Qemu => ExecutorType::QemuKernel,
        }
    }
}

impl From<CliArgs> for FuzzerConfig {
    fn from(args: CliArgs) -> Self {
        let targeted_config = if args.target.is_some() || args.range.is_some() {
            Some(TargetedConfig {
                target: args.target,
                range: args.range,
                before: args.before,
                after: args.after,
                strategy: args.strategy,
                count: args.count,
            })
        } else {
            None
        };

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
            targeted_config,
            targeted_only: args.targeted,
            executor_type: args.executor.into(),
            kernel_path: args.kernel,
            initramfs_path: args.initramfs,
            qemu_path: args.qemu_path,
            qemu_memory: args.qemu_memory,
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

    #[test]
    fn test_targeted_args() {
        let args = CliArgs::parse_from([
            "test", "--seeds", "./seeds",
            "--target", "superblock.checksum",
            "--after", "4",
            "--strategy", "bitflip",
            "--count", "3",
        ]);
        assert_eq!(args.target, Some("superblock.checksum".to_string()));
        assert_eq!(args.after, 4);
        assert_eq!(args.strategy, "bitflip");
        assert_eq!(args.count, 3);
    }

    #[test]
    fn test_range_targeting() {
        let args = CliArgs::parse_from([
            "test", "--seeds", "./seeds",
            "--range", "1024:8",
            "--strategy", "zero",
        ]);
        assert_eq!(args.range, Some("1024:8".to_string()));
        assert_eq!(args.strategy, "zero");
    }
}
