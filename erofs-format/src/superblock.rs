//! EROFS Super Block definitions
//!
//! The super block is located at offset 1024 and contains filesystem metadata.

use serde::{Deserialize, Serialize};
use std::mem::size_of;

/// EROFS super block (144 bytes)
///
/// Located at offset 1024 from the start of the filesystem.
/// Contains global filesystem metadata and configuration.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ErofsSuperBlock {
    /// File system magic number (should be 0xE0F5E1E2)
    pub magic: u32,

    /// CRC32c checksum for super block validation
    pub checksum: u32,

    /// Compatible feature flags
    pub feature_compat: u32,

    /// Block size in bits (e.g., 12 for 4096 bytes)
    pub blkszbits: u8,

    /// Number of extra super block slots (each 16 bytes)
    pub sb_extslots: u8,

    /// Root nid (16-bit) or blocks_hi (48-bit mode)
    pub rootnid_or_blocks_hi: RootNidOrBlocksHi,

    /// Total number of inodes
    pub inos: u64,

    /// Base seconds for compact inode timestamps
    pub epoch: u64,

    /// Fixed nanoseconds for compact inode timestamps
    pub fixed_nsec: u32,

    /// Block count (low 32 bits)
    pub blocks_lo: u32,

    /// Start block address of metadata area
    pub meta_blkaddr: u32,

    /// Start block address of shared xattr area
    pub xattr_blkaddr: u32,

    /// 128-bit UUID for the volume
    pub uuid: [u8; 16],

    /// Volume name (16 bytes, null-terminated)
    pub volume_name: [u8; 16],

    /// Incompatible feature flags
    pub feature_incompat: u32,

    /// Union: available compression algorithms or lz4 max distance
    pub u1: SuperBlockU1,

    /// Number of extra devices (besides primary)
    pub extra_devices: u16,

    /// Start offset of device table (in device slots)
    pub devt_slotoff: u16,

    /// Directory block size in bits
    pub dirblkbits: u8,

    /// Number of long xattr name prefixes
    pub xattr_prefix_count: u8,

    /// Start block of long xattr prefixes
    pub xattr_prefix_start: u32,

    /// NID of the packed inode (for fragments)
    pub packed_nid: u64,

    /// Reserved for xattr name filter
    pub xattr_filter_reserved: u8,

    /// iSHARE xattr prefix ID
    pub ishare_xattr_prefix_id: u8,

    /// Reserved bytes
    pub reserved: [u8; 2],

    /// Build time seconds (added to epoch)
    pub build_time: u32,

    /// Root nid (64-bit, for 48-bit mode)
    pub rootnid_8b: u64,

    /// Reserved
    pub reserved2: u64,

    /// NID of the metabox inode
    pub metabox_nid: u64,

    /// Reserved for alignment
    pub reserved3: u64,
}

/// Union for rootnid and blocks_hi
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub union RootNidOrBlocksHi {
    /// Root directory nid (for 32-bit block addressing)
    pub rootnid_2b: u16,
    /// High bits of blocks count (for 48-bit addressing)
    pub blocks_hi: u16,
}

/// Union for compression config
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub union SuperBlockU1 {
    /// Bitmap for available compression algorithms
    pub available_compr_algs: u16,
    /// Customized sliding window size (for LZ4)
    pub lz4_max_distance: u16,
}

// Compile-time size check
const _: () = assert!(size_of::<ErofsSuperBlock>() == 144);

/// Feature compatibility flags
pub mod feature_compat {
    /// Super block checksum enabled
    pub const SB_CHKSUM: u32 = 0x00000001;
    /// Extended mtime support
    pub const MTIME: u32 = 0x00000002;
    /// XATTR name filter
    pub const XATTR_FILTER: u32 = 0x00000004;
    /// Shared EA in metabox
    pub const SHARED_EA_IN_METABOX: u32 = 0x00000008;
    /// Plain xattr prefix
    pub const PLAIN_XATTR_PFX: u32 = 0x00000010;
    /// iSHARE xattrs
    pub const ISHARE_XATTRS: u32 = 0x00000020;
}

/// Feature incompatibility flags
pub mod feature_incompat {
    /// LZ4 0-padding
    pub const LZ4_0PADDING: u32 = 0x00000001;
    /// Compression configs
    pub const COMPR_CFGS: u32 = 0x00000002;
    /// Big physical cluster
    pub const BIG_PCLUSTER: u32 = 0x00000002;
    /// Chunk-based files
    pub const CHUNKED_FILE: u32 = 0x00000004;
    /// Device table
    pub const DEVICE_TABLE: u32 = 0x00000008;
    /// Compression header v2
    pub const COMPR_HEAD2: u32 = 0x00000008;
    /// Ztailpacking
    pub const ZTAILPACKING: u32 = 0x00000010;
    /// Fragments
    pub const FRAGMENTS: u32 = 0x00000020;
    /// Deduplication
    pub const DEDUPE: u32 = 0x00000020;
    /// Xattr prefixes
    pub const XATTR_PREFIXES: u32 = 0x00000040;
    /// 48-bit support
    pub const BIT48: u32 = 0x00000080;
    /// Metabox
    pub const METABOX: u32 = 0x00000100;
}

