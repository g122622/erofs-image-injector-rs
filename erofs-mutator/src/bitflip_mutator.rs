//! Bitflip Mutator for EROFS Images
//!
//! Simple bit-flip mutation strategy that operates on raw bytes.

use libafl::mutators::{Mutator, MutationResult};
use libafl::state::HasMetadata;
use libafl_bolts::rands::Rand;
use libafl_bolts::Error;

use crate::{
    arithmetic_mutate, flip_random_bit, interesting_value_mutate, rand_in_range, set_random_bytes,
};
use erofs_input::ErofsImageInput;

/// Bitflip mutator for EROFS images
///
/// Performs random bit-flips, byte mutations, and interesting value
/// injections on the raw image data.
#[derive(Debug, Default)]
pub struct ErofsBitflipMutator {
    /// Maximum number of mutations per invocation
    max_mutations: usize,
}

impl ErofsBitflipMutator {
    /// Create a new bitflip mutator
    pub fn new() -> Self {
        Self { max_mutations: 4 }
    }

    /// Create with custom max mutations
    pub fn with_max_mutations(max_mutations: usize) -> Self {
        Self { max_mutations }
    }

    /// Perform a random mutation on the data
    fn mutate_data<R: Rand>(&self, data: &mut [u8], rng: &mut R) -> MutationResult {
        if data.is_empty() {
            return MutationResult::Skipped;
        }

        let mutation_type = rng.below(6);

        match mutation_type {
            0 => {
                // Single bit flip
                flip_random_bit(data, rng);
                MutationResult::Mutated
            }
            1 => {
                // Multiple bit flips
                let count = 1 + rng.below(4);
                for _ in 0..count {
                    flip_random_bit(data, rng);
                }
                MutationResult::Mutated
            }
            2 => {
                // Random bytes
                let offset = rng.below(data.len() as u64) as usize;
                let len = std::cmp::min(1 + rng.below(8) as usize, data.len() - offset);
                set_random_bytes(&mut data[offset..offset + len], rng);
                MutationResult::Mutated
            }
            3 => {
                // Arithmetic mutation
                let offset = rng.below(data.len() as u64) as usize;
                let len = std::cmp::min(1 + rng.below(4) as usize, data.len() - offset);
                for _ in 0..len {
                    arithmetic_mutate(&mut data[offset..], rng);
                }
                MutationResult::Mutated
            }
            4 => {
                // Interesting value
                let sizes = [1, 2, 4, 8];
                let size = sizes[rng.below(sizes.len() as u64) as usize];
                if data.len() >= size {
                    let offset = rng.below((data.len() - size + 1) as u64) as usize;
                    interesting_value_mutate(&mut data[offset..offset + size], rng, size);
                    MutationResult::Mutated
                } else {
                    MutationResult::Skipped
                }
            }
            5 => {
                // Block deletion/insertion (rare)
                let offset = rng.below(data.len() as u64) as usize;
                let max_block = std::cmp::min(64, data.len() - offset);
                if max_block > 0 {
                    let block_size = 1 + rng.below(max_block as u64) as usize;
                    // Overwrite with zeros or duplicate adjacent
                    if offset + block_size < data.len() {
                        data[offset..offset + block_size].fill(0);
                    }
                    MutationResult::Mutated
                } else {
                    MutationResult::Skipped
                }
            }
            _ => MutationResult::Skipped,
        }
    }
}

impl<S> Mutator<ErofsImageInput, S> for ErofsBitflipMutator
where
    S: HasMetadata,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut ErofsImageInput,
    ) -> Result<MutationResult, Error> {
        let rng = state.rand_mut();
        let data = input.data_mut();

        if data.is_empty() {
            return Ok(MutationResult::Skipped);
        }

        // Apply multiple mutations
        let mut result = MutationResult::Skipped;
        for _ in 0..self.max_mutations {
            if rng.below(3) == 0 {
                // 1/3 chance to apply mutation
                let mutation_result = self.mutate_data(data, rng);
                if mutation_result == MutationResult::Mutated {
                    result = MutationResult::Mutated;
                }
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libafl_bolts::rands::StdRand;

    struct TestState {
        rand: StdRand,
    }

    impl TestState {
        fn new() -> Self {
            Self {
                rand: StdRand::with_seed(42),
            }
        }
    }

    impl HasMetadata for TestState {}

    impl libafl::state::UsesRand for TestState {
        type Rand = StdRand;

        fn rand(&self) -> &Self::Rand {
            &self.rand
        }

        fn rand_mut(&mut self) -> &mut Self::Rand {
            &mut self.rand
        }
    }

    #[test]
    fn test_bitflip_mutator() {
        let mut mutator = ErofsBitflipMutator::new();
        let mut state = TestState::new();
        let mut input = ErofsImageInput::new(vec![0u8; 1024]);

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        assert_eq!(result, MutationResult::Mutated);
        // Data should have been modified
        assert_ne!(input.data(), &[0u8; 1024]);
    }

    #[test]
    fn test_empty_input() {
        let mut mutator = ErofsBitflipMutator::new();
        let mut state = TestState::new();
        let mut input = ErofsImageInput::empty();

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        assert_eq!(result, MutationResult::Skipped);
    }
}
