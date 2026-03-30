//! Main Fuzzer Logic
//!
//! Core fuzzing loop and orchestration.

use std::io::Write;
use std::num::NonZeroUsize;

use libafl::mutators::Mutator;
use libafl::state::HasRand;
use libafl_bolts::current_nanos;
use libafl_bolts::rands::{Rand, StdRand};
use tracing::{debug, info, warn};

use erofs_input::{
    ErofsImageInput, MutationStrategy, MutationTarget,
    parse_target,
};
use erofs_mutator::{
    ErofsBitflipMutator, ErofsDirectoryMutator, ErofsInodeMutator, ErofsSuperblockMutator,
    ErofsXattrMutator, TargetedMutator, rand_below,
};

use crate::cli::{CliArgs, FuzzerConfig, ExecutorType};
use crate::executor::ErofsfuseExecutor;
use crate::qemu_executor::QemuKernelExecutor;
use crate::executor_trait::{Executor, ExecutionResult, ExecutionOutput, ExecutorConfig};

/// Fuzzer error types
#[derive(Debug, thiserror::Error)]
pub enum FuzzerError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// No seeds found
    #[error("No seed images found in {0}")]
    NoSeeds(String),

    /// Executor error
    #[error("Executor error: {0}")]
    Executor(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    Config(String),
}

/// Result type for fuzzer operations
pub type FuzzerResult<T> = Result<T, FuzzerError>;

/// Unified executor wrapper
enum FuzzerExecutor {
    /// User-space erofsfuse executor
    Erofsfuse(ErofsfuseExecutor),
    /// QEMU kernel executor
    QemuKernel(QemuKernelExecutor),
}

impl FuzzerExecutor {
    fn execute(&mut self, input: &ErofsImageInput) -> FuzzerResult<ExecutionResult> {
        match self {
            FuzzerExecutor::Erofsfuse(e) => {
                e.execute(input).map_err(|e| FuzzerError::Executor(e.to_string()))
            }
            FuzzerExecutor::QemuKernel(e) => {
                e.execute(input).map_err(|e| FuzzerError::Executor(e.to_string()))
            }
        }
    }

    fn execute_with_output(&mut self, input: &ErofsImageInput) -> FuzzerResult<ExecutionOutput> {
        match self {
            FuzzerExecutor::Erofsfuse(e) => {
                e.execute_with_output(input).map_err(|e| FuzzerError::Executor(e.to_string()))
            }
            FuzzerExecutor::QemuKernel(e) => {
                e.execute_with_output(input).map_err(|e| FuzzerError::Executor(e.to_string()))
            }
        }
    }

    fn name(&self) -> &'static str {
        match self {
            FuzzerExecutor::Erofsfuse(_) => "ErofsfuseExecutor",
            FuzzerExecutor::QemuKernel(_) => "QemuKernelExecutor",
        }
    }
}

/// Run the fuzzer with the given arguments
pub fn run_fuzzer(args: CliArgs) -> FuzzerResult<()> {
    let config: FuzzerConfig = args.into();

    info!("Initializing EROFS fuzzer...");
    info!("Configuration: {:?}", config);

    // Validate configuration
    validate_config(&config)?;

    // Create output directories
    std::fs::create_dir_all(&config.output_dir)?;
    std::fs::create_dir_all(&config.corpus_dir)?;
    std::fs::create_dir_all(&config.mount_base)?;

    // Create state with randomness
    let mut state = SimpleFuzzerState::new(current_nanos());

    // Load seeds
    load_seeds(&mut state, &config)?;

    // Check if we have any seeds
    if state.corpus_count == 0 {
        return Err(FuzzerError::NoSeeds(config.seeds_dir.display().to_string()));
    }

    info!("Loaded {} seeds", state.corpus_count);

    // Run the simple fuzzer loop (without coverage for now)
    info!("Starting fuzzing loop...");

    run_simple_fuzzer(&config, &mut state)?;

    info!("Fuzzing completed");
    Ok(())
}

