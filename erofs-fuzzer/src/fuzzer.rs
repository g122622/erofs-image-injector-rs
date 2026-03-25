//! Main Fuzzer Logic
//!
//! Core fuzzing loop and orchestration.

use std::num::NonZeroUsize;

use libafl::corpus::{Corpus, InMemoryCorpus, Testcase};
use libafl::mutators::Mutator;
use libafl::state::{HasCorpus, HasRand};
use libafl_bolts::current_nanos;
use libafl_bolts::rands::{Rand, StdRand};
use tracing::{debug, info, warn};

use erofs_input::ErofsImageInput;
use erofs_mutator::{
    ErofsBitflipMutator, ErofsDirectoryMutator, ErofsInodeMutator, ErofsSuperblockMutator,
    ErofsXattrMutator, rand_below,
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

        // Apply multiple mutations
        let num_mutations = 1 + rand_below(state.rand_mut(), config.mutations_per_input);
        for _ in 0..num_mutations {
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

        // Execute
        match executor.execute(&mutated_input) {
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
            Ok(ErofsfuseExit::Timeout) => {
                debug!("Execution timeout");
            }
            Ok(ErofsfuseExit::Error(code)) => {
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
