//! Xattr (Extended Attribute) Mutator for EROFS Images
//!
//! Structure-aware mutations targeting EROFS extended attribute structures.

use libafl::corpus::CorpusId;
use libafl::mutators::{Mutator, MutationResult};
use libafl::state::HasRand;
use libafl_bolts::Named;
use libafl_bolts::rands::Rand;
use libafl_bolts::Error;
use std::borrow::Cow;
use tracing::{debug, trace};

use crate::{rand_below, arithmetic_mutate, interesting_value_mutate, set_random_bytes};
use erofs_input::ErofsImageInput;

/// Xattr mutator for EROFS images
///
/// Performs targeted mutations on extended attribute structures.
#[derive(Debug, Default)]
pub struct ErofsXattrMutator {
    /// Minimum xattr entry size
    min_xattr_size: usize,
}

impl Named for ErofsXattrMutator {
    fn name(&self) -> &Cow<'static, str> {
        static NAME: Cow<'static, str> = Cow::Borrowed("ErofsXattrMutator");
        &NAME
    }
}

impl ErofsXattrMutator {
    /// Create a new xattr mutator
    pub fn new() -> Self {
        Self {
            min_xattr_size: 4, // Minimum ErofsXattrEntry size
        }
    }

    /// Mutate an xattr at the given offset
    fn mutate_xattr_at<R: Rand>(
        &self,
        data: &mut [u8],
        offset: usize,
        rng: &mut R,
    ) -> MutationResult {
        if offset + self.min_xattr_size > data.len() {
            return MutationResult::Skipped;
        }

        let field = self.select_xattr_field(rng);
        self.mutate_xattr_field(data, offset, field, rng)
    }

    /// Select a random xattr field to mutate
    fn select_xattr_field<R: Rand>(&self, rng: &mut R) -> XattrField {
        let fields = [
            XattrField::NameLen,
            XattrField::NameIndex,
            XattrField::ValueSize,
            XattrField::NameData,
            XattrField::ValueData,
        ];
        fields[rand_below(rng, fields.len())]
    }

