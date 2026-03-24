//! Inode Mutator for EROFS Images
//!
//! Structure-aware mutations targeting EROFS inode structures.

use libafl::mutators::{Mutator, MutationResult};
use libafl::state::HasMetadata;
use libafl_bolts::rands::Rand;
use libafl_bolts::Error;
use tracing::{debug, trace};

use crate::{arithmetic_mutate, interesting_value_mutate, set_random_bytes};
use erofs_format::{inode_format, ErofsDataLayout, ErofsInodeCompact, ErofsInodeExtended, EROFS_SUPER_OFFSET};
use erofs_input::{ErofsImageInput, InodeField, InodeLayout};

/// Inode mutator for EROFS images
///
/// Performs targeted mutations on inode structures.
#[derive(Debug)]
pub struct ErofsInodeMutator {
    /// Minimum image size
    min_size: usize,
}

impl Default for ErofsInodeMutator {
    fn default() -> Self {
        Self::new()
    }
}

impl ErofsInodeMutator {
    /// Create a new inode mutator
    pub fn new() -> Self {
        Self {
            min_size: EROFS_SUPER_OFFSET + std::mem::size_of::<ErofsInodeCompact>(),
        }
    }

    /// Mutate an inode at the given offset
    fn mutate_inode_at<R: Rand>(
        &self,
        data: &mut [u8],
        offset: usize,
        layout: InodeLayout,
        rng: &mut R,
    ) -> MutationResult {
        let field = self.select_inode_field(rng);
        let inode_size = match layout {
            InodeLayout::Compact => std::mem::size_of::<ErofsInodeCompact>(),
            InodeLayout::Extended => std::mem::size_of::<ErofsInodeExtended>(),
        };

        if offset + inode_size > data.len() {
            return MutationResult::Skipped;
        }

        self.mutate_inode_field(data, offset, layout, field, rng)
    }

    /// Select a random inode field to mutate
    fn select_inode_field<R: Rand>(&self, rng: &mut R) -> InodeField {
        let fields = [
            InodeField::Format,
            InodeField::XattrIcount,
            InodeField::Mode,
            InodeField::Nb,
            InodeField::Size,
            InodeField::Mtime,
            InodeField::U,
            InodeField::Uid,
            InodeField::Gid,
        ];

        fields[rng.below(fields.len() as u64) as usize]
    }

