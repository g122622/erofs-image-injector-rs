//! Main Fuzzer Logic
//!
//! Core fuzzing loop and orchestration.

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

use crate::cli::{CliArgs, FuzzerConfig};
use crate::executor::{ErofsfuseExecutor, ErofsfuseExit};

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
    // Check erofsfuse exists
    if !config.erofsfuse_path.exists() {
        return Err(FuzzerError::Config(format!(
            "erofsfuse not found at {:?}",
            config.erofsfuse_path
        )));
    }

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

    Ok(())
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
                        let input = ErofsImageInput::new(data);
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

/// Simple fuzzer implementation without coverage-guided feedback
fn run_simple_fuzzer(config: &FuzzerConfig, state: &mut SimpleFuzzerState) -> FuzzerResult<()> {
    let mut executor = ErofsfuseExecutor::new(config);
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

    // First, test all seeds without mutation to find existing crashes
    info!("Testing {} seed images without mutation...", state.seeds.len());
    for (idx, seed) in state.seeds.iter().enumerate() {
        info!("Testing seed {} of {}", idx + 1, state.seeds.len());
        match executor.execute_erofs(seed) {
            Ok(ErofsfuseExit::Crashed(signal)) => {
                info!("Found crash in seed with signal {}!", signal);
                crashes += 1;

                // Save crash
                let crash_path = config.output_dir.join(format!(
                    "seed-crash-{:016x}-signal-{}.erofs",
                    current_nanos(),
                    signal
                ));
                std::fs::write(&crash_path, seed.data())?;
                info!("Crash saved to {:?}", crash_path);

                // Also save crash info
                let info_path = config.output_dir.join(format!(
                    "seed-crash-{:016x}-signal-{}.log",
                    current_nanos(),
                    signal
                ));
                let info = format!(
                    "Signal: {}\nIteration: 0 (seed)\nSize: {} bytes\nSeed index: {}\n",
                    signal,
                    seed.len(),
                    idx
                );
                let _ = std::fs::write(&info_path, info);
            }
            Ok(ErofsfuseExit::AsanError) => {
                info!("Found ASan error in seed!");
                crashes += 1;

                let crash_path = config.output_dir.join(format!(
                    "seed-crash-{:016x}-asan.erofs",
                    current_nanos()
                ));
                std::fs::write(&crash_path, seed.data())?;
                info!("ASan crash saved to {:?}", crash_path);

                let info_path = config.output_dir.join(format!(
                    "seed-crash-{:016x}-asan.log",
                    current_nanos()
                ));
                let info = format!(
                    "Type: AddressSanitizer Error\nIteration: 0 (seed)\nSize: {} bytes\nSeed index: {}\n",
                    seed.len(),
                    idx
                );
                let _ = std::fs::write(&info_path, info);
            }
            Ok(ErofsfuseExit::Timeout) => {
                debug!("Seed execution timeout");
            }
            Ok(ErofsfuseExit::Error(code)) => {
                // Check for ASan-related exit codes
                if code == 134 {
                    info!("Found ASan crash in seed (exit code 134)!");
                    crashes += 1;

                    let crash_path = config.output_dir.join(format!(
                        "seed-crash-{:016x}-asan.erofs",
                        current_nanos()
                    ));
                    std::fs::write(&crash_path, seed.data())?;
                    info!("Crash saved to {:?}", crash_path);
                }
            }
            Ok(_) => {
                // Normal execution
            }
            Err(e) => {
                debug!("Seed execution error: {}", e);
            }
        }
    }

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

        // Get the input from corpus
        let input = state.seeds[seed_idx].clone();

        // Clone and mutate
        let mut mutated_input = input.clone();

        // Apply mutations
        if config.targeted_only {
            // Targeted-only mode: only use targeted mutator
            if let Some(ref mut tm) = targeted_mutator {
                for _ in 0..config.mutations_per_input {
                    let _ = tm.mutate(state, &mut mutated_input);
                }
            }
        } else {
            // Mixed mode: combine targeted mutations with random mutations
            let num_mutations = 1 + rand_below(state.rand_mut(), config.mutations_per_input);
            for _ in 0..num_mutations {
                // Use targeted mutator if available, with 30% probability
                if let Some(ref mut tm) = targeted_mutator {
                    if rand_below(state.rand_mut(), 10) < 3 {
                        let _ = tm.mutate(state, &mut mutated_input);
                        continue;
                    }
                }

                // Otherwise use random mutators
                let mutation_choice = rand_below(state.rand_mut(), 5);
                let _ = match mutation_choice {
                    0 => bitflip_mutator.mutate(state, &mut mutated_input),
                    1 => sb_mutator.mutate(state, &mut mutated_input),
                    2 => inode_mutator.mutate(state, &mut mutated_input),
                    3 => dir_mutator.mutate(state, &mut mutated_input),
                    4 => xattr_mutator.mutate(state, &mut mutated_input),
                    _ => bitflip_mutator.mutate(state, &mut mutated_input),
                };
            }
        }

        // Execute
        match executor.execute_erofs(&mutated_input) {
            Ok(ErofsfuseExit::Crashed(signal)) => {
                info!("Found crash with signal {}!", signal);
                crashes += 1;

                // Save crash
                let crash_path = config.output_dir.join(format!(
                    "crash-{:016x}-signal-{}.erofs",
                    current_nanos(),
                    signal
                ));
                std::fs::write(&crash_path, mutated_input.data())?;
                info!("Crash saved to {:?}", crash_path);

                // Also save crash info
                let info_path = config.output_dir.join(format!(
                    "crash-{:016x}-signal-{}.log",
                    current_nanos(),
                    signal
                ));
                let info = format!(
                    "Signal: {}\nIteration: {}\nSize: {} bytes\n",
                    signal,
                    iterations,
                    mutated_input.len()
                );
                let _ = std::fs::write(&info_path, info);
            }
            Ok(ErofsfuseExit::AsanError) => {
                info!("Found ASan memory error!");
                crashes += 1;

                // Save crash
                let crash_path = config.output_dir.join(format!(
                    "crash-{:016x}-asan.erofs",
                    current_nanos()
                ));
                std::fs::write(&crash_path, mutated_input.data())?;
                info!("ASan crash saved to {:?}", crash_path);

                // Also save crash info
                let info_path = config.output_dir.join(format!(
                    "crash-{:016x}-asan.log",
                    current_nanos()
                ));
                let info = format!(
                    "Type: AddressSanitizer Error\nIteration: {}\nSize: {} bytes\n",
                    iterations,
                    mutated_input.len()
                );
                let _ = std::fs::write(&info_path, info);
            }
            Ok(ErofsfuseExit::Timeout) => {
                debug!("Execution timeout");
            }
            Ok(ErofsfuseExit::Error(code)) => {
                // Check for ASan-related exit codes (134 = 128 + 6 = SIGABRT)
                if code == 134 {
                    info!("Found ASan crash (exit code 134)!");
                    crashes += 1;

                    let crash_path = config.output_dir.join(format!(
                        "crash-{:016x}-asan.erofs",
                        current_nanos()
                    ));
                    std::fs::write(&crash_path, mutated_input.data())?;
                    info!("Crash saved to {:?}", crash_path);

                    let info_path = config.output_dir.join(format!(
                        "crash-{:016x}-asan.log",
                        current_nanos()
                    ));
                    let info = format!(
                        "ASan crash (exit code 134)\nIteration: {}\nSize: {} bytes\n",
                        iterations,
                        mutated_input.len()
                    );
                    let _ = std::fs::write(&info_path, info);
                }
                debug!("Execution error: {}", code);
            }
            Ok(ErofsfuseExit::Success) => {
                // Normal execution - could add to corpus if interesting
            }
            Ok(ErofsfuseExit::FailedToStart) => {
                debug!("Failed to start erofsfuse");
            }
            Err(e) => {
                debug!("Execution error: {}", e);
            }
        }

        iterations += 1;

        if iterations % 100 == 0 {
            info!("Iterations: {}, Crashes: {}, Corpus: {}",
                  iterations, crashes, state.seeds.len());
        }
    }

    info!("Fuzzing finished. Total iterations: {}, Total crashes: {}", iterations, crashes);
    Ok(())
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
