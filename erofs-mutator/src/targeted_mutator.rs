//! Targeted Mutator for EROFS Images
//!
//! This module provides precise mutation capabilities that allow targeting
//! specific fields or byte ranges within EROFS images.

use libafl::corpus::CorpusId;
use libafl::mutators::{Mutator, MutationResult};
use libafl::state::HasRand;
use libafl_bolts::Named;
use libafl_bolts::rands::Rand;
use libafl_bolts::Error;
use std::borrow::Cow;
use tracing::{debug, trace};

use erofs_input::{
    ErofsImageInput, MutationStrategy, MutationTarget, TargetLocation,
};
use crate::field_locator::FieldLocator;
use crate::{rand_below, interesting_value_mutate, set_random_bytes};

/// Targeted mutator for precise EROFS image mutations
#[derive(Debug)]
pub struct TargetedMutator {
    /// Target specification
    target: MutationTarget,
    /// Mutation strategy
    strategy: MutationStrategy,
    /// Maximum mutations to apply
    max_mutations: usize,
    /// Mutations applied so far
    mutations_applied: usize,
}

impl Named for TargetedMutator {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("TargetedMutator");
        &NAME
    }
}

impl TargetedMutator {
    /// Create a new targeted mutator
    pub fn new(target: MutationTarget, strategy: MutationStrategy) -> Self {
        Self {
            target,
            strategy,
            max_mutations: 1,
            mutations_applied: 0,
        }
    }

    /// Create a targeted mutator from configuration
    pub fn from_config(config: &erofs_input::TargetedMutationConfig) -> Self {
        Self {
            target: config.target.clone(),
            strategy: config.strategy.clone(),
            max_mutations: config.max_mutations,
            mutations_applied: 0,
        }
    }

    /// Set maximum mutations
    pub fn with_max_mutations(mut self, max: usize) -> Self {
        self.max_mutations = max;
        self
    }

    /// Locate the target in the image
    fn locate_target(&self, input: &ErofsImageInput) -> Option<TargetLocation> {
        let data = input.data();

        match &self.target {
            MutationTarget::FieldRange { field, offset_before, offset_after } => {
                FieldLocator::locate_with_offsets(data, field, *offset_before, *offset_after)
            }
            MutationTarget::AbsoluteRange { start, length } => {
                if *start + *length <= data.len() {
                    Some(TargetLocation::new(*start, *length))
                } else {
                    None
                }
            }
            MutationTarget::InodeByIndex { index, field, offset_before, offset_after } => {
                // TODO: Implement inode by index lookup
                if *index == 0 {
                    // Use first inode (root)
                    let base = FieldLocator::locate_first_inode_field(data, field)?;
                    let new_offset = base.offset.saturating_sub(*offset_before);
                    let new_length = base.length + offset_before + offset_after;
                    let max_length = data.len().saturating_sub(new_offset);
                    Some(TargetLocation::new(new_offset, new_length.min(max_length)))
                } else {
                    None
                }
            }
            MutationTarget::DirentByIndex { index, target_part } => {
                FieldLocator::locate_dirent_by_index(data, *index, *target_part)
            }
            MutationTarget::DataBlock { block_num, offset_in_block, length } => {
                FieldLocator::locate_data_block(data, *block_num, *offset_in_block, *length)
            }
        }
    }

    /// Apply the mutation strategy to the target region
    fn apply_strategy<R: Rand>(
        &self,
        data: &mut [u8],
        location: TargetLocation,
        rng: &mut R,
    ) -> MutationResult {
        let region = &mut data[location.offset..location.offset + location.length];

        match &self.strategy {
            MutationStrategy::BitFlip { count } => {
                self.apply_bitflips(region, *count, rng)
            }
            MutationStrategy::ByteReplace { values } => {
                self.apply_byte_replace(region, values)
            }
            MutationStrategy::Arithmetic { min_delta, max_delta } => {
                self.apply_arithmetic(region, *min_delta, *max_delta, rng)
            }
            MutationStrategy::InterestingValues { size } => {
                self.apply_interesting_values(region, *size, rng)
            }
            MutationStrategy::Boundary { values } => {
                self.apply_boundary_values(region, values, rng)
            }
            MutationStrategy::Random => {
                self.apply_random(region, rng)
            }
            MutationStrategy::Zero => {
                self.apply_zero(region)
            }
            MutationStrategy::Max => {
                self.apply_max(region)
            }
        }
    }

    /// Apply bit flips
    fn apply_bitflips<R: Rand>(&self, data: &mut [u8], count: usize, rng: &mut R) -> MutationResult {
        if data.is_empty() {
            return MutationResult::Skipped;
        }

        let flips = count.min(data.len() * 8);
        for _ in 0..flips {
            let byte_idx = rand_below(rng, data.len());
            let bit_idx = rand_below(rng, 8) as u8;
            data[byte_idx] ^= 1 << bit_idx;
        }

        debug!("Applied {} bit flips", flips);
        MutationResult::Mutated
    }

