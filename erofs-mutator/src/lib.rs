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

pub use bitflip_mutator::*;
pub use superblock_mutator::*;
pub use inode_mutator::*;
pub use directory_mutator::*;
pub use xattr_mutator::*;

use libafl::mutators::MutationResult;
use libafl::state::HasMetadata;
use libafl_bolts::rands::Rand;
use libafl_bolts::Error;

/// Re-export common types
pub mod prelude {
    pub use crate::{
        ErofsBitflipMutator, ErofsSuperblockMutator, ErofsInodeMutator,
        ErofsDirectoryMutator, ErofsXattrMutator,
    };
    pub use libafl::mutators::MutationResult;
}

/// Helper function to get a random value in range
fn rand_in_range<R: Rand>(rng: &mut R, min: u64, max: u64) -> u64 {
    if max <= min {
        return min;
    }
    min + rng.below(max - min)
}

/// Helper function to flip a random bit in a byte slice
fn flip_random_bit<R: Rand>(data: &mut [u8], rng: &mut R) {
    if data.is_empty() {
        return;
    }
    let byte_idx = rng.below(data.len() as u64) as usize;
    let bit_idx = rng.below(8) as u8;
    data[byte_idx] ^= 1 << bit_idx;
}

/// Helper function to set random bytes
fn set_random_bytes<R: Rand>(data: &mut [u8], rng: &mut R) {
    for byte in data.iter_mut() {
        *byte = rng.below(256) as u8;
    }
}

/// Helper function to arithmetic mutate (add/sub small value)
fn arithmetic_mutate<R: Rand>(data: &mut [u8], rng: &mut R) {
    if data.is_empty() {
        return;
    }
    let delta = (rng.below(32) as i8) - 16; // -16 to +15
    let idx = rng.below(data.len() as u64) as usize;
    data[idx] = data[idx].wrapping_add(delta as u8);
}

/// Helper function to interesting value mutation
fn interesting_value_mutate<R: Rand>(data: &mut [u8], rng: &mut R, size: usize) {
    let interesting_8: [i8; 9] = [-128, -1, 0, 1, 16, 32, 64, 100, 127];
    let interesting_16: [i16; 10] = [-32768, -129, -1, 0, 1, 128, 255, 256, 512, 32767];
    let interesting_32: [i32; 10] = [
        -2147483648, -65537, -32769, -129, -1, 0, 1, 128, 32767, 2147483647,
    ];

    match size {
        1 => {
            let val = interesting_8[rng.below(interesting_8.len() as u64) as usize];
            data[0] = val as u8;
        }
        2 => {
            let val = interesting_16[rng.below(interesting_16.len() as u64) as usize];
            let bytes = val.to_le_bytes();
            data[..2].copy_from_slice(&bytes);
        }
        4 => {
            let val = interesting_32[rng.below(interesting_32.len() as u64) as usize];
            let bytes = val.to_le_bytes();
            data[..4].copy_from_slice(&bytes);
        }
        8 => {
            let val = interesting_32[rng.below(interesting_32.len() as u64) as usize];
            let bytes = (val as i64).to_le_bytes();
            data[..8].copy_from_slice(&bytes);
        }
        _ => {
            // For other sizes, just random bytes
            set_random_bytes(data, rng);
        }
    }
}