/// Validate the fuzzer configuration
fn validate_config(config: &FuzzerConfig) -> FuzzerResult<()> {
    // Check seeds directory exists
    if !config.seeds_dir.exists() {
        return Err(FuzzerError::Config(format!(
            "Seeds directory not found: {:?}",
            config.seeds_dir
        )));
    }

    // Validate size constraints
    if config.min_image_size > config.max_image_size {
        return Err(FuzzerError::Config(
            "min_image_size cannot be greater than max_image_size".to_string(),
        ));
    }

    // Validate workers
    if config.num_workers < 1 {
        return Err(FuzzerError::Config(
            "num_workers must be at least 1".to_string(),
        ));
    }

    // Check executor-specific requirements
    match config.executor_type {
        ExecutorType::Erofsfuse => {
            if !is_executable_available(&config.erofsfuse_path) {
                return Err(FuzzerError::Config(format!(
                    "erofsfuse not found at {:?}",
                    config.erofsfuse_path
                )));
            }
        }
        ExecutorType::QemuKernel => {
            if !config.kernel_path.exists() {
                return Err(FuzzerError::Config(format!(
                    "Kernel not found at {:?}. Run scripts/build_kernel.sh first.",
                    config.kernel_path
                )));
            }
            if !config.initramfs_path.exists() {
                return Err(FuzzerError::Config(format!(
                    "Initramfs not found at {:?}. Run scripts/build_kernel.sh first.",
                    config.initramfs_path
                )));
            }
            if !is_executable_available(&config.qemu_path) {
                return Err(FuzzerError::Config(format!(
                    "QEMU not found at {:?}",
                    config.qemu_path
                )));
            }
        }
    }

    Ok(())
}

fn is_executable_available(path: &std::path::Path) -> bool {
    if path.is_absolute() || path.to_string_lossy().contains('/') {
        return path.exists();
    }

    if path.exists() {
        return true;
    }

    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths)
                .any(|dir| dir.join(path).exists())
        })
        .unwrap_or(false)
}

/// Simple fuzzer state that holds seeds and random generator
struct SimpleFuzzerState {
    rand: StdRand,
    seeds: Vec<ErofsImageInput>,
    corpus_count: usize,
}

impl SimpleFuzzerState {
    fn new(seed: u64) -> Self {
        Self {
            rand: StdRand::with_seed(seed),
            seeds: Vec::new(),
            corpus_count: 0,
        }
    }
}

impl HasRand for SimpleFuzzerState {
    type Rand = StdRand;

    fn rand(&self) -> &Self::Rand {
        &self.rand
    }

    fn rand_mut(&mut self) -> &mut Self::Rand {
        &mut self.rand
    }
}

/// Load seed images from the seeds directory
fn load_seeds(state: &mut SimpleFuzzerState, config: &FuzzerConfig) -> FuzzerResult<()> {
    info!("Loading seeds from {:?}", config.seeds_dir);

    let mut count = 0;
    let entries = std::fs::read_dir(&config.seeds_dir)?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_file() {
            // Check extension
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext == "erofs" || ext == "img" || ext == "" {
                // Load the file
                match std::fs::read(&path) {
                    Ok(data) => {
                        // Get file name for tracking
                        let name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let input = ErofsImageInput::with_name(data, &name);
                        state.seeds.push(input);
                        count += 1;
                        debug!("Loaded seed: {:?}", path);
                    }
                    Err(e) => {
                        warn!("Failed to load seed {:?}: {}", path, e);
                    }
                }
            }
        }
    }

    state.corpus_count = count;
    info!("Loaded {} seed images", count);
    Ok(())
}

