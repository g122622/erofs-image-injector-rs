//! EROFS Image Mutators for LibAFL
//!
//! This crate provides structure-aware mutators for EROFS filesystem images.
//! These mutators understand the EROFS on-disk format and can inject errors
//! into specific fields while maintaining structural validity.

#![deny(missing_docs)]

mod bitflip_mutator;
mod superblock_mutator;
mod inode_mutator;
mod directory_mutator;
mod xattr_mutator;
mod field_locator;
mod targeted_mutator;

pub use bitflip_mutator::*;
pub use superblock_mutator::*;
pub use inode_mutator::*;
pub use directory_mutator::*;
pub use xattr_mutator::*;
pub use field_locator::*;
pub use targeted_mutator::*;

use libafl_bolts::rands::Rand;
use std::num::NonZeroUsize;

/// Re-export common types
pub mod prelude {
    pub use crate::{
        ErofsBitflipMutator, ErofsSuperblockMutator, ErofsInodeMutator,
        ErofsDirectoryMutator, ErofsXattrMutator,
    };
    pub use libafl::mutators::MutationResult;
}

/// Helper function to call rng.below with a plain usize
/// LibAFL 0.15 requires NonZeroUsize for below()
#[inline]
pub fn rand_below<R: Rand>(rng: &mut R, upper_bound: usize) -> usize {
    if upper_bound == 0 {
        return 0;
    }
    rng.below(NonZeroUsize::new(upper_bound).unwrap())
}

/// Helper function to get a random value in range
pub fn rand_in_range<R: Rand>(rng: &mut R, min: u64, max: u64) -> u64 {
    if max <= min {
        return min;
    }
    min + rand_below(rng, (max - min) as usize) as u64
}

/// Helper function to flip a random bit in a byte slice
pub fn flip_random_bit<R: Rand>(data: &mut [u8], rng: &mut R) {
    if data.is_empty() {
        return;
    }
    let byte_idx = rand_below(rng, data.len());
    let bit_idx = rand_below(rng, 8) as u8;
    data[byte_idx] ^= 1 << bit_idx;
}

/// Helper function to set random bytes
pub fn set_random_bytes<R: Rand>(data: &mut [u8], rng: &mut R) {
    for byte in data.iter_mut() {
        *byte = rand_below(rng, 256) as u8;
    }
}

/// Helper function to arithmetic mutate (add/sub small value)
pub fn arithmetic_mutate<R: Rand>(data: &mut [u8], rng: &mut R) {
    if data.is_empty() {
        return;
    }
    let delta = (rand_below(rng, 32) as i8) - 16; // -16 to +15
    let idx = rand_below(rng, data.len());
    data[idx] = data[idx].wrapping_add(delta as u8);
}

/// Helper function to interesting value mutation
pub fn interesting_value_mutate<R: Rand>(data: &mut [u8], rng: &mut R, size: usize) {
    let interesting_8: [i8; 9] = [-128, -1, 0, 1, 16, 32, 64, 100, 127];
    let interesting_16: [i16; 10] = [-32768, -129, -1, 0, 1, 128, 255, 256, 512, 32767];
    let interesting_32: [i32; 10] = [
        -2147483648, -65537, -32769, -129, -1, 0, 1, 128, 32767, 2147483647,
    ];

    match size {
        1 => {
            let val = interesting_8[rand_below(rng, interesting_8.len())];
            data[0] = val as u8;
        }
        2 => {
            let val = interesting_16[rand_below(rng, interesting_16.len())];
            let bytes = val.to_le_bytes();
            data[..2].copy_from_slice(&bytes);
        }
        4 => {
            let val = interesting_32[rand_below(rng, interesting_32.len())];
            let bytes = val.to_le_bytes();
            data[..4].copy_from_slice(&bytes);
        }
        8 => {
            let val = interesting_32[rand_below(rng, interesting_32.len())];
            let bytes = (val as i64).to_le_bytes();
            data[..8].copy_from_slice(&bytes);
        }
        _ => {
            // For other sizes, just random bytes
            set_random_bytes(data, rng);
        }
    }
}
