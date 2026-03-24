//! Main Fuzzer Logic
//!
//! Core fuzzing loop and orchestration.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use libafl::corpus::{Corpus, InMemoryCorpus, OnDiskCorpus};
use libafl::events::SimpleEventManager;
use libafl::feedback_and_fast, feedback_or;
use libafl::feedbacks::{CrashFeedback, MaxMapFeedback, TimeFeedback};
use libafl::fuzzer::{Fuzzer, StdFuzzer};
use libafl::inputs::Input;
use libafl::monitors::SimpleMonitor;
use libafl::mutators::{HavocScheduledMutator, havoc_mutations};
use libafl::observers::{CanTrack, HitcountsMapObserver, StdMapObserver, TimeObserver};
use libafl::schedulers::QueueScheduler;
use libafl::stages::mutational::StdMutationalStage;
use libafl::state::{HasCorpus, StdState};
use libafl_bolts::current_nanos;
use libafl_bolts::rands::StdRand;
use libafl_bolts::shmem::{ShMem, ShMemProvider, UnixShMemProvider};
use libafl_bolts::tuples::tuple_list;
use tracing::{debug, error, info, warn};

use erofs_input::ErofsImageInput;
use erofs_mutator::{
    ErofsBitflipMutator, ErofsDirectoryMutator, ErofsInodeMutator, ErofsSuperblockMutator,
    ErofsXattrMutator,
};

use crate::cli::CliArgs;
use crate::executor::{ErofsfuseExecutor, ErofsfuseExit};

pub use crate::cli::FuzzerConfig;

/// Fuzzer error types
#[derive(Debug, thiserror::Error)]
pub enum FuzzerError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// LibAFL error
    #[error("LibAFL error: {0}")]
    LibAfl(#[from] libafl_bolts::Error),

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

    // Initialize the fuzzer components
    let monitor = SimpleMonitor::new(|s| info!("{}", s));

    // Create the event manager
    let mut mgr = SimpleEventManager::new(monitor);

    // Create the corpus
    let mut corpus = InMemoryCorpus::new();

    // Create solutions corpus (crashes)
    let solutions = OnDiskCorpus::new(&config.output_dir)?;

    // Create the feedback
    let time_observer = TimeObserver::new("time");

    // Create feedback to rate inputs
    let mut feedback = feedback_or!(
        TimeFeedback::new(&time_observer),
    );

    // Objective: we want crashes
    let mut objective = CrashFeedback::new();

    // Create state
    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()),
        corpus,
        solutions,
        &mut feedback,
        &mut objective,
    )?;

    // Load seeds
    load_seeds(&mut state, &config)?;

    // Check if we have any seeds
    if state.corpus().count() == 0 {
        return Err(FuzzerError::NoSeeds(config.seeds_dir.display().to_string()));
    }

    info!("Loaded {} seeds", state.corpus().count());

    // Create mutators
    let mutators = tuple_list!(
        ErofsSuperblockMutator::new(),
        ErofsInodeMutator::new(),
        ErofsDirectoryMutator::new(),
        ErofsXattrMutator::new(),
        ErofsBitflipMutator::new(),
    );

    let mutator_scheduler = HavocScheduledMutator::new(mutators);

    // Create stages
    let mut stages = tuple_list!(
        StdMutationalStage::new(mutator_scheduler)
    );

    // Create scheduler
    let scheduler = QueueScheduler::new();

    // Create fuzzer
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    // Create executor
    let executor = ErofsfuseExecutor::new(&config);

    // Run the fuzzing loop
    info!("Starting fuzzing loop...");

    let max_iterations = config.max_iterations;
    let mut iterations = 0;

    loop {
        // Check iteration limit
        if max_iterations > 0 && iterations >= max_iterations {
            info!("Reached max iterations: {}", max_iterations);
            break;
        }

        // Get next input from corpus
        let corpus_id = match fuzzer.schedule(&state) {
            Some(id) => id,
            None => {
                warn!("No inputs in corpus");
                break;
            }
        };

        // Get the input
        let input = state.corpus().get(corpus_id)?.borrow().input().cloned();

        if let Some(input) = input {
            let input = if let Some(input) = input.as_any().downcast_ref::<ErofsImageInput>() {
                input.clone()
            } else {
                warn!("Input is not ErofsImageInput");
                continue;
            };

            // Execute
            let exit_kind = execute_input(&executor, &input)?;

            // Check for crash
            match exit_kind {
                ErofsfuseExit::Crashed(signal) => {
                    info!("Found crash with signal {}!", signal);

                    // Save crash
                    let crash_path = config.output_dir.join(format!(
                        "crash-{:016x}-signal-{}.erofs",
                        current_nanos(),
                        signal
                    ));
                    std::fs::write(&crash_path, input.data())?;
                    info!("Crash saved to {:?}", crash_path);
                }
                ErofsfuseExit::Timeout => {
                    debug!("Execution timeout");
                }
                ErofsfuseExit::Error(code) => {
                    debug!("Execution error: {}", code);
                }
                ErofsfuseExit::Success | ErofsfuseExit::FailedToStart => {
                    // Normal execution
                }
            }

            iterations += 1;

            if iterations % 100 == 0 {
                info!("Iterations: {}", iterations);
            }
        }

        // Mutate and add to corpus
        let mut new_input = state.corpus().get(corpus_id)?.borrow().input().cloned().unwrap();
        // Note: In a real implementation, we would apply mutations here
    }

    info!("Fuzzing completed. Total iterations: {}", iterations);
    Ok(())
}