/// Create a targeted mutator from configuration
fn create_targeted_mutator(config: &FuzzerConfig) -> Option<TargetedMutator> {
    let tc = config.targeted_config.as_ref()?;

    // Parse strategy
    let strategy = MutationStrategy::from_str(&tc.strategy)
        .unwrap_or_else(|_| MutationStrategy::BitFlip { count: 1 });

    // Parse target
    let target = if let Some(target_str) = &tc.target {
        // Field targeting
        parse_target(target_str).ok()?
    } else if let Some(range_str) = &tc.range {
        // Range targeting
        let parts: Vec<&str> = range_str.split(':').collect();
        if parts.len() != 2 {
            warn!("Invalid range format: {}. Expected start:length", range_str);
            return None;
        }
        let start = parts[0].parse::<usize>().ok()?;
        let length = parts[1].parse::<usize>().ok()?;
        MutationTarget::AbsoluteRange { start, length }
    } else {
        warn!("No target or range specified for targeted mutation");
        return None;
    };

    // Apply before/after offsets if targeting a field
    let target = match target {
        MutationTarget::FieldRange { field, offset_before: _, offset_after: _ } => {
            MutationTarget::FieldRange {
                field,
                offset_before: tc.before,
                offset_after: tc.after,
            }
        }
        other => other,
    };

    let mut mutator = TargetedMutator::new(target, strategy);
    mutator = mutator.with_max_mutations(tc.count);

    Some(mutator)
}

/// Create executor based on configuration
fn create_executor(config: &FuzzerConfig) -> FuzzerResult<FuzzerExecutor> {
    match config.executor_type {
        ExecutorType::Erofsfuse => {
            info!("Using ErofsfuseExecutor");
            Ok(FuzzerExecutor::Erofsfuse(ErofsfuseExecutor::new(config)))
        }
        ExecutorType::QemuKernel => {
            info!("Using QemuKernelExecutor");
            info!("Kernel: {:?}", config.kernel_path);
            info!("Initramfs: {:?}", config.initramfs_path);
            let executor_config = crate::executor_trait::ExecutorConfig::qemu(
                config.kernel_path.clone(),
                config.initramfs_path.clone(),
                config.timeout_ms,
            )
            .with_qemu_path(config.qemu_path.clone())
            .with_qemu_memory(config.qemu_memory)
            .with_max_size(config.max_image_size)
            .with_min_size(config.min_image_size);

            let mut executor = QemuKernelExecutor::new(&executor_config);
            // Check requirements early
            executor.check_requirements()
                .map_err(|e| FuzzerError::Config(e.to_string()))?;
            Ok(FuzzerExecutor::QemuKernel(executor))
        }
    }
}

/// Handle crash result and save to disk
fn handle_crash(config: &FuzzerConfig, input: &ErofsImageInput, result: ExecutionResult, iteration: u64, seed_idx: Option<usize>, kernel_log: Option<&str>) -> FuzzerResult<()> {
    let (crash_type, suffix) = match result {
        ExecutionResult::Crashed(signal) => ("Signal", format!("signal-{}", signal)),
        ExecutionResult::AsanError => ("ASan", "asan".to_string()),
        ExecutionResult::KernelPanic => ("KernelPanic", "kernel-panic".to_string()),
        ExecutionResult::KernelOops => ("KernelOops", "kernel-oops".to_string()),
        _ => return Ok(()),
    };

    let prefix = seed_idx.map(|_| "seed-crash").unwrap_or("crash");
    let timestamp = current_nanos();

    let crash_path = config.output_dir.join(format!(
        "{}-{:016x}-{}.erofs",
        prefix, timestamp, suffix
    ));
    std::fs::write(&crash_path, input.data())?;
    info!("Crash ({}) saved to {:?}", crash_type, crash_path);

    let info_path = config.output_dir.join(format!(
        "{}-{:016x}-{}.log",
        prefix, timestamp, suffix
    ));

    // Build log content with crash info and kernel log
    let mut info = match seed_idx {
        Some(idx) => format!(
            "Type: {}\nIteration: 0 (seed)\nSize: {} bytes\nSeed index: {}\n",
            crash_type, input.len(), idx
        ),
        None => format!(
            "Type: {}\nIteration: {}\nSize: {} bytes\n",
            crash_type, iteration, input.len()
        ),
    };

    // Add kernel log if available
    if let Some(log) = kernel_log {
        info.push_str("\n========================================\n");
        info.push_str("Kernel Log:\n");
        info.push_str("========================================\n");
        info.push_str(log);
    }

    let _ = std::fs::write(&info_path, info);

    Ok(())
}

