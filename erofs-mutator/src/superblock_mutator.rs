//! Superblock Mutator for EROFS Images
//!
//! Structure-aware mutations targeting the EROFS super block.

use libafl::mutators::{Mutator, MutationResult};
use libafl::state::HasMetadata;
use libafl_bolts::rands::Rand;
use libafl_bolts::Error;
use tracing::{debug, trace};

use crate::{arithmetic_mutate, interesting_value_mutate, set_random_bytes};
use erofs_format::{ErofsSuperBlock, EROFS_SUPER_OFFSET};
use erofs_input::{ErofsImageInput, InjectionPoint, SuperblockField};

/// Superblock mutator for EROFS images
///
/// Performs targeted mutations on super block fields.
#[derive(Debug)]
pub struct ErofsSuperblockMutator {
    /// Minimum image size to have a valid super block
    min_size: usize,
}

impl Default for ErofsSuperblockMutator {
    fn default() -> Self {
        Self::new()
    }
}

impl ErofsSuperblockMutator {
    /// Create a new superblock mutator
    pub fn new() -> Self {
        Self {
            min_size: EROFS_SUPER_OFFSET + std::mem::size_of::<ErofsSuperBlock>(),
        }
    }

    /// Mutate a specific superblock field
    fn mutate_field<R: Rand>(
        &self,
        data: &mut [u8],
        field: SuperblockField,
        rng: &mut R,
    ) -> MutationResult {
        let sb_offset = EROFS_SUPER_OFFSET;

        match field {
            SuperblockField::Magic => {
                // Rarely mutate magic - usually want to keep it valid
                if rng.below(20) == 0 {
                    // 5% chance
                    let offset = sb_offset;
                    interesting_value_mutate(&mut data[offset..offset + 4], rng, 4);
                    MutationResult::Mutated
                } else {
                    MutationResult::Skipped
                }
            }
            SuperblockField::Checksum => {
                // Corrupt checksum
                let offset = sb_offset + 4;
                let mutation_type = rng.below(4);
                match mutation_type {
                    0 => {
                        // Set to zero
                        data[offset..offset + 4].copy_from_slice(&[0, 0, 0, 0]);
                    }
                    1 => {
                        // Set to max
                        data[offset..offset + 4].copy_from_slice(&[0xFF; 4]);
                    }
                    2 => {
                        // Flip bits
                        data[offset] ^= 0xFF;
                    }
                    _ => {
                        // Random
                        set_random_bytes(&mut data[offset..offset + 4], rng);
                    }
                }
                debug!("Mutated checksum field");
                MutationResult::Mutated
            }
            SuperblockField::FeatureCompat => {
                let offset = sb_offset + 8;
                self.mutate_feature_flags(&mut data[offset..offset + 4], rng, true)
            }
            SuperblockField::Blkszbits => {
                let offset = sb_offset + 12;
                // Valid values are 9-16 (512 to 65536 bytes)
                let mutation_type = rng.below(4);
                match mutation_type {
                    0 => {
                        // Set to invalid small value
                        data[offset] = rng.below(9) as u8;
                    }
                    1 => {
                        // Set to invalid large value
                        data[offset] = 17 + (rng.below(16) as u8);
                    }
                    2 => {
                        // Valid but unusual
                        data[offset] = 9 + (rng.below(8) as u8);
                    }
                    _ => {
                        // Bit flip
                        data[offset] ^= 1 << (rng.below(8) as u8);
                    }
                }
                debug!("Mutated blkszbits to {}", data[offset]);
                MutationResult::Mutated
            }
            SuperblockField::SbExtslots => {
                let offset = sb_offset + 13;
                data[offset] = rng.below(256) as u8;
                MutationResult::Mutated
            }
            SuperblockField::RootNid => {
                let offset = sb_offset + 14;
                // Mutate root nid (16-bit)
                let mutation_type = rng.below(4);
                match mutation_type {
                    0 => {
                        // Set to zero
                        data[offset..offset + 2].copy_from_slice(&[0, 0]);
                    }
                    1 => {
                        // Set to large value
                        data[offset..offset + 2].copy_from_slice(&[0xFF; 2]);
                    }
                    2 => {
                        // Random valid-looking
                        set_random_bytes(&mut data[offset..offset + 2], rng);
                    }
                    _ => {
                        arithmetic_mutate(&mut data[offset..offset + 2], rng);
                    }
                }
                MutationResult::Mutated
            }
            SuperblockField::Inos => {
                let offset = sb_offset + 16;
                self.mutate_64bit_field(&mut data[offset..offset + 8], rng)
            }
            SuperblockField::Epoch => {
                let offset = sb_offset + 24;
                self.mutate_64bit_field(&mut data[offset..offset + 8], rng)
            }
            SuperblockField::FixedNsec => {
                let offset = sb_offset + 32;
                self.mutate_32bit_field(&mut data[offset..offset + 4], rng)
            }
            SuperblockField::Blocks => {
                let offset = sb_offset + 36;
                self.mutate_32bit_field(&mut data[offset..offset + 4], rng)
            }
            SuperblockField::MetaBlkaddr => {
                let offset = sb_offset + 40;
                self.mutate_32bit_field(&mut data[offset..offset + 4], rng)
            }
            SuperblockField::XattrBlkaddr => {
                let offset = sb_offset + 44;
                self.mutate_32bit_field(&mut data[offset..offset + 4], rng)
            }
            SuperblockField::Uuid => {
                let offset = sb_offset + 48;
                // Mutate UUID bytes
                let uuid_offset = rng.below(16) as usize;
                data[offset + uuid_offset] ^= rng.below(256) as u8;
                MutationResult::Mutated
            }
            SuperblockField::VolumeName => {
                let offset = sb_offset + 64;
                // Mutate volume name (16 bytes)
                let name_offset = rng.below(16) as usize;
                if data[offset + name_offset] != 0 {
                    // Don't mutate null terminator
                    data[offset + name_offset] = rng.below(128) as u8; // ASCII range
                }
                MutationResult::Mutated
            }
            SuperblockField::FeatureIncompat => {
                let offset = sb_offset + 80;
                self.mutate_feature_flags(&mut data[offset..offset + 4], rng, false)
            }
            SuperblockField::ExtraDevices => {
                let offset = sb_offset + 84;
                self.mutate_16bit_field(&mut data[offset..offset + 2], rng)
            }
            SuperblockField::DevtSlotoff => {
                let offset = sb_offset + 86;
                self.mutate_16bit_field(&mut data[offset..offset + 2], rng)
            }
            SuperblockField::Dirblkbits => {
                let offset = sb_offset + 88;
                // Similar to blkszbits
                let mutation_type = rng.below(3);
                match mutation_type {
                    0 => data[offset] = 9 + (rng.below(8) as u8),
                    1 => data[offset] = rng.below(9) as u8,
                    _ => data[offset] ^= 1 << (rng.below(8) as u8),
                }
                MutationResult::Mutated
            }
            SuperblockField::PackedNid => {
                let offset = sb_offset + 96;
                self.mutate_64bit_field(&mut data[offset..offset + 8], rng)
            }
            SuperblockField::MetaboxNid => {
                let offset = sb_offset + 112;
                self.mutate_64bit_field(&mut data[offset..offset + 8], rng)
            }
        }
    }

