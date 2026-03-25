//! Directory Entry Mutator for EROFS Images
//!
//! Structure-aware mutations targeting EROFS directory entry structures.

use libafl::corpus::CorpusId;
use libafl::mutators::{Mutator, MutationResult};
use libafl::state::HasRand;
use libafl_bolts::Named;
use libafl_bolts::rands::Rand;
use libafl_bolts::Error;
use std::borrow::Cow;
use tracing::{debug, trace};

use crate::{rand_below, arithmetic_mutate, set_random_bytes};
use erofs_format::ErofsDirent;
use erofs_input::ErofsImageInput;

/// Directory entry mutator for EROFS images
///
/// Performs targeted mutations on directory entry structures.
#[derive(Debug, Default)]
pub struct ErofsDirectoryMutator {
    /// Minimum dirent size
    min_dirent_size: usize,
}

impl Named for ErofsDirectoryMutator {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ErofsDirectoryMutator");
        &NAME
    }
}

impl ErofsDirectoryMutator {
    /// Create a new directory mutator
    pub fn new() -> Self {
        Self {
            min_dirent_size: std::mem::size_of::<ErofsDirent>(),
        }
    }

    /// Mutate a directory entry at the given offset
    fn mutate_dirent_at<R: Rand>(
        &self,
        data: &mut [u8],
        offset: usize,
        rng: &mut R,
    ) -> MutationResult {
        if offset + self.min_dirent_size > data.len() {
            return MutationResult::Skipped;
        }

        let field = self.select_dirent_field(rng);
        self.mutate_dirent_field(data, offset, field, rng)
    }

    /// Select a random dirent field to mutate
    fn select_dirent_field<R: Rand>(&self, rng: &mut R) -> DirentField {
        let fields = [DirentField::Nid, DirentField::NameOff, DirentField::FileType];
        fields[rand_below(rng, fields.len())]
    }

    /// Mutate a specific dirent field
    fn mutate_dirent_field<R: Rand>(
        &self,
        data: &mut [u8],
        offset: usize,
        field: DirentField,
        rng: &mut R,
    ) -> MutationResult {
        match field {
            DirentField::Nid => {
                // nid (u64 at offset 0)
                let nid_offset = offset;
                let mutation_type = rand_below(rng, 5);

                match mutation_type {
                    0 => {
                        // Zero NID
                        data[nid_offset..nid_offset + 8].fill(0);
                    }
                    1 => {
                        // Max NID
                        data[nid_offset..nid_offset + 8].fill(0xFF);
                    }
                    2 => {
                        // Set metabox bit
                        let metabox_bit = 63;
                        data[nid_offset + 7] |= 0x80; // Set bit 63
                    }
                    3 => {
                        // Random NID
                        set_random_bytes(&mut data[nid_offset..nid_offset + 8], rng);
                    }
                    _ => {
                        // Arithmetic mutation
                        arithmetic_mutate(&mut data[nid_offset..nid_offset + 8], rng);
                    }
                }

                debug!("Mutated dirent nid at offset {:#x}", offset);
                MutationResult::Mutated
            }
            DirentField::NameOff => {
                // nameoff (u16 at offset 8)
                let nameoff_offset = offset + 8;
                let mutation_type = rand_below(rng, 4);

                match mutation_type {
                    0 => {
                        // Zero offset (usually invalid)
                        data[nameoff_offset..nameoff_offset + 2].fill(0);
                    }
                    1 => {
                        // Large offset (potentially out of bounds)
                        data[nameoff_offset..nameoff_offset + 2].fill(0xFF);
                    }
                    2 => {
                        // Random offset
                        set_random_bytes(&mut data[nameoff_offset..nameoff_offset + 2], rng);
                    }
                    _ => {
                        arithmetic_mutate(&mut data[nameoff_offset..nameoff_offset + 2], rng);
                    }
                }

                debug!("Mutated dirent nameoff at offset {:#x}", offset);
                MutationResult::Mutated
            }
            DirentField::FileType => {
                // file_type (u8 at offset 10)
                let ftype_offset = offset + 10;
                let mutation_type = rand_below(rng, 5);

                match mutation_type {
                    0 => {
                        // Unknown type
                        data[ftype_offset] = 0;
                    }
                    1 => {
                        // Regular file
                        data[ftype_offset] = 1;
                    }
                    2 => {
                        // Directory
                        data[ftype_offset] = 2;
                    }
                    3 => {
                        // Symlink
                        data[ftype_offset] = 7;
                    }
                    _ => {
                        // Invalid type
                        data[ftype_offset] = 100; // Out of range
                    }
                }

                debug!("Mutated dirent file_type at offset {:#x}", offset);
                MutationResult::Mutated
            }
        }
    }