    /// Mutate a specific inode field
    fn mutate_inode_field<R: Rand>(
        &self,
        data: &mut [u8],
        offset: usize,
        layout: InodeLayout,
        field: InodeField,
        rng: &mut R,
    ) -> MutationResult {
        match field {
            InodeField::Format => {
                // i_format (u16 at offset 0)
                let mutation_type = rng.below(4);
                match mutation_type {
                    0 => {
                        // Set to specific layout
                        let layouts = [
                            ErofsDataLayout::FlatPlain as u16,
                            ErofsDataLayout::FlatInline as u16,
                            ErofsDataLayout::CompressedFull as u16,
                            ErofsDataLayout::CompressedCompact as u16,
                            ErofsDataLayout::ChunkBased as u16,
                        ];
                        let layout_val = layouts[rng.below(layouts.len() as u64) as usize];
                        let format = (layout_val << inode_format::DATALAYOUT_BIT)
                            | (rng.below(2) as u16); // version bit
                        data[offset..offset + 2].copy_from_slice(&format.to_le_bytes());
                    }
                    1 => {
                        // Invalid layout
                        let invalid_layout = 5 + (rng.below(8) as u16);
                        data[offset..offset + 2].copy_from_slice(&(invalid_layout << 1).to_le_bytes());
                    }
                    2 => {
                        // Bit flip
                        data[offset] ^= 1 << (rng.below(8) as u8);
                    }
                    _ => {
                        // Random
                        set_random_bytes(&mut data[offset..offset + 2], rng);
                    }
                }
                debug!("Mutated inode format at offset {:#x}", offset);
                MutationResult::Mutated
            }
            InodeField::XattrIcount => {
                // i_xattr_icount (u16 at offset 2)
                let xattr_offset = offset + 2;
                let mutation_type = rng.below(3);
                match mutation_type {
                    0 => data[xattr_offset..xattr_offset + 2].fill(0),
                    1 => data[xattr_offset..xattr_offset + 2].fill(0xFF),
                    _ => set_random_bytes(&mut data[xattr_offset..xattr_offset + 2], rng),
                }
                MutationResult::Mutated
            }
            InodeField::Mode => {
                // i_mode (u16 at offset 4)
                let mode_offset = offset + 4;
                let mode_type = rng.below(5);
                match mode_type {
                    0 => {
                        // Set to directory
                        let mode = 0o40755u16;
                        data[mode_offset..mode_offset + 2].copy_from_slice(&mode.to_le_bytes());
                    }
                    1 => {
                        // Set to regular file
                        let mode = 0o100644u16;
                        data[mode_offset..mode_offset + 2].copy_from_slice(&mode.to_le_bytes());
                    }
                    2 => {
                        // Set to symlink
                        let mode = 0o120777u16;
                        data[mode_offset..mode_offset + 2].copy_from_slice(&mode.to_le_bytes());
                    }
                    3 => {
                        // Invalid mode
                        data[mode_offset..mode_offset + 2].fill(0xFF);
                    }
                    _ => {
                        // Random mode
                        set_random_bytes(&mut data[mode_offset..mode_offset + 2], rng);
                    }
                }
                MutationResult::Mutated
            }
            InodeField::Nb => {
                // i_nb union (u16 at offset 6)
                let nb_offset = offset + 6;
                self.mutate_16bit(&mut data[nb_offset..nb_offset + 2], rng)
            }
            InodeField::Size => {
                // i_size (u32 for compact, u64 for extended)
                let size_offset = offset + 8;
                match layout {
                    InodeLayout::Compact => {
                        self.mutate_32bit(&mut data[size_offset..size_offset + 4], rng)
                    }
                    InodeLayout::Extended => {
                        self.mutate_64bit(&mut data[size_offset..size_offset + 8], rng)
                    }
                }
            }
            InodeField::Mtime => {
                // i_mtime (u32 at offset 12 for compact, different for extended)
                let mtime_offset = match layout {
                    InodeLayout::Compact => offset + 12,
                    InodeLayout::Extended => offset + 32, // i_mtime in extended
                };
                self.mutate_32bit(&mut data[mtime_offset..mtime_offset + 4], rng)
            }
            InodeField::U => {
                // i_u union (u32 at offset 16)
                let u_offset = offset + 16;
                // This could be blocks, startblk, or rdev
                let mutation_type = rng.below(3);
                match mutation_type {
                    0 => {
                        // Zero (invalid block)
                        data[u_offset..u_offset + 4].fill(0);
                    }
                    1 => {
                        // Large value (out of bounds)
                        data[u_offset..u_offset + 4].fill(0xFF);
                    }
                    _ => {
                        self.mutate_32bit(&mut data[u_offset..u_offset + 4], rng)
                    }
                }
                MutationResult::Mutated
            }
            InodeField::Ino => {
                // i_ino (u32 at offset 20 for compact)
                let ino_offset = match layout {
                    InodeLayout::Compact => offset + 20,
                    InodeLayout::Extended => offset + 24,
                };
                self.mutate_32bit(&mut data[ino_offset..ino_offset + 4], rng)
            }
            InodeField::Uid => {
                // i_uid (u16 for compact, u32 for extended)
                let uid_offset = match layout {
                    InodeLayout::Compact => offset + 24,
                    InodeLayout::Extended => offset + 28,
                };
                match layout {
                    InodeLayout::Compact => {
                        self.mutate_16bit(&mut data[uid_offset..uid_offset + 2], rng)
                    }
                    InodeLayout::Extended => {
                        self.mutate_32bit(&mut data[uid_offset..uid_offset + 4], rng)
                    }
                }
            }
            InodeField::Gid => {
                // i_gid (u16 for compact, u32 for extended)
                let gid_offset = match layout {
                    InodeLayout::Compact => offset + 26,
                    InodeLayout::Extended => offset + 32,
                };
                match layout {
                    InodeLayout::Compact => {
                        self.mutate_16bit(&mut data[gid_offset..gid_offset + 2], rng)
                    }
                    InodeLayout::Extended => {
                        self.mutate_32bit(&mut data[gid_offset..gid_offset + 4], rng)
                    }
                }
            }
            InodeField::Nlink => {
                // i_nlink (only in extended layout at offset 44)
                if layout == InodeLayout::Extended {
                    let nlink_offset = offset + 48;
                    self.mutate_32bit(&mut data[nlink_offset..nlink_offset + 4], rng)
                } else {
                    MutationResult::Skipped
                }
            }
            InodeField::MtimeNsec => {
                // i_mtime_nsec (only in extended layout)
                if layout == InodeLayout::Extended {
                    let nsec_offset = offset + 40;
                    self.mutate_32bit(&mut data[nsec_offset..nsec_offset + 4], rng)
                } else {
                    MutationResult::Skipped
                }
            }
        }
    }