/// Simple fuzzer implementation without coverage-guided feedback
fn run_simple_fuzzer(config: &FuzzerConfig, state: &mut SimpleFuzzerState) -> FuzzerResult<()> {
    let mut executor = create_executor(config)?;
    info!("Created executor: {}", executor.name());

    let mut iterations = 0u64;
    let mut crashes = 0u64;

    let mut bitflip_mutator = ErofsBitflipMutator::new();
    let mut sb_mutator = ErofsSuperblockMutator::new();
    let mut inode_mutator = ErofsInodeMutator::new();
    let mut dir_mutator = ErofsDirectoryMutator::new();
    let mut xattr_mutator = ErofsXattrMutator::new();

    // Create targeted mutator if configured
    let mut targeted_mutator = create_targeted_mutator(config);

    if targeted_mutator.is_some() {
        info!("Targeted mutation mode enabled");
        if let Some(ref tc) = config.targeted_config {
            info!("Target config: {:?}", tc);
        }
    }

    // Build weighted mutator list from strategy config
    let weighted_mutators = build_weighted_mutators(config);
    let total_weight: u32 = weighted_mutators.iter().map(|(_, w)| *w).sum();
    info!("Loaded {} mutators with total weight {}", weighted_mutators.len(), total_weight);
    for (name, weight) in &weighted_mutators {
        info!("  Mutator '{}': weight {} ({:.1}%)", name, weight, (*weight as f64 / total_weight as f64) * 100.0);
    }

    // Detect and output version information
    let (kernel_version, erofs_version) = detect_versions(config);
    println!("[FUZZER] start seeds={} max_iterations={} kernel_version={} erofs_version={}",
             state.seeds.len(), config.max_iterations,
             kernel_version.as_deref().unwrap_or("unknown"),
             erofs_version.as_deref().unwrap_or("unknown"));
    let _ = std::io::stdout().flush();

    // Track timing for speed calculation
    let mut last_progress_time = std::time::Instant::now();
    let mut last_progress_iterations = 0u64;

    // First, test all seeds without mutation to find existing crashes
    info!("Testing {} seed images without mutation...", state.seeds.len());
    for (idx, seed) in state.seeds.iter().enumerate() {
        info!("Testing seed {} of {}", idx + 1, state.seeds.len());
        println!("[MUTATOR] seed-test-{}", idx + 1);
        let _ = std::io::stdout().flush();
        match executor.execute_with_output(seed) {
            Ok(output) if output.result.is_crash() => {
                info!("Found crash in seed: {:?}", output.result);
                crashes += 1;
                handle_crash(config, seed, output.result, 0, Some(idx), output.kernel_log.as_deref())?;
            }
            Ok(output) => {
                debug!("Seed result: {:?}", output.result);
            }
            Err(e) => {
                debug!("Seed execution error: {}", e);
            }
        }
    }

    // Output progress after seed testing
    println!("[PROGRESS] iteration=0 crashes={} speed=0.0", crashes);
    let _ = std::io::stdout().flush();

    info!("Starting mutation-based fuzzing...");

    loop {
        // Check iteration limit
        if config.max_iterations > 0 && iterations >= config.max_iterations {
            info!("Reached max iterations: {}", config.max_iterations);
            break;
        }

        // Get a random seed from corpus
        if state.seeds.is_empty() {
            warn!("Corpus is empty");
            break;
        }

        let seed_idx = state.rand.below(NonZeroUsize::new(state.seeds.len()).unwrap());

        // Get seed name for tracking
        let seed_name = state.seeds[seed_idx].name()
            .unwrap_or("unknown")
            .to_string();

        // Output seed tracking info for UI
        println!("[SEED] index={} total={} name={}",
                 seed_idx, state.seeds.len(), seed_name);
        let _ = std::io::stdout().flush();

        // Get the input from corpus
        let input = state.seeds[seed_idx].clone();

        // Clone and mutate
        let mut mutated_input = input.clone();

        // Apply mutations using weighted selection
        let mut current_mutator_name: Option<&str> = None;
        if config.targeted_only {
            // Targeted-only mode: only use targeted mutator
            if let Some(ref mut tm) = targeted_mutator {
                for _ in 0..config.mutations_per_input {
                    let _ = tm.mutate(state, &mut mutated_input);
                }
                current_mutator_name = Some("targeted");
            }
        } else {
            let num_mutations = 1 + rand_below(state.rand_mut(), config.mutations_per_input);
            for _ in 0..num_mutations {
                // Use targeted mutator if available, with probability based on its weight
                if let Some(ref mut tm) = targeted_mutator {
                    let targeted_weight = weighted_mutators.iter()
                        .find(|(name, _)| *name == "targeted")
                        .map(|(_, w)| *w)
                        .unwrap_or(0);

                    if targeted_weight > 0 && rand_below(state.rand_mut(), total_weight as usize) < targeted_weight as usize {
                        let _ = tm.mutate(state, &mut mutated_input);
                        current_mutator_name = Some("targeted");
                        continue;
                    }
                }

                // Select mutator based on weighted random selection
                current_mutator_name = select_weighted_mutator(
                    &weighted_mutators,
                    total_weight,
                    state,
                    &mut bitflip_mutator,
                    &mut sb_mutator,
                    &mut inode_mutator,
                    &mut dir_mutator,
                    &mut xattr_mutator,
                    &mut mutated_input,
                );
            }
        }

        // Output current mutator for UI (to stdout for parent process to parse)
        if let Some(name) = current_mutator_name {
            println!("[MUTATOR] {}", name);
            // Flush stdout to ensure it's immediately visible to parent
            let _ = std::io::stdout().flush();
        }

        // Execute
        match executor.execute_with_output(&mutated_input) {
            Ok(output) if output.result.is_crash() => {
                info!("Found crash: {:?}", output.result);
                crashes += 1;
                handle_crash(config, &mutated_input, output.result, iterations, None, output.kernel_log.as_deref())?;
            }
            Ok(output) => match output.result {
                ExecutionResult::Timeout => {
                    debug!("Execution timeout");
                }
                ExecutionResult::Error(code) => {
                    // Check for ASan-related exit codes (134 = 128 + 6 = SIGABRT)
                    if code == 134 {
                        info!("Found ASan crash (exit code 134)!");
                        crashes += 1;
                        handle_crash(config, &mutated_input, ExecutionResult::AsanError, iterations, None, output.kernel_log.as_deref())?;
                    }
                    debug!("Execution error: {}", code);
                }
                ExecutionResult::Success => {
                    // Normal execution - could add to corpus if interesting
                }
                ExecutionResult::FailedToStart => {
                    debug!("Failed to start executor");
                }
                _ => {
                    // Other results
                }
            },
            Err(e) => {
                debug!("Execution error: {}", e);
            }
        }

        iterations += 1;

        // Output progress periodically (every 10 iterations or every 2 seconds)
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(last_progress_time).as_secs_f64();
        if iterations % 10 == 0 || elapsed >= 2.0 {
            let iter_delta = iterations.saturating_sub(last_progress_iterations);
            let speed = if elapsed > 0.0 {
                iter_delta as f64 / elapsed
            } else {
                0.0
            };
            println!("[PROGRESS] iteration={} crashes={} speed={:.2}", iterations, crashes, speed);
            let _ = std::io::stdout().flush();
            last_progress_time = now;
            last_progress_iterations = iterations;
        }

        if iterations % 100 == 0 {
            info!("Iterations: {}, Crashes: {}, Corpus: {}",
                  iterations, crashes, state.seeds.len());
        }
    }

    // Final progress report
    println!("[PROGRESS] iteration={} crashes={} speed=0.0", iterations, crashes);
    println!("[FUZZER] finished iterations={} crashes={}", iterations, crashes);
    let _ = std::io::stdout().flush();

    info!("Fuzzing finished. Total iterations: {}, Total crashes: {}", iterations, crashes);
    Ok(())
}