impl ErofsSuperBlock {
    /// Create a new super block with default values
    pub fn new() -> Self {
        Self {
            magic: super::EROFS_SUPER_MAGIC_V1,
            checksum: 0,
            feature_compat: 0,
            blkszbits: 12, // 4096 bytes
            sb_extslots: 0,
            rootnid_or_blocks_hi: RootNidOrBlocksHi { rootnid_2b: 0 },
            inos: 0,
            epoch: 0,
            fixed_nsec: 0,
            blocks_lo: 0,
            meta_blkaddr: 0,
            xattr_blkaddr: 0,
            uuid: [0u8; 16],
            volume_name: [0u8; 16],
            feature_incompat: 0,
            u1: SuperBlockU1 {
                available_compr_algs: 0,
            },
            extra_devices: 0,
            devt_slotoff: 0,
            dirblkbits: 12,
            xattr_prefix_count: 0,
            xattr_prefix_start: 0,
            packed_nid: 0,
            xattr_filter_reserved: 0,
            ishare_xattr_prefix_id: 0,
            reserved: [0u8; 2],
            build_time: 0,
            rootnid_8b: 0,
            reserved2: 0,
            metabox_nid: 0,
            reserved3: 0,
        }
    }

    /// Get block size in bytes
    pub fn block_size(&self) -> u32 {
        1u32 << self.blkszbits
    }

    /// Check if super block has a feature
    pub fn has_compat_feature(&self, feature: u32) -> bool {
        (self.feature_compat & feature) != 0
    }

    /// Check if super block has an incompatible feature
    pub fn has_incompat_feature(&self, feature: u32) -> bool {
        (self.feature_incompat & feature) != 0
    }

    /// Check if super block has checksum
    pub fn has_sb_chksum(&self) -> bool {
        self.has_compat_feature(feature_compat::SB_CHKSUM)
    }

    /// Get root directory nid (handles both 32-bit and 48-bit modes)
    pub fn root_nid(&self) -> u64 {
        if self.has_incompat_feature(feature_incompat::BIT48) {
            unsafe { self.rootnid_8b }
        } else {
            unsafe { self.rootnid_or_blocks_hi.rootnid_2b as u64 }
        }
    }

    /// Parse super block from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < size_of::<Self>() {
            return None;
        }

        // Check magic number first
        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        if magic != super::EROFS_SUPER_MAGIC_V1 {
            return None;
        }

        // SAFETY: We've verified the size and magic
        Some(unsafe { std::ptr::read_unaligned(data.as_ptr() as *const Self) })
    }

    /// Write super block to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; size_of::<Self>()];
        // SAFETY: We're writing to a properly sized buffer
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

impl Default for ErofsSuperBlock {
    fn default() -> Self {
        Self::new()
    }
}

/// Device slot entry (128 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ErofsDeviceSlot {
    /// Tag (digest, sha256, etc.)
    pub tag: [u8; 64],
    /// Total blocks count (low 32 bits)
    pub blocks_lo: u32,
    /// Unified starting block (low 32 bits)
    pub uniaddr_lo: u32,
    /// Total blocks count (high 32 bits)
    pub blocks_hi: u32,
    /// Unified starting block (high 16 bits)
    pub uniaddr_hi: u16,
    /// Reserved
    pub reserved: [u8; 50],
}

const _: () = assert!(size_of::<ErofsDeviceSlot>() == 128);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_super_block_defaults() {
        let sb = ErofsSuperBlock::new();
        assert_eq!(sb.magic, EROFS_SUPER_MAGIC_V1);
        assert_eq!(sb.block_size(), 4096);
        assert!(!sb.has_sb_chksum());
    }

    #[test]
    fn test_super_block_roundtrip() {
        let mut sb = ErofsSuperBlock::new();
        sb.blocks_lo = 1000;
        sb.meta_blkaddr = 1;
        sb.blkszbits = 12;

        let bytes = sb.to_bytes();
        let parsed = ErofsSuperBlock::from_bytes(&bytes).expect("Should parse");

        assert_eq!(parsed.magic, sb.magic);
        assert_eq!(parsed.blocks_lo, sb.blocks_lo);
        assert_eq!(parsed.meta_blkaddr, sb.meta_blkaddr);
    }
}