    /// Apply byte replacement
    fn apply_byte_replace(&self, data: &mut [u8], values: &[u8]) -> MutationResult {
        if data.is_empty() || values.is_empty() {
            return MutationResult::Skipped;
        }

        let copy_len = values.len().min(data.len());
        data[..copy_len].copy_from_slice(&values[..copy_len]);

        debug!("Replaced {} bytes", copy_len);
        MutationResult::Mutated
    }

    /// Apply arithmetic mutation
    fn apply_arithmetic<R: Rand>(
        &self,
        data: &mut [u8],
        min_delta: i8,
        max_delta: i8,
        rng: &mut R,
    ) -> MutationResult {
        if data.is_empty() {
            return MutationResult::Skipped;
        }

        let range = (max_delta - min_delta + 1) as usize;
        let delta = if range == 0 {
            0i8
        } else {
            min_delta + (rand_below(rng, range) as i8)
        };

        // Apply delta to the first byte or as a multi-byte value
        let idx = rand_below(rng, data.len());
        data[idx] = data[idx].wrapping_add(delta as u8);

        debug!("Applied arithmetic mutation with delta {}", delta);
        MutationResult::Mutated
    }

    /// Apply interesting values
    fn apply_interesting_values<R: Rand>(
        &self,
        data: &mut [u8],
        size: usize,
        rng: &mut R,
    ) -> MutationResult {
        if data.is_empty() {
            return MutationResult::Skipped;
        }

        let actual_size = size.min(data.len());
        interesting_value_mutate(&mut data[..actual_size], rng, actual_size);

        debug!("Applied interesting values (size {})", actual_size);
        MutationResult::Mutated
    }

    /// Apply boundary values
    fn apply_boundary_values<R: Rand>(
        &self,
        data: &mut [u8],
        values: &[Vec<u8>],
        rng: &mut R,
    ) -> MutationResult {
        if data.is_empty() || values.is_empty() {
            return MutationResult::Skipped;
        }

        let value = &values[rand_below(rng, values.len())];
        let copy_len = value.len().min(data.len());
        data[..copy_len].copy_from_slice(&value[..copy_len]);

        debug!("Applied boundary value ({} bytes)", copy_len);
        MutationResult::Mutated
    }

    /// Apply random bytes
    fn apply_random<R: Rand>(&self, data: &mut [u8], rng: &mut R) -> MutationResult {
        if data.is_empty() {
            return MutationResult::Skipped;
        }

        set_random_bytes(data, rng);
        debug!("Applied random bytes ({} bytes)", data.len());
        MutationResult::Mutated
    }

    /// Apply zero fill
    fn apply_zero(&self, data: &mut [u8]) -> MutationResult {
        if data.is_empty() {
            return MutationResult::Skipped;
        }

        data.fill(0);
        debug!("Applied zero fill");
        MutationResult::Mutated
    }

    /// Apply max (0xFF) fill
    fn apply_max(&self, data: &mut [u8]) -> MutationResult {
        if data.is_empty() {
            return MutationResult::Skipped;
        }

        data.fill(0xFF);
        debug!("Applied max fill");
        MutationResult::Mutated
    }
}

impl<S> Mutator<ErofsImageInput, S> for TargetedMutator
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut ErofsImageInput,
    ) -> Result<MutationResult, Error> {
        // Check if we've exceeded max mutations
        if self.mutations_applied >= self.max_mutations {
            trace!("Max mutations reached for targeted mutator");
            return Ok(MutationResult::Skipped);
        }

        // Locate the target
        let location = match self.locate_target(input) {
            Some(loc) => loc,
            None => {
                trace!("Could not locate target for mutation");
                return Ok(MutationResult::Skipped);
            }
        };

        trace!("Targeting location: offset={}, length={}", location.offset, location.length);

        // Apply the mutation
        let rng = state.rand_mut();
        let data = input.data_mut();
        let result = self.apply_strategy(data, location, rng);

        if result == MutationResult::Mutated {
            self.mutations_applied += 1;
            debug!(
                "Applied targeted mutation at offset {} (mutation {}/{})",
                location.offset, self.mutations_applied, self.max_mutations
            );
        }

        Ok(result)
    }

    fn post_exec(&mut self, _state: &mut S, _new_corpus_id: Option<CorpusId>) -> Result<(), Error> {
        // Reset mutation count for next input
        self.mutations_applied = 0;
        Ok(())
    }
}

