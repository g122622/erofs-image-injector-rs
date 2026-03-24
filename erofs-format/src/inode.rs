//! EROFS Inode definitions
//!
//! Inodes contain file/directory metadata and data location information.

use serde::{Deserialize, Serialize};
use std::mem::size_of;

/// Inode data layout types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErofsDataLayout {
    /// Uncompressed flat inode without tail-packing inline data
    FlatPlain = 0,
    /// Compressed inode with non-compact indexes
    CompressedFull = 1,
    /// Uncompressed flat inode with tail-packing inline data
    FlatInline = 2,
    /// Compressed inode with compact indexes
    CompressedCompact = 3,
    /// Chunk-based inode with multi-device support
    ChunkBased = 4,
    /// Reserved layouts 5-7
    Reserved = 5,
}

impl Default for ErofsDataLayout {
    fn default() -> Self {
        Self::FlatPlain
    }
}

/// Inode format bit definitions
pub mod inode_format {
    /// Version mask (bit 0)
    pub const VERSION_MASK: u16 = 0x01;
    /// Data layout mask (bits 1-3)
    pub const DATALAYOUT_MASK: u16 = 0x07;
    /// Version bit position
    pub const VERSION_BIT: u8 = 0;
    /// Data layout bit position
    pub const DATALAYOUT_BIT: u8 = 1;
    /// nlink == 1 for compact inodes (bit 4)
    pub const NLINK_1_BIT: u8 = 4;
    /// Dot omitted for directories (bit 4)
    pub const DOT_OMITTED_BIT: u8 = 4;
    /// All format bits
    pub const ALL: u16 = (1 << (NLINK_1_BIT + 1)) - 1;
}

/// Inode layout types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErofsInodeLayout {
    /// Compact 32-byte inode
    Compact = 0,
    /// Extended 64-byte inode
    Extended = 1,
}

impl Default for ErofsInodeLayout {
    fn default() -> Self {
        Self::Compact
    }
}

/// Union for inode i_nb field
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub union InodeNb {
    /// nlink count (if NLINK_1_BIT is unset)
    pub nlink: u16,
    /// High bits of blocks count
    pub blocks_hi: u16,
    /// High bits of start block
    pub startblk_hi: u16,
}

/// Union for inode i_u field
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub union InodeU {
    /// Total blocks (for compressed inodes)
    pub blocks_lo: u32,
    /// Starting block (for flat inodes)
    pub startblk_lo: u32,
    /// Device ID (for special files)
    pub rdev: u32,
    /// Chunk info (for chunk-based files)
    pub chunk_info: InodeChunkInfo,
}

/// Chunk info for chunk-based inodes
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct InodeChunkInfo {
    /// Format (chunk blkbits, indexes flag, 48-bit flag)
    pub format: u16,
    /// Reserved
    pub reserved: u16,
}

/// Compact inode (32 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ErofsInodeCompact {
    /// Inode format hints
    pub i_format: u16,
    /// Inline xattr count
    pub i_xattr_icount: u16,
    /// File mode
    pub i_mode: u16,
    /// nlink or blocks_hi depending on format
    pub i_nb: InodeNb,
    /// File size (low 32 bits)
    pub i_size: u32,
    /// Modification time
    pub i_mtime: u32,
    /// Union: blocks, startblk, rdev, or chunk info
    pub i_u: InodeU,
    /// Inode number (for 32-bit stat compatibility)
    pub i_ino: u32,
    /// User ID
    pub i_uid: u16,
    /// Group ID
    pub i_gid: u16,
    /// Reserved
    pub i_reserved: u32,
}

// Compile-time size check
const _: () = assert!(size_of::<ErofsInodeCompact>() == 32);

/// Extended inode (64 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ErofsInodeExtended {
    /// Inode format hints
    pub i_format: u16,
    /// Inline xattr count
    pub i_xattr_icount: u16,
    /// File mode
    pub i_mode: u16,
    /// nlink or blocks_hi depending on format
    pub i_nb: InodeNb,
    /// File size (64-bit)
    pub i_size: u64,
    /// Union: blocks, startblk, rdev, or chunk info
    pub i_u: InodeU,
    /// Inode number
    pub i_ino: u32,
    /// User ID (32-bit)
    pub i_uid: u32,
    /// Group ID (32-bit)
    pub i_gid: u32,
    /// Modification time (seconds)
    pub i_mtime: u64,
    /// Modification time (nanoseconds)
    pub i_mtime_nsec: u32,
    /// Link count
    pub i_nlink: u32,
    /// Reserved
    pub i_reserved2: [u8; 16],
}

// Compile-time size check
const _: () = assert!(size_of::<ErofsInodeExtended>() == 64);

impl ErofsInodeCompact {
    /// Create a new compact inode
    pub fn new() -> Self {
        Self {
            i_format: 0,
            i_xattr_icount: 0,
            i_mode: 0o644, // Regular file with rw-r--r--
            i_nb: InodeNb { nlink: 1 },
            i_size: 0,
            i_mtime: 0,
            i_u: InodeU { blocks_lo: 0 },
            i_ino: 0,
            i_uid: 0,
            i_gid: 0,
            i_reserved: 0,
        }
    }

    /// Get inode layout
    pub fn layout(&self) -> ErofsInodeLayout {
        if (self.i_format & inode_format::VERSION_MASK) == 1 {
            ErofsInodeLayout::Extended
        } else {
            ErofsInodeLayout::Compact
        }
    }

