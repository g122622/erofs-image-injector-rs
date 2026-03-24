//! EROFS Compression definitions

use serde::{Deserialize, Serialize};
use std::mem::size_of;

/// Maximum supported encoded size of a physical compressed cluster
pub const Z_EROFS_PCLUSTER_MAX_SIZE: u64 = 1024 * 1024;

/// Maximum supported decoded size of a physical compressed cluster
pub const Z_EROFS_PCLUSTER_MAX_DSIZE: u64 = 12 * 1024 * 1024;

/// Compression algorithm types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZErofsCompression {
    /// LZ4 compression
    Lz4 = 0,
    /// LZMA compression
    Lzma = 1,
    /// DEFLATE compression
    Deflate = 2,
    /// ZSTD compression
    Zstd = 3,
    /// Maximum compression type
    Max = 4,
}

impl Default for ZErofsCompression {
    fn default() -> Self {
        Self::Lz4
    }
}

impl TryFrom<u8> for ZErofsCompression {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Lz4),
            1 => Ok(Self::Lzma),
            2 => Ok(Self::Deflate),
            3 => Ok(Self::Zstd),
            _ => Err(()),
        }
    }
}

/// All compression algorithms bitmap
pub const Z_EROFS_ALL_COMPR_ALGS: u16 = (1 << 4) - 1;

/// LZ4 compression config (14 bytes + length field = 16 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsLz4Cfgs {
    /// Maximum distance
    pub max_distance: u16,
    /// Maximum physical cluster blocks
    pub max_pclusterblks: u16,
    /// Reserved
    pub reserved: [u8; 10],
}

const _: () = assert!(size_of::<ZErofsLz4Cfgs>() == 14);

/// LZMA compression config (14 bytes + length field = 16 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsLzmaCfgs {
    /// Dictionary size
    pub dict_size: u32,
    /// Format flags
    pub format: u16,
    /// Reserved
    pub reserved: [u8; 8],
}

const _: () = assert!(size_of::<ZErofsLzmaCfgs>() == 14);

/// Maximum LZMA dictionary size
pub const Z_EROFS_LZMA_MAX_DICT_SIZE: u64 = 8 * Z_EROFS_PCLUSTER_MAX_SIZE;

/// DEFLATE compression config (6 bytes + length field = 8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsDeflateCfgs {
    /// Window bits (8..15)
    pub windowbits: u8,
    /// Reserved
    pub reserved: [u8; 5],
}

const _: () = assert!(size_of::<ZErofsDeflateCfgs>() == 6);

/// ZSTD compression config (6 bytes + length field = 8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsZstdCfgs {
    /// Format flags
    pub format: u8,
    /// Window log minus minimum (10)
    pub windowlog: u8,
    /// Reserved
    pub reserved: [u8; 4],
}

const _: () = assert!(size_of::<ZErofsZstdCfgs>() == 6);

/// Maximum ZSTD dictionary size
pub const Z_EROFS_ZSTD_MAX_DICT_SIZE: u64 = Z_EROFS_PCLUSTER_MAX_SIZE;

/// Compression advise flags
pub mod z_erofs_advise {
    /// Compacted 2B indexes
    pub const COMPACTED_2B: u16 = 0x0001;
    /// Extents metadata for FULL inodes
    pub const EXTENTS: u16 = 0x0001;
    /// Big physical cluster flag 1
    pub const BIG_PCLUSTER_1: u16 = 0x0002;
    /// Big physical cluster flag 2
    pub const BIG_PCLUSTER_2: u16 = 0x0004;
    /// Inline physical cluster
    pub const INLINE_PCLUSTER: u16 = 0x0008;
    /// Interlaced physical cluster
    pub const INTERLACED_PCLUSTER: u16 = 0x0010;
    /// Fragment physical cluster
    pub const FRAGMENT_PCLUSTER: u16 = 0x0020;
    /// Extent record size bit position
    pub const EXTRECSZ_BIT: u8 = 1;
    /// Extent record size mask
    pub const EXTRECSZ_MASK: u16 = 0x0003;
}

/// Fragment inode bit
pub const Z_EROFS_FRAGMENT_INODE_BIT: u8 = 7;

/// Z_EROFS map header (8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsMapHeader {
    /// Fragment offset or inline data size
    pub h_fragmentoff_or_idata: ZErofsMapHeaderU1,
    /// Advise flags
    pub h_advise: u16,
    /// Algorithm type and cluster bits
    pub h_algorithmtype_or_extents_hi: ZErofsMapHeaderU2,
}

/// Union for fragment offset or inline data
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub union ZErofsMapHeaderU1 {
    /// Fragment data offset in packed inode
    pub h_fragmentoff: u32,
    /// Inline data size and reserved
    pub h_idata: ZErofsInlineData,
    /// Extent count low bits
    pub h_extents_lo: u32,
}