    /// Mutate a specific xattr field
    fn mutate_xattr_field<R: Rand>(
        &self,
        data: &mut [u8],
        offset: usize,
        field: XattrField,
        rng: &mut R,
    ) -> MutationResult {
        match field {
            XattrField::NameLen => {
                // e_name_len (u8 at offset 0)
                let namelen_offset = offset;
                let mutation_type = rand_below(rng, 4);

                match mutation_type {
                    0 => {
                        // Zero length (invalid)
                        data[namelen_offset] = 0;
                    }
                    1 => {
                        // Very long name
                        data[namelen_offset] = 255;
                    }
                    2 => {
                        // Random length
                        data[namelen_offset] = rand_below(rng, 256) as u8;
                    }
                    _ => {
                        // Bit flip
                        data[namelen_offset] ^= 1 << (rand_below(rng, 8) as u8);
                    }
                }

                debug!("Mutated xattr name_len at offset {:#x}", offset);
                MutationResult::Mutated
            }
            XattrField::NameIndex => {
                // e_name_index (u8 at offset 1)
                let index_offset = offset + 1;
                let mutation_type = rand_below(rng, 5);

                match mutation_type {
                    0 => {
                        // User namespace
                        data[index_offset] = 1;
                    }
                    1 => {
                        // Security namespace
                        data[index_offset] = 6;
                    }
                    2 => {
                        // Long prefix flag
                        data[index_offset] = 0x80 | (rand_below(rng, 8) as u8);
                    }
                    3 => {
                        // Invalid index
                        data[index_offset] = 100;
                    }
                    _ => {
                        // Random
                        data[index_offset] = rand_below(rng, 256) as u8;
                    }
                }

                debug!("Mutated xattr name_index at offset {:#x}", offset);
                MutationResult::Mutated
            }
            XattrField::ValueSize => {
                // e_value_size (u16 at offset 2)
                let valuesize_offset = offset + 2;
                let mutation_type = rand_below(rng, 4);

                match mutation_type {
                    0 => {
                        // Zero size
                        data[valuesize_offset..valuesize_offset + 2].fill(0);
                    }
                    1 => {
                        // Large size
                        data[valuesize_offset..valuesize_offset + 2].fill(0xFF);
                    }
                    2 => {
                        // Interesting size
                        interesting_value_mutate(&mut data[valuesize_offset..valuesize_offset + 2], rng, 2);
                    }
                    _ => {
                        arithmetic_mutate(&mut data[valuesize_offset..valuesize_offset + 2], rng);
                    }
                }

                debug!("Mutated xattr value_size at offset {:#x}", offset);
                MutationResult::Mutated
            }
            XattrField::NameData => {
                // Name data starts at offset 4
                let name_offset = offset + 4;

                // Try to get name length from entry
                let name_len = if offset + 1 < data.len() {
                    data[offset] as usize
                } else {
                    8 // Default
                };

                if name_offset + name_len > data.len() {
                    return MutationResult::Skipped;
                }

                if name_len == 0 {
                    // Mutate the reserved byte instead
                    if name_offset < data.len() {
                        data[name_offset] = rand_below(rng, 256) as u8;
                        return MutationResult::Mutated;
                    }
                    return MutationResult::Skipped;
                }

                let mutation_type = rand_below(rng, 4);

                match mutation_type {
                    0 => {
                        // Clear name
                        data[name_offset..name_offset + name_len].fill(0);
                    }
                    1 => {
                        // Random bytes
                        set_random_bytes(&mut data[name_offset..name_offset + name_len], rng);
                    }
                    2 => {
                        // Add null bytes in the middle
                        let null_pos = name_offset + rand_below(rng, name_len);
                        data[null_pos] = 0;
                    }
                    _ => {
                        // Bit flip
                        let byte_pos = name_offset + rand_below(rng, name_len);
                        data[byte_pos] ^= 1 << (rand_below(rng, 8) as u8);
                    }
                }

                debug!("Mutated xattr name_data at offset {:#x}", offset);
                MutationResult::Mutated
            }
            XattrField::ValueData => {
                // Value data starts at offset 4 + name_len
                let name_len = if offset + 1 < data.len() {
                    data[offset] as usize
                } else {
                    0
                };

                let value_size = if offset + 4 <= data.len() {
                    u16::from_le_bytes([data[offset + 2], data[offset + 3]]) as usize
                } else {
                    0
                };

                let value_offset = offset + 4 + name_len;

                if value_offset + value_size > data.len() || value_size == 0 {
                    return MutationResult::Skipped;
                }

                let mutation_type = rand_below(rng, 4);

                match mutation_type {
                    0 => {
                        // Clear value
                        data[value_offset..value_offset + value_size].fill(0);
                    }
                    1 => {
                        // Random bytes
                        set_random_bytes(&mut data[value_offset..value_offset + value_size], rng);
                    }
                    2 => {
                        // Partial mutation
                        let mutate_len = std::cmp::min(value_size, 16);
                        set_random_bytes(
                            &mut data[value_offset..value_offset + mutate_len],
                            rng,
                        );
                    }
                    _ => {
                        // Single byte mutation
                        let byte_pos = value_offset + rand_below(rng, value_size);
                        data[byte_pos] ^= 1 << (rand_below(rng, 8) as u8);
                    }
                }

                debug!("Mutated xattr value_data at offset {:#x}", offset);
                MutationResult::Mutated
            }
        }
    }