    /// Get data layout
    pub fn data_layout(&self) -> ErofsDataLayout {
        let layout = (self.i_format >> inode_format::DATALAYOUT_BIT)
            & inode_format::DATALAYOUT_MASK;
        match layout {
            0 => ErofsDataLayout::FlatPlain,
            1 => ErofsDataLayout::CompressedFull,
            2 => ErofsDataLayout::FlatInline,
            3 => ErofsDataLayout::CompressedCompact,
            4 => ErofsDataLayout::ChunkBased,
            _ => ErofsDataLayout::Reserved,
        }
    }

    /// Check if data is compressed
    pub fn is_compressed(&self) -> bool {
        matches!(
            self.data_layout(),
            ErofsDataLayout::CompressedFull | ErofsDataLayout::CompressedCompact
        )
    }

    /// Check if nlink is 1 (compact inode optimization)
    pub fn has_nlink_1(&self) -> bool {
        (self.i_format & (1 << inode_format::NLINK_1_BIT)) != 0
    }

    /// Get nlink count
    pub fn nlink(&self) -> u32 {
        if self.has_nlink_1() {
            1
        } else {
            unsafe { self.i_nb.nlink as u32 }
        }
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < size_of::<Self>() {
            return None;
        }
        Some(unsafe { std::ptr::read_unaligned(data.as_ptr() as *const Self) })
    }

    /// Write to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; size_of::<Self>()];
        unsafe {
            std::ptr::copy_nonoverlapping(
                self as *const Self as *const u8,
                bytes.as_mut_ptr(),
                size_of::<Self>(),
            );
        }
        bytes
    }
}

impl Default for ErofsInodeCompact {
    fn default() -> Self {
        Self::new()
    }
}

impl ErofsInodeExtended {
    /// Create a new extended inode
    pub fn new() -> Self {
        Self {
            i_format: 1, // Extended layout
            i_xattr_icount: 0,
            i_mode: 0o644,
            i_nb: InodeNb { nlink: 1 },
            i_size: 0,
            i_u: InodeU { blocks_lo: 0 },
            i_ino: 0,
            i_uid: 0,
            i_gid: 0,
            i_mtime: 0,
            i_mtime_nsec: 0,
            i_nlink: 1,
            i_reserved2: [0u8; 16],
        }
    }

    /// Get data layout
    pub fn data_layout(&self) -> ErofsDataLayout {
        let layout = (self.i_format >> inode_format::DATALAYOUT_BIT)
            & inode_format::DATALAYOUT_MASK;
        match layout {
            0 => ErofsDataLayout::FlatPlain,
            1 => ErofsDataLayout::CompressedFull,
            2 => ErofsDataLayout::FlatInline,
            3 => ErofsDataLayout::CompressedCompact,
            4 => ErofsDataLayout::ChunkBased,
            _ => ErofsDataLayout::Reserved,
        }
    }

    /// Check if data is compressed
    pub fn is_compressed(&self) -> bool {
        matches!(
            self.data_layout(),
            ErofsDataLayout::CompressedFull | ErofsDataLayout::CompressedCompact
        )
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < size_of::<Self>() {
            return None;
        }
        Some(unsafe { std::ptr::read_unaligned(data.as_ptr() as *const Self) })
    }

    /// Write to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; size_of::<Self>()];
        unsafe {
            std::ptr::copy_nonoverlapping(
                self as *const Self as *const u8,
                bytes.as_mut_ptr(),
                size_of::<Self>(),
            );
        }
        bytes
    }
}

impl Default for ErofsInodeExtended {
    fn default() -> Self {
        Self::new()
    }
}

/// Chunk format flags
pub mod chunk_format {
    /// Chunk block bits mask
    pub const BLKBITS_MASK: u16 = 0x001F;
    /// Has chunk indexes flag
    pub const INDEXES: u16 = 0x0020;
    /// 48-bit addresses flag
    pub const BIT48: u16 = 0x0040;
    /// All format bits
    pub const ALL: u16 = (BIT48 << 1) - 1;
}

/// Inode chunk index (8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ErofsInodeChunkIndex {
    /// Starting block number (high 16 bits)
    pub startblk_hi: u16,
    /// Device ID
    pub device_id: u16,
    /// Starting block number (low 32 bits)
    pub startblk_lo: u32,
}

const _: () = assert!(size_of::<ErofsInodeChunkIndex>() == 8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inode_compact_size() {
        assert_eq!(size_of::<ErofsInodeCompact>(), 32);
    }

    #[test]
    fn test_inode_extended_size() {
        assert_eq!(size_of::<ErofsInodeExtended>(), 64);
    }

    #[test]
    fn test_inode_data_layout() {
        let mut inode = ErofsInodeCompact::new();
        assert_eq!(inode.data_layout(), ErofsDataLayout::FlatPlain);
        assert!(!inode.is_compressed());

        // Set compressed layout
        inode.i_format = (ErofsDataLayout::CompressedCompact as u16)
            << inode_format::DATALAYOUT_BIT;
        assert_eq!(inode.data_layout(), ErofsDataLayout::CompressedCompact);
        assert!(inode.is_compressed());
    }

    #[test]
    fn test_inode_roundtrip() {
        let mut inode = ErofsInodeCompact::new();
        inode.i_mode = 0o755;
        inode.i_size = 12345;
        inode.i_uid = 1000;
        inode.i_gid = 1000;

        let bytes = inode.to_bytes();
        let parsed = ErofsInodeCompact::from_bytes(&bytes).expect("Should parse");

        assert_eq!(parsed.i_mode, inode.i_mode);
        assert_eq!(parsed.i_size, inode.i_size);
        assert_eq!(parsed.i_uid, inode.i_uid);
        assert_eq!(parsed.i_gid, inode.i_gid);
    }
}