/// Build weighted mutator list from strategy configuration
fn build_weighted_mutators(config: &FuzzerConfig) -> Vec<(&'static str, u32)> {
    let mut mutators = Vec::new();

    // Default weights if no strategy config
    let defaults = [
        ("bitflip", 100u32),
        ("superblock", 100u32),
        ("inode", 100u32),
        ("dirent", 100u32),
        ("xattr", 50u32),
        ("targeted", 50u32),
    ];

    if let Some(ref strategy) = config.strategy_config {
        // Map CLI mutator names to internal names
        let name_map = [
            ("bitflip", "bitflip"),
            ("bit_flip", "bitflip"),
            ("random", "random"),
            ("zero", "zero"),
            ("max", "max"),
            ("arithmetic", "arithmetic"),
            ("interesting_values", "interesting_values"),
            ("boundary", "boundary"),
            ("superblock", "superblock"),
            ("inode", "inode"),
            ("dirent", "dirent"),
            ("xattr", "xattr"),
            ("targeted", "targeted"),
        ];

        for (cli_name, internal_name) in &name_map {
            // Only include mutators we actually use
            if !["bitflip", "superblock", "inode", "dirent", "xattr", "targeted"].contains(internal_name) {
                continue;
            }

            if let Some(m_config) = strategy.mutators.get(*cli_name) {
                if m_config.enabled {
                    mutators.push((*internal_name, m_config.weight));
                }
            } else if let Some(m_config) = strategy.mutators.get(*internal_name) {
                if m_config.enabled {
                    mutators.push((*internal_name, m_config.weight));
                }
            }
        }

        // If no mutators were enabled, use defaults
        if mutators.is_empty() {
            info!("No enabled mutators in strategy config, using defaults");
            for (name, weight) in defaults {
                mutators.push((name, weight));
            }
        }
    } else {
        // Use default weights
        for (name, weight) in defaults {
            mutators.push((name, weight));
        }
    }

    mutators
}

