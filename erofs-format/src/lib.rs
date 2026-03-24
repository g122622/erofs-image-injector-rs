//! EROFS (Enhanced Read-Only File System) on-disk format definitions
//!
//! This crate provides Rust definitions for the EROFS filesystem on-disk format,
//! based on the kernel and erofs-utils implementation.
//!
//! # References
//! - Linux kernel: `fs/erofs/erofs_fs.h`
//! - erofs-utils: `include/erofs_fs.h`

#![deny(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

mod superblock;
mod inode;
mod directory;
mod xattr;
mod compression;
mod utils;

pub use superblock::*;
pub use inode::*;
pub use directory::*;
pub use xattr::*;
pub use compression::*;
pub use utils::*;

/// EROFS super magic number
pub const EROFS_SUPER_MAGIC_V1: u32 = 0xE0F5E1E2;

/// EROFS super block offset (1024 bytes from start)
pub const EROFS_SUPER_OFFSET: usize = 1024;

/// Maximum EROFS name length
pub const EROFS_NAME_LEN: usize = 255;

/// EROFS device slot size
pub const EROFS_DEVT_SLOT_SIZE: usize = 128;

/// Re-export common types
pub mod prelude {
    pub use crate::{ErofsSuperBlock, ErofsInodeCompact, ErofsInodeExtended, ErofsDirent};
    pub use crate::{EROFS_SUPER_MAGIC_V1, EROFS_SUPER_OFFSET};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_super_block_size() {
        assert_eq!(std::mem::size_of::<ErofsSuperBlock>(), 144);
    }

    #[test]
    fn test_inode_compact_size() {
        assert_eq!(std::mem::size_of::<ErofsInodeCompact>(), 32);
    }

    #[test]
    fn test_inode_extended_size() {
        assert_eq!(std::mem::size_of::<ErofsInodeExtended>(), 64);
    }

    #[test]
    fn test_dirent_size() {
        assert_eq!(std::mem::size_of::<ErofsDirent>(), 12);
    }
}