/// Multi-target mutator that can target multiple regions
#[derive(Debug)]
pub struct MultiTargetMutator {
    /// List of targets to mutate
    targets: Vec<TargetedMutator>,
    /// Current target index
    current_target: usize,
}

impl Named for MultiTargetMutator {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("MultiTargetMutator");
        &NAME
    }
}

impl MultiTargetMutator {
    /// Create a new multi-target mutator
    pub fn new(targets: Vec<(MutationTarget, MutationStrategy)>) -> Self {
        let mutators: Vec<TargetedMutator> = targets
            .into_iter()
            .map(|(target, strategy)| TargetedMutator::new(target, strategy))
            .collect();

        Self {
            targets: mutators,
            current_target: 0,
        }
    }

    /// Add a target
    pub fn add_target(&mut self, target: MutationTarget, strategy: MutationStrategy) {
        self.targets.push(TargetedMutator::new(target, strategy));
    }
}

impl<S> Mutator<ErofsImageInput, S> for MultiTargetMutator
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut ErofsImageInput,
    ) -> Result<MutationResult, Error> {
        if self.targets.is_empty() {
            return Ok(MutationResult::Skipped);
        }

        // Try each target in sequence
        let start_idx = self.current_target;
        loop {
            let result = self.targets[self.current_target].mutate(state, input)?;
            self.current_target = (self.current_target + 1) % self.targets.len();

            if result == MutationResult::Mutated {
                return Ok(result);
            }

            if self.current_target == start_idx {
                // Tried all targets, none succeeded
                return Ok(MutationResult::Skipped);
            }
        }
    }

    fn post_exec(&mut self, state: &mut S, new_corpus_id: Option<CorpusId>) -> Result<(), Error> {
        for target in &mut self.targets {
            target.post_exec(state, new_corpus_id)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libafl_bolts::rands::StdRand;
    use erofs_input::FieldType;
    use erofs_input::SuperblockField;
    use erofs_format::EROFS_SUPER_OFFSET;

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

    impl HasRand for TestState {
        type Rand = StdRand;

        fn rand(&self) -> &Self::Rand {
            &self.rand
        }

        fn rand_mut(&mut self) -> &mut Self::Rand {
            &mut self.rand
        }
    }

    fn create_test_image() -> Vec<u8> {
        let mut data = vec![0u8; 2048];
        // Set magic
        let magic = erofs_format::EROFS_SUPER_MAGIC_V1.to_le_bytes();
        data[EROFS_SUPER_OFFSET..EROFS_SUPER_OFFSET + 4].copy_from_slice(&magic);
        data
    }

    #[test]
    fn test_targeted_mutator_absolute_range() {
        let mut mutator = TargetedMutator::new(
            MutationTarget::AbsoluteRange { start: 1024, length: 4 },
            MutationStrategy::BitFlip { count: 2 },
        );

        let mut state = TestState::new();
        let data = create_test_image();
        let mut input = ErofsImageInput::new(data);
        let original = input.data()[1024..1028].to_vec();

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        assert_eq!(result, MutationResult::Mutated);

        // Something should have changed
        assert_ne!(input.data()[1024..1028], original);
    }

    #[test]
    fn test_targeted_mutator_zero() {
        let mut mutator = TargetedMutator::new(
            MutationTarget::AbsoluteRange { start: 1024, length: 4 },
            MutationStrategy::Zero,
        );

        let mut state = TestState::new();
        let data = create_test_image();
        let mut input = ErofsImageInput::new(data);

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        assert_eq!(result, MutationResult::Mutated);

        // Should be all zeros
        assert_eq!(&input.data()[1024..1028], &[0, 0, 0, 0]);
    }

    #[test]
    fn test_targeted_mutator_superblock_field() {
        let mut mutator = TargetedMutator::new(
            MutationTarget::FieldRange {
                field: FieldType::Superblock(SuperblockField::Checksum),
                offset_before: 0,
                offset_after: 0,
            },
            MutationStrategy::Max,
        );

        let mut state = TestState::new();
        let data = create_test_image();
        let mut input = ErofsImageInput::new(data);

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        assert_eq!(result, MutationResult::Mutated);

        // Checksum is at offset 1024+4 = 1028
        assert_eq!(&input.data()[1028..1032], &[0xFF, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_multi_target_mutator() {
        let mut mutator = MultiTargetMutator::new(vec![
            (MutationTarget::AbsoluteRange { start: 1024, length: 4 }, MutationStrategy::Zero),
            (MutationTarget::AbsoluteRange { start: 1028, length: 4 }, MutationStrategy::Max),
        ]);

        let mut state = TestState::new();
        let data = create_test_image();
        let mut input = ErofsImageInput::new(data);

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        assert_eq!(result, MutationResult::Mutated);
    }
}