    /// Mutate feature flags
    fn mutate_feature_flags<R: Rand>(&self, data: &mut [u8], rng: &mut R, is_compat: bool) -> MutationResult {
        let mutation_type = rng.below(5);

        match mutation_type {
            0 => {
                // Set random flag
                let bit = rng.below(32) as usize;
                let byte_idx = bit / 8;
                let bit_idx = bit % 8;
                data[byte_idx] ^= 1 << bit_idx;
            }
            1 => {
                // Clear all flags
                data.fill(0);
            }
            2 => {
                // Set all flags
                data.fill(0xFF);
            }
            3 => {
                // Random flags
                set_random_bytes(data, rng);
            }
            _ => {
                // Interesting flag combinations
                if is_compat {
                    // Known compat flags
                    let flags: [u32; 4] = [
                        0x00000001, // SB_CHKSUM
                        0x00000003, // SB_CHKSUM | MTIME
                        0x00000007, // SB_CHKSUM | MTIME | XATTR_FILTER
                        0xFFFFFFFF, // All flags
                    ];
                    let flag = flags[rng.below(4) as usize];
                    data.copy_from_slice(&flag.to_le_bytes());
                } else {
                    // Known incompat flags
                    let flags: [u32; 4] = [
                        0x00000001, // LZ4_0PADDING
                        0x00000007, // Various compression
                        0x000000FF, // Many features
                        0xFFFFFFFF, // All flags (invalid)
                    ];
                    let flag = flags[rng.below(4) as usize];
                    data.copy_from_slice(&flag.to_le_bytes());
                }
            }
        }

        MutationResult::Mutated
    }