/// Select a mutator based on weighted random selection and apply it
fn select_weighted_mutator(
    weighted_mutators: &[(&'static str, u32)],
    total_weight: u32,
    state: &mut SimpleFuzzerState,
    bitflip_mutator: &mut ErofsBitflipMutator,
    sb_mutator: &mut ErofsSuperblockMutator,
    inode_mutator: &mut ErofsInodeMutator,
    dir_mutator: &mut ErofsDirectoryMutator,
    xattr_mutator: &mut ErofsXattrMutator,
    input: &mut erofs_input::ErofsImageInput,
) -> Option<&'static str> {
    if total_weight == 0 || weighted_mutators.is_empty() {
        return None;
    }

    // Select based on weighted random
    let mut choice = rand_below(state.rand_mut(), total_weight as usize) as u32;
    for (name, weight) in weighted_mutators {
        if choice < *weight {
            // Apply this mutator
            let _ = match *name {
                "bitflip" => bitflip_mutator.mutate(state, input),
                "superblock" => sb_mutator.mutate(state, input),
                "inode" => inode_mutator.mutate(state, input),
                "dirent" => dir_mutator.mutate(state, input),
                "xattr" => xattr_mutator.mutate(state, input),
                _ => return None, // Skip unknown mutators like targeted (handled separately)
            };
            return Some(*name);
        }
        choice -= weight;
    }

    // Fallback to bitflip
    let _ = bitflip_mutator.mutate(state, input);
    Some("bitflip")
}

/// Detect kernel version and EROFS version
fn detect_versions(config: &FuzzerConfig) -> (Option<String>, Option<String>) {
    let kernel_version = detect_kernel_version(config);
    let erofs_version = detect_erofs_version(config);
    (kernel_version, erofs_version)
}

/// Detect kernel version from kernel file or running system
fn detect_kernel_version(config: &FuzzerConfig) -> Option<String> {
    match config.executor_type {
        ExecutorType::QemuKernel => {
            let kernel_path = &config.kernel_path;

            // Try to read kernel version string from the file
            if let Ok(data) = std::fs::read(kernel_path) {
                // Search for "Linux version X.Y.Z" string
                if let Some(version) = find_linux_version(&data) {
                    return Some(version);
                }

                // Alternative: look for version string like "X.Y.Z (user@host)"
                // This is common in kernel builds
                if let Some(version) = find_kernel_version_string(&data) {
                    return Some(version);
                }
            }

            // Try to extract version from kernel file name
            let file_name = kernel_path.file_name()?.to_str()?;

            // Common patterns: bzImage-5.15.0, vmlinuz-5.15.0
            if file_name.starts_with("bzImage-") || file_name.starts_with("vmlinuz-") {
                let version = file_name
                    .strip_prefix("bzImage-")
                    .or_else(|| file_name.strip_prefix("vmlinuz-"));
                return version.map(|s| s.to_string());
            }

            // Check if parent directory has version info (e.g., linux-6.8/bzImage)
            if let Some(parent) = kernel_path.parent() {
                if let Some(parent_name) = parent.file_name().and_then(|n| n.to_str()) {
                    // Pattern: linux-6.8, kernel-5.15.0
                    if let Some(version) = parent_name.strip_prefix("linux-")
                        .or_else(|| parent_name.strip_prefix("kernel-"))
                    {
                        // Check if it looks like a version
                        if version.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                            return Some(version.to_string());
                        }
                    }
                }
            }

            None
        }
        ExecutorType::Erofsfuse => {
            // For erofsfuse, get the running kernel version
            std::process::Command::new("uname")
                .arg("-r")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
        }
    }
}