    /// Mutate the name data after a dirent
    fn mutate_dirent_name<R: Rand>(
        &self,
        data: &mut [u8],
        name_offset: usize,
        max_len: usize,
        rng: &mut R,
    ) -> MutationResult {
        if name_offset >= data.len() {
            return MutationResult::Skipped;
        }

        let actual_len = std::cmp::min(max_len, data.len() - name_offset);
        if actual_len == 0 {
            return MutationResult::Skipped;
        }

        let mutation_type = rand_below(rng, 4);

        match mutation_type {
            0 => {
                // Truncate name
                data[name_offset] = 0;
            }
            1 => {
                // Extend with garbage
                let extend_len = std::cmp::min(actual_len, 16);
                set_random_bytes(&mut data[name_offset..name_offset + extend_len], rng);
            }
            2 => {
                // Invalid characters
                let char_offset = name_offset + rand_below(rng, actual_len);
                data[char_offset] = 0xFF; // Invalid UTF-8
            }
            _ => {
                // Bit flip in name
                let byte_offset = name_offset + rand_below(rng, actual_len);
                data[byte_offset] ^= 1 << (rand_below(rng, 8) as u8);
            }
        }

        MutationResult::Mutated
    }
}

/// Directory entry field to mutate
#[derive(Debug, Clone, Copy)]
enum DirentField {
    /// Node ID
    Nid,
    /// Name offset
    NameOff,
    /// File type
    FileType,
}

impl<S> Mutator<ErofsImageInput, S> for ErofsDirectoryMutator
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut ErofsImageInput,
    ) -> Result<MutationResult, Error> {
        let rng = state.rand_mut();

        // Find directory entry locations from injection points first
        let dirent_offsets: Vec<usize> = input
            .injection_points()
            .iter()
            .filter_map(|p| {
                if let erofs_input::InjectionPoint::Dirent { offset, .. } = p {
                    Some(*offset)
                } else {
                    None
                }
            })
            .collect();

        if dirent_offsets.is_empty() {
            // No dirent injection points, try a random offset
            trace!("No dirent injection points found");
            // Could scan for potential dirent patterns here
            return Ok(MutationResult::Skipped);
        }

        let data = input.data_mut();

        if data.len() < self.min_dirent_size {
            trace!("Image too small for directory mutation");
            return Ok(MutationResult::Skipped);
        }

        // Select a random dirent to mutate
        let offset = dirent_offsets[rand_below(rng, dirent_offsets.len())];
        trace!("Mutating dirent at offset {:#x}", offset);

        let result = self.mutate_dirent_at(data, offset, rng);

        if result == MutationResult::Mutated {
            debug!("Successfully mutated dirent at offset {:#x}", offset);
        }

        Ok(result)
    }

    fn post_exec(&mut self, _state: &mut S, _new_corpus_id: Option<CorpusId>) -> Result<(), Error> {
        Ok(())
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
                rand: StdRand::with_seed(456),
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

    #[test]
    fn test_directory_mutator() {
        let mut mutator = ErofsDirectoryMutator::new();
        let mut state = TestState::new();

        let mut data = vec![0u8; 2048];
        // Add a dirent at offset 100
        data[100..112].copy_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0]);

        let mut input = ErofsImageInput::new(data);
        input.add_raw_injection_point(100, 12, "test dirent");

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        assert_ne!(result, MutationResult::Skipped);
    }
}