    /// Mutate a 16-bit field
    fn mutate_16bit_field<R: Rand>(&self, data: &mut [u8], rng: &mut R) -> MutationResult {
        let mutation_type = rng.below(4);

        match mutation_type {
            0 => {
                data.fill(0);
            }
            1 => {
                data.fill(0xFF);
            }
            2 => {
                interesting_value_mutate(data, rng, 2);
            }
            _ => {
                arithmetic_mutate(data, rng);
            }
        }

        MutationResult::Mutated
    }

    /// Mutate a 32-bit field
    fn mutate_32bit_field<R: Rand>(&self, data: &mut [u8], rng: &mut R) -> MutationResult {
        let mutation_type = rng.below(4);

        match mutation_type {
            0 => {
                data.fill(0);
            }
            1 => {
                data.fill(0xFF);
            }
            2 => {
                interesting_value_mutate(data, rng, 4);
            }
            _ => {
                arithmetic_mutate(data, rng);
            }
        }

        MutationResult::Mutated
    }

    /// Mutate a 64-bit field
    fn mutate_64bit_field<R: Rand>(&self, data: &mut [u8], rng: &mut R) -> MutationResult {
        let mutation_type = rng.below(4);

        match mutation_type {
            0 => {
                data.fill(0);
            }
            1 => {
                data.fill(0xFF);
            }
            2 => {
                interesting_value_mutate(data, rng, 8);
            }
            _ => {
                arithmetic_mutate(data, rng);
            }
        }

        MutationResult::Mutated
    }

    /// Select a random superblock field to mutate
    fn select_random_field<R: Rand>(&self, rng: &mut R) -> SuperblockField {
        let fields = [
            SuperblockField::Checksum,
            SuperblockField::FeatureCompat,
            SuperblockField::Blkszbits,
            SuperblockField::RootNid,
            SuperblockField::Inos,
            SuperblockField::Blocks,
            SuperblockField::MetaBlkaddr,
            SuperblockField::XattrBlkaddr,
            SuperblockField::FeatureIncompat,
            SuperblockField::ExtraDevices,
            SuperblockField::PackedNid,
        ];

        fields[rng.below(fields.len() as u64) as usize].clone()
    }
}

impl<S> Mutator<ErofsImageInput, S> for ErofsSuperblockMutator
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

        // Check minimum size
        if data.len() < self.min_size {
            trace!("Image too small for superblock mutation");
            return Ok(MutationResult::Skipped);
        }

        // Select a field to mutate
        let field = self.select_random_field(rng);
        trace!("Mutating superblock field: {:?}", field);

        // Apply mutation
        let result = self.mutate_field(data, field.clone(), rng);

        if result == MutationResult::Mutated {
            debug!("Successfully mutated superblock field: {:?}", field);
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
    fn test_superblock_mutator() {
        let mut mutator = ErofsSuperblockMutator::new();
        let mut state = TestState::new();

        // Create a minimal valid image
        let mut data = vec![0u8; 2048];
        let magic = erofs_format::EROFS_SUPER_MAGIC_V1.to_le_bytes();
        data[1024..1028].copy_from_slice(&magic);

        let mut input = ErofsImageInput::new(data);
        let original = input.data().to_vec();

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        assert_eq!(result, MutationResult::Mutated);

        // Something should have changed
        assert_ne!(input.data(), original);
    }

    #[test]
    fn test_small_input() {
        let mut mutator = ErofsSuperblockMutator::new();
        let mut state = TestState::new();
        let mut input = ErofsImageInput::new(vec![0u8; 100]);

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        assert_eq!(result, MutationResult::Skipped);
    }
}