/// Execute an input and return the exit kind
fn execute_input(executor: &ErofsfuseExecutor, input: &ErofsImageInput) -> FuzzerResult<ErofsfuseExit> {
    // This is a simplified version - in practice, we'd integrate with LibAFL's executor framework
    let mut executor = executor.clone();
    executor.execute(input).map_err(|e| FuzzerError::Executor(e.to_string()))
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

/// Load seed images from the seeds directory
fn load_seeds<S>(state: &mut StdState<ErofsImageInput, S>, config: &FuzzerConfig) -> FuzzerResult<()>
where
    S: libafl::state::HasCorpus + libafl::state::HasRand,
{
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
                        state.corpus_mut().add(input)?;
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

    info!("Loaded {} seed images", count);
    Ok(())
}

/// Simple fuzzer for testing without full LibAFL integration
pub struct SimpleFuzzer {
    /// Configuration
    config: FuzzerConfig,

    /// Seeds
    seeds: Vec<ErofsImageInput>,

    /// Crashes found
    crashes: Vec<ErofsImageInput>,

    /// Iterations completed
    iterations: u64,
}

impl SimpleFuzzer {
    /// Create a new simple fuzzer
    pub fn new(config: FuzzerConfig) -> Self {
        Self {
            config,
            seeds: Vec::new(),
            crashes: Vec::new(),
            iterations: 0,
        }
    }

    /// Load seeds
    pub fn load_seeds(&mut self) -> FuzzerResult<()> {
        let entries = std::fs::read_dir(&self.config.seeds_dir)?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                let data = std::fs::read(&path)?;
                self.seeds.push(ErofsImageInput::new(data));
            }
        }

        info!("Loaded {} seeds", self.seeds.len());
        Ok(())
    }

    /// Run the fuzzer
    pub fn run(&mut self) -> FuzzerResult<()> {
        if self.seeds.is_empty() {
            return Err(FuzzerError::NoSeeds(self.config.seeds_dir.display().to_string()));
        }

        let mut executor = ErofsfuseExecutor::new(&self.config);
        let mut rng = StdRand::with_seed(current_nanos());
        let mut bitflip_mutator = ErofsBitflipMutator::new();
        let mut sb_mutator = ErofsSuperblockMutator::new();

        std::fs::create_dir_all(&self.config.output_dir)?;

        loop {
            // Check iteration limit
            if self.config.max_iterations > 0 && self.iterations >= self.config.max_iterations {
                info!("Reached max iterations: {}", self.config.max_iterations);
                break;
            }

            // Pick a random seed
            let seed_idx = rng.below(self.seeds.len() as u64) as usize;
            let mut input = self.seeds[seed_idx].clone();

            // Mutate
            let _ = bitflip_mutator.mutate(&mut rng, &mut input);
            let _ = sb_mutator.mutate(&mut rng, &mut input);

            // Execute
            match executor.execute(&input) {
                Ok(ErofsfuseExit::Crashed(signal)) => {
                    info!("Found crash with signal {}!", signal);

                    // Save crash
                    let crash_path = self.config.output_dir.join(format!(
                        "crash-{:016x}-signal-{}.erofs",
                        current_nanos(),
                        signal
                    ));
                    std::fs::write(&crash_path, input.data())?;
                    self.crashes.push(input);
                }
                Ok(ErofsfuseExit::Success) => {
                    // Could add to corpus if interesting
                }
                Ok(_) => {}
                Err(e) => {
                    debug!("Execution error: {}", e);
                }
            }

            self.iterations += 1;

            if self.iterations % 100 == 0 {
                info!("Iterations: {}, Crashes: {}", self.iterations, self.crashes.len());
            }
        }

        info!("Fuzzing completed. Total crashes: {}", self.crashes.len());
        Ok(())
    }

    /// Get crashes
    pub fn crashes(&self) -> &[ErofsImageInput] {
        &self.crashes
    }

    /// Get iterations
    pub fn iterations(&self) -> u64 {
        self.iterations
    }
}

impl Clone for ErofsfuseExecutor {
    fn clone(&self) -> Self {
        Self {
            erofsfuse_path: self.erofsfuse_path.clone(),
            mount_base: self.mount_base.clone(),
            timeout: self.timeout,
            max_size: self.max_size,
            min_size: self.min_size,
            keep_temp: self.keep_temp,
            executions: 0,
        }
    }
}

impl libafl::state::HasRand for StdRand {
    type Rand = StdRand;

    fn rand(&self) -> &Self::Rand {
        self
    }

    fn rand_mut(&mut self) -> &mut Self::Rand {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_config_validation() {
        let config = FuzzerConfig::from(CliArgs::parse_from(["test", "--seeds", "/nonexistent"]));
        let result = validate_config(&config);
        assert!(result.is_err());
    }
}