/// Find "Linux version X.Y.Z" string in kernel binary data
fn find_linux_version(data: &[u8]) -> Option<String> {
    // Look for "Linux version " followed by version number
    let pattern = b"Linux version ";
    let start = data.windows(pattern.len())
        .position(|w| w == pattern)?;

    let rest = &data[start + pattern.len()..];

    // Version is typically like "5.15.0"
    let version_end = rest.iter()
        .position(|&b| !b.is_ascii_digit() && b != b'.')
        .unwrap_or(rest.len().min(20));

    if version_end > 0 {
        String::from_utf8(rest[..version_end].to_vec()).ok()
    } else {
        None
    }
}

/// Find kernel version string in format "X.Y.Z (user@host)" common in kernel builds
fn find_kernel_version_string(data: &[u8]) -> Option<String> {
    // The kernel version string is embedded as "X.Y.Z (user@host)"
    // We search for this pattern directly

    for i in 0..data.len().saturating_sub(10) {
        // Check if this position looks like version start (digit)
        if data[i].is_ascii_digit() {
            // Try to parse version: X.Y.Z
            let mut j = i;
            let mut dot_count = 0;
            let mut has_digit_after_dot = false;

            while j < data.len() && j < i + 20 {
                let c = data[j];
                if c.is_ascii_digit() {
                    if dot_count > 0 {
                        has_digit_after_dot = true;
                    }
                    j += 1;
                } else if c == b'.' {
                    dot_count += 1;
                    j += 1;
                } else {
                    break;
                }
            }

            // Check if we have a valid version (at least X.Y format)
            if dot_count >= 1 && has_digit_after_dot && j < data.len() {
                // Check if followed by space and '(' (kernel version string pattern)
                if data[j] == b' ' && j + 1 < data.len() && data[j + 1] == b'(' {
                    if let Ok(version) = std::str::from_utf8(&data[i..j]) {
                        return Some(version.to_string());
                    }
                }
            }
        }
    }

    None
}