    /// Mutate a 16-bit field
    fn mutate_16bit<R: Rand>(&self, data: &mut [u8], rng: &mut R) -> MutationResult {
        let mutation_type = rng.below(4);
        match mutation_type {
            0 => data.fill(0),
            1 => data.fill(0xFF),
            2 => interesting_value_mutate(data, rng, 2),
            _ => arithmetic_mutate(data, rng),
        }
        MutationResult::Mutated
    }

    /// Mutate a 32-bit field
    fn mutate_32bit<R: Rand>(&self, data: &mut [u8], rng: &mut R) -> MutationResult {
        let mutation_type = rng.below(4);
        match mutation_type {
            0 => data.fill(0),
            1 => data.fill(0xFF),
            2 => interesting_value_mutate(data, rng, 4),
            _ => arithmetic_mutate(data, rng),
        }
        MutationResult::Mutated
    }

    /// Mutate a 64-bit field
    fn mutate_64bit<R: Rand>(&self, data: &mut [u8], rng: &mut R) -> MutationResult {
        let mutation_type = rng.below(4);
        match mutation_type {
            0 => data.fill(0),
            1 => data.fill(0xFF),
            2 => interesting_value_mutate(data, rng, 8),
            _ => arithmetic_mutate(data, rng),
        }
        MutationResult::Mutated
    }
}

impl<S> Mutator<ErofsImageInput, S> for ErofsInodeMutator
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
            trace!("Image too small for inode mutation");
            return Ok(MutationResult::Skipped);
        }

        // Find inode locations from injection points
        let inode_offsets: Vec<(usize, InodeLayout)> = input
            .injection_points()
            .iter()
            .filter_map(|p| {
                if let erofs_input::InjectionPoint::Inode { offset, layout, .. } = p {
                    Some((*offset, *layout))
                } else {
                    None
                }
            })
            .collect();

        if inode_offsets.is_empty() {
            // No inode injection points, try to find inodes manually
            // For now, just try to mutate at the metadata block
            trace!("No inode injection points found, trying metadata block");
            return Ok(MutationResult::Skipped);
        }

        // Select a random inode to mutate
        let (offset, layout) = inode_offsets[rng.below(inode_offsets.len() as u64) as usize];
        trace!("Mutating inode at offset {:#x}", offset);

        let result = self.mutate_inode_at(data, offset, layout, rng);

        if result == MutationResult::Mutated {
            debug!("Successfully mutated inode at offset {:#x}", offset);
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
                rand: StdRand::with_seed(123),
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
    fn test_inode_mutator_basic() {
        let mut mutator = ErofsInodeMutator::new();
        let mut state = TestState::new();

        // Create image with an inode
        let mut data = vec![0u8; 2048];
        // Add an inode at offset 4096 (after super block)
        let mut input = ErofsImageInput::new(data);
        input.add_raw_injection_point(4096, 32, "test inode");

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        // Result depends on whether we have inode injection points
        assert_ne!(result, MutationResult::Skipped);
    }
}
