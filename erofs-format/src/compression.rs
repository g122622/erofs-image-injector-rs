//! EROFS Compression definitions

use serde::{Deserialize, Serialize};
use std::fmt;
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

/// LZ4 compression config (14 bytes)
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

/// LZMA compression config (14 bytes)
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

/// DEFLATE compression config (6 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsDeflateCfgs {
    /// Window bits (8..15)
    pub windowbits: u8,
    /// Reserved
    pub reserved: [u8; 5],
}

const _: () = assert!(size_of::<ZErofsDeflateCfgs>() == 6);

/// ZSTD compression config (6 bytes)
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

/// Inline data size
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsInlineData {
    /// Reserved
    pub h_reserved1: u16,
    /// Encoded size of tailpacking data
    pub h_idata_size: u16,
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

/// Union for fragment offset or inline data
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub union ZErofsMapHeaderU1 {
    /// Fragment data offset in packed inode
    pub h_fragmentoff: u32,
    /// Inline data size and reserved
    pub h_idata: ZErofsInlineData,
    /// Extent count low bits
    pub h_extents_lo: u32,
}

impl fmt::Debug for ZErofsMapHeaderU1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = unsafe { self.h_fragmentoff };
        f.debug_struct("ZErofsMapHeaderU1")
            .field("h_fragmentoff", &val)
            .finish()
    }
}

impl Serialize for ZErofsMapHeaderU1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = unsafe {
            std::slice::from_raw_parts(self as *const Self as *const u8, std::mem::size_of::<Self>())
        };
        serializer.serialize_bytes(bytes)
    }
}

impl<'de> Deserialize<'de> for ZErofsMapHeaderU1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ZErofsMapHeaderU1Visitor;

        impl<'de> serde::de::Visitor<'de> for ZErofsMapHeaderU1Visitor {
            type Value = ZErofsMapHeaderU1;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a byte array of 4 bytes")
            }

            fn visit_bytes<E>(self, bytes: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if bytes.len() != 4 {
                    return Err(E::invalid_length(bytes.len(), &self));
                }
                let mut val = ZErofsMapHeaderU1 { h_fragmentoff: 0 };
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        &mut val as *mut ZErofsMapHeaderU1 as *mut u8,
                        4,
                    );
                }
                Ok(val)
            }
        }

        deserializer.deserialize_bytes(ZErofsMapHeaderU1Visitor)
    }
}

impl Default for ZErofsMapHeaderU1 {
    fn default() -> Self {
        Self { h_fragmentoff: 0 }
    }
}

/// Union for algorithm type or extent count high
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub union ZErofsMapHeaderU2 {
    /// Algorithm type and cluster bits
    pub h_algo: ZErofsAlgoInfo,
    /// Extent count high bits
    pub h_extents_hi: u16,
}

impl fmt::Debug for ZErofsMapHeaderU2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = unsafe { self.h_extents_hi };
        f.debug_struct("ZErofsMapHeaderU2")
            .field("h_extents_hi", &val)
            .finish()
    }
}

impl Serialize for ZErofsMapHeaderU2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = unsafe {
            std::slice::from_raw_parts(self as *const Self as *const u8, std::mem::size_of::<Self>())
        };
        serializer.serialize_bytes(bytes)
    }
}

impl<'de> Deserialize<'de> for ZErofsMapHeaderU2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ZErofsMapHeaderU2Visitor;

        impl<'de> serde::de::Visitor<'de> for ZErofsMapHeaderU2Visitor {
            type Value = ZErofsMapHeaderU2;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a byte array of 2 bytes")
            }

            fn visit_bytes<E>(self, bytes: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if bytes.len() != 2 {
                    return Err(E::invalid_length(bytes.len(), &self));
                }
                let mut val = ZErofsMapHeaderU2 { h_extents_hi: 0 };
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        &mut val as *mut ZErofsMapHeaderU2 as *mut u8,
                        2,
                    );
                }
                Ok(val)
            }
        }

        deserializer.deserialize_bytes(ZErofsMapHeaderU2Visitor)
    }
}

impl Default for ZErofsMapHeaderU2 {
    fn default() -> Self {
        Self { h_extents_hi: 0 }
    }
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
#[derive(Clone, Copy)]
pub union ZErofsLclusterIndexU {
    /// Block address for HEAD lclusters
    pub blkaddr: u32,
    /// Delta values for NONHEAD lclusters
    pub delta: [u16; 2],
}

impl fmt::Debug for ZErofsLclusterIndexU {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = unsafe { self.blkaddr };
        f.debug_struct("ZErofsLclusterIndexU")
            .field("blkaddr", &val)
            .finish()
    }
}

impl Serialize for ZErofsLclusterIndexU {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = unsafe {
            std::slice::from_raw_parts(self as *const Self as *const u8, std::mem::size_of::<Self>())
        };
        serializer.serialize_bytes(bytes)
    }
}

impl<'de> Deserialize<'de> for ZErofsLclusterIndexU {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ZErofsLclusterIndexUVisitor;

        impl<'de> serde::de::Visitor<'de> for ZErofsLclusterIndexUVisitor {
            type Value = ZErofsLclusterIndexU;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a byte array of 4 bytes")
            }

            fn visit_bytes<E>(self, bytes: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if bytes.len() != 4 {
                    return Err(E::invalid_length(bytes.len(), &self));
                }
                let mut val = ZErofsLclusterIndexU { blkaddr: 0 };
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        bytes.as_ptr(),
                        &mut val as *mut ZErofsLclusterIndexU as *mut u8,
                        4,
                    );
                }
                Ok(val)
            }
        }

        deserializer.deserialize_bytes(ZErofsLclusterIndexUVisitor)
    }
}

impl Default for ZErofsLclusterIndexU {
    fn default() -> Self {
        Self { blkaddr: 0 }
    }
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

/// Z_EROFS extent (12 bytes for basic)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ZErofsExtent {
    /// Encoded length with flags
    pub plen: u32,
    /// Physical offset (low 32 bits)
    pub pstart_lo: u32,
    /// Physical offset (high 32 bits) or logical offset
    pub pstart_hi_or_lstart: u32,
}

const _: () = assert!(size_of::<ZErofsExtent>() == 12);

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
        assert_eq!(size_of::<ZErofsExtent>(), 12);
    }

    #[test]
    fn test_extent_recsize() {
        assert_eq!(z_erofs_extent_recsize(0), 4);
        assert_eq!(z_erofs_extent_recsize(2), 8);
        assert_eq!(z_erofs_extent_recsize(4), 16);
    }
}