/// Inline data size
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsInlineData {
    /// Reserved
    pub h_reserved1: u16,
    /// Encoded size of tailpacking data
    pub h_idata_size: u16,
}

/// Union for algorithm type or extent count high
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub union ZErofsMapHeaderU2 {
    /// Algorithm type and cluster bits
    pub h_algo: ZErofsAlgoInfo,
    /// Extent count high bits
    pub h_extents_hi: u16,
}

/// Algorithm type and cluster bits
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsAlgoInfo {
    /// Algorithm type (bits 0-3: HEAD1, bits 4-7: HEAD2)
    pub h_algorithmtype: u8,
    /// Cluster bits (bits 0-3: logical cluster bits - blkszbits)
    pub h_clusterbits: u8,
}

const _: () = assert!(size_of::<ZErofsMapHeader>() == 8);

/// Logical cluster types
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZErofsLclusterType {
    /// Plain
    Plain = 0,
    /// Head type 1
    Head1 = 1,
    /// Non-head
    NonHead = 2,
    /// Head type 2
    Head2 = 3,
}

/// Logical cluster type mask
pub const Z_EROFS_LI_LCLUSTER_TYPE_MASK: u16 = 0x0003;

/// Partial reference flag for non-compact HEAD
pub const Z_EROFS_LI_PARTIAL_REF: u16 = 1 << 15;

/// D0 compressed block count flag
pub const Z_EROFS_LI_D0_CBLKCNT: u16 = 1 << 11;

/// Logical cluster index (8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsLclusterIndex {
    /// Advise (cluster type, offsets)
    pub di_advise: u16,
    /// Decompress offset in head lcluster
    pub di_clusterofs: u16,
    /// Union: block address or delta values
    pub di_u: ZErofsLclusterIndexU,
}

/// Union for lcluster index
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub union ZErofsLclusterIndexU {
    /// Block address for HEAD lclusters
    pub blkaddr: u32,
    /// Delta values for NONHEAD lclusters
    pub delta: [u16; 2],
}

const _: () = assert!(size_of::<ZErofsLclusterIndex>() == 8);

/// Map header start offset
pub const Z_EROFS_MAP_HEADER_START: fn(usize) -> usize = |end| (end + 7) & !7;

/// Map header end offset
pub fn z_erofs_map_header_end(end: usize) -> usize {
    Z_EROFS_MAP_HEADER_START(end) + size_of::<ZErofsMapHeader>()
}

/// Full index start offset
pub fn z_erofs_full_index_start(end: usize) -> usize {
    z_erofs_map_header_end(end) + 8
}

/// Extent partial length flag
pub const Z_EROFS_EXTENT_PLEN_PARTIAL: u32 = 1 << 27;

/// Extent partial length format bit
pub const Z_EROFS_EXTENT_PLEN_FMT_BIT: u32 = 1 << 28;

/// Extent partial length mask
pub const Z_EROFS_EXTENT_PLEN_MASK: u64 =
    (Z_EROFS_PCLUSTER_MAX_SIZE << 1) - 1;

/// Z_EROFS extent (20 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsExtent {
    /// Encoded length
    pub plen: u32,
    /// Physical offset (low 32 bits)
    pub pstart_lo: u32,
    /// Physical offset (high 32 bits)
    pub pstart_hi: u32,
    /// Logical offset (low 32 bits)
    pub lstart_lo: u32,
    /// Logical offset (high 32 bits)
    pub lstart_hi: u32,
    /// Reserved for future use
    pub reserved: [u8; 12],
}

const _: () = assert!(size_of::<ZErofsExtent>() == 20);

/// Get extent record size based on advise flags
pub fn z_erofs_extent_recsize(advise: u16) -> usize {
    4 << ((advise >> z_erofs_advise::EXTRECSZ_BIT) & z_erofs_advise::EXTRECSZ_MASK) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_type() {
        assert_eq!(
            ZErofsCompression::try_from(0).unwrap(),
            ZErofsCompression::Lz4
        );
        assert_eq!(
            ZErofsCompression::try_from(3).unwrap(),
            ZErofsCompression::Zstd
        );
        assert!(ZErofsCompression::try_from(99).is_err());
    }

    #[test]
    fn test_map_header_size() {
        assert_eq!(size_of::<ZErofsMapHeader>(), 8);
    }

    #[test]
    fn test_lcluster_index_size() {
        assert_eq!(size_of::<ZErofsLclusterIndex>(), 8);
    }

    #[test]
    fn test_extent_size() {
        assert_eq!(size_of::<ZErofsExtent>(), 20);
    }

    #[test]
    fn test_extent_recsize() {
        assert_eq!(z_erofs_extent_recsize(0), 4);
        assert_eq!(z_erofs_extent_recsize(2), 8);
        assert_eq!(z_erofs_extent_recsize(4), 16);
    }
}