    /// Mutate an xattr ibody header
    fn mutate_xattr_header<R: Rand>(
        &self,
        data: &mut [u8],
        offset: usize,
        rng: &mut R,
    ) -> MutationResult {
        // ErofsXattrIbodyHeader is 12 bytes
        if offset + 12 > data.len() {
            return MutationResult::Skipped;
        }

        let field = rand_below(rng, 3);

        match field {
            0 => {
                // Mutate h_name_filter (u32 at offset 0)
                let filter_offset = offset;
                let mutation_type = rand_below(rng, 3);
                match mutation_type {
                    0 => data[filter_offset..filter_offset + 4].fill(0),
                    1 => data[filter_offset..filter_offset + 4].fill(0xFF),
                    _ => set_random_bytes(&mut data[filter_offset..filter_offset + 4], rng),
                }
                debug!("Mutated xattr header name_filter");
            }
            1 => {
                // Mutate h_shared_count (u8 at offset 4)
                let count_offset = offset + 4;
                let mutation_type = rand_below(rng, 3);
                match mutation_type {
                    0 => data[count_offset] = 0,
                    1 => data[count_offset] = 255,
                    _ => data[count_offset] = rand_below(rng, 256) as u8,
                }
                debug!("Mutated xattr header shared_count");
            }
            _ => {
                // Mutate reserved bytes
                let reserved_offset = offset + 5;
                set_random_bytes(&mut data[reserved_offset..reserved_offset + 7], rng);
                debug!("Mutated xattr header reserved");
            }
        }

        MutationResult::Mutated
    }
}

/// Xattr field to mutate
#[derive(Debug, Clone, Copy)]
enum XattrField {
    /// Name length
    NameLen,
    /// Name index
    NameIndex,
    /// Value size
    ValueSize,
    /// Name data
    NameData,
    /// Value data
    ValueData,
}

impl<S> Mutator<ErofsImageInput, S> for ErofsXattrMutator
where
    S: HasRand,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut ErofsImageInput,
    ) -> Result<MutationResult, Error> {
        let rng = state.rand_mut();

        // Find xattr locations from injection points first
        let xattr_offsets: Vec<usize> = input
            .injection_points()
            .iter()
            .filter_map(|p| {
                if let erofs_input::InjectionPoint::Xattr { offset, .. } = p {
                    Some(*offset)
                } else {
                    None
                }
            })
            .collect();

        if xattr_offsets.is_empty() {
            trace!("No xattr injection points found");
            return Ok(MutationResult::Skipped);
        }

        let data = input.data_mut();

        if data.len() < self.min_xattr_size {
            trace!("Image too small for xattr mutation");
            return Ok(MutationResult::Skipped);
        }

        // Select a random xattr to mutate
        let offset = xattr_offsets[rand_below(rng, xattr_offsets.len())];
        trace!("Mutating xattr at offset {:#x}", offset);

        let result = self.mutate_xattr_at(data, offset, rng);

        if result == MutationResult::Mutated {
            debug!("Successfully mutated xattr at offset {:#x}", offset);
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
                rand: StdRand::with_seed(789),
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
    fn test_xattr_mutator() {
        let mut mutator = ErofsXattrMutator::new();
        let mut state = TestState::new();

        let mut data = vec![0u8; 2048];
        // Add an xattr entry at offset 200
        // name_len=4, name_index=1, value_size=8
        data[200..204].copy_from_slice(&[4, 1, 8, 0]);
        // Name: "test"
        data[204..208].copy_from_slice(b"test");
        // Value: 8 bytes
        data[208..216].copy_from_slice(b"testval!");

        let mut input = ErofsImageInput::new(data);
        input.add_raw_injection_point(200, 16, "test xattr");

        let result = mutator.mutate(&mut state, &mut input).unwrap();
        assert_ne!(result, MutationResult::Skipped);
    }

    #[test]
    fn test_xattr_header_mutation() {
        let mut mutator = ErofsXattrMutator::new();
        let mut state = TestState::new();

        let mut data = vec![0u8; 2048];
        // Add xattr header at offset 300
        data[300..312].copy_from_slice(&[
            0xFF, 0xFF, 0xFF, 0xFF, // h_name_filter
            2, // h_shared_count
            0, 0, 0, 0, 0, 0, 0, // reserved
        ]);

        let result = mutator.mutate_xattr_header(&mut data, 300, &mut state.rand);
        assert_eq!(result, MutationResult::Mutated);
    }
}