/// Detect EROFS version
fn detect_erofs_version(config: &FuzzerConfig) -> Option<String> {
    match config.executor_type {
        ExecutorType::QemuKernel => {
            // For kernel fuzzing, EROFS is built into the kernel.
            // Try to find EROFS version from kernel source or config.
            let kernel_path = &config.kernel_path;

            // Check kernel source directory for EROFS version
            if let Some(parent) = kernel_path.parent() {
                // Check common kernel source locations
                let source_paths = [
                    parent.join("fs/erofs/erofs.h"),  // In kernel source tree
                    parent.join("../fs/erofs/erofs.h"), // Relative to arch/x86/boot
                ];

                for source_path in &source_paths {
                    if source_path.exists() {
                        if let Ok(data) = std::fs::read(source_path) {
                            let data_str = String::from_utf8_lossy(&data);
                            // Look for EROFS_VERSION or similar macro
                            for line in data_str.lines() {
                                if line.contains("EROFS_VERSION") || line.contains("EROFS_VERSION_CODE") {
                                    // Extract version number from #define
                                    if let Some(eq_idx) = line.find('"') {
                                        let rest = &line[eq_idx + 1..];
                                        if let Some(end_idx) = rest.find('"') {
                                            return Some(rest[..end_idx].to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Check for .config with EROFS enabled
                let config_paths = [
                    parent.join(".config"),
                    parent.join("../.config"),
                ];

                for config_path in &config_paths {
                    if config_path.exists() {
                        if let Ok(data) = std::fs::read(config_path) {
                            let data_str = String::from_utf8_lossy(&data);
                            // Just indicate EROFS is enabled in kernel
                            if data_str.contains("CONFIG_EROFS_FS=y") {
                                return Some("in-kernel".to_string());
                            }
                        }
                    }
                }
            }

            // Indicate it's in kernel mode, version unknown
            Some("in-kernel".to_string())
        }
        ExecutorType::Erofsfuse => {
            // Try to get erofsfuse version
            let erofsfuse_path = &config.erofsfuse_path;

            // Try --version first (may output to stdout or stderr)
            let output = std::process::Command::new(erofsfuse_path)
                .arg("--version")
                .output()
                .ok();

            if let Some(ref o) = output {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                let combined = format!("{} {}", stdout, stderr);

                // Parse version from output like "erofsfuse (erofs-utils) 1.7.1"
                // or "erofs-utils 1.7.1"
                for line in combined.lines() {
                    let line_lower = line.to_lowercase();
                    if line_lower.contains("erofs") || line_lower.contains("fuse") {
                        // Try to extract version number (X.Y.Z format)
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        for part in parts {
                            // Check if this looks like a version number
                            if part.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                                let version_part: String = part.chars()
                                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                                    .collect();
                                if version_part.contains('.') {
                                    return Some(version_part);
                                }
                            }
                        }
                    }
                }

                // If no version found in parsed lines, return first non-empty line
                for line in stdout.lines().chain(stderr.lines()) {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }

            // Try -V flag as fallback
            let output = std::process::Command::new(erofsfuse_path)
                .arg("-V")
                .output()
                .ok();

            if let Some(o) = output {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);

                for line in stdout.lines().chain(stderr.lines()) {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_string());
                    }
                }
            }

            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_config_validation() {
        let config = FuzzerConfig::from(CliArgs::try_parse_from(["test", "--seeds", "/nonexistent"]).unwrap());
        let result = validate_config(&config);
        assert!(result.is_err());
    }
}
