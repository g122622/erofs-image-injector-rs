//! EROFS Image Input Type
//!
//! Custom LibAFL input type that represents an EROFS filesystem image
//! with metadata about injection points for structure-aware mutations.

use std::hash::{Hash, Hasher};

use libafl::inputs::Input;
use libafl::corpus::CorpusId;
use libafl_bolts::Error;
use serde::{Deserialize, Serialize};

use erofs_format::{
    ErofsSuperBlock,
    EROFS_SUPER_OFFSET,
};

/// EROFS image input type for LibAFL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErofsImageInput {
    /// Raw image data
    data: Vec<u8>,

    /// Injection points for structure-aware mutations
    injection_points: Vec<InjectionPoint>,

    /// Cached super block (if parsed)
    #[serde(skip)]
    super_block: Option<ErofsSuperBlock>,

    /// Cached root inode offset
    #[serde(skip)]
    root_inode_offset: Option<u64>,

    /// Optional name/source of this input (e.g., file name)
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl Hash for ErofsImageInput {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

/// Injection point for mutations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjectionPoint {
    /// Super block field injection
    Superblock {
        /// Field to inject into
        field: SuperblockField,
        /// Offset in the image
        offset: usize,
        /// Size in bytes
        size: usize,
    },

    /// Inode field injection
    Inode {
        /// Node ID
        nid: u64,
        /// Field to inject into
        field: InodeField,
        /// Offset in the image
        offset: usize,
        /// Size in bytes
        size: usize,
        /// Inode layout (compact or extended)
        layout: InodeLayout,
    },

    /// Directory entry injection
    Dirent {
        /// Offset in the image
        offset: usize,
        /// Entry index in the directory
        entry_index: usize,
    },

    /// Extended attribute injection
    Xattr {
        /// Offset in the image
        offset: usize,
        /// Xattr type
        xattr_type: XattrType,
    },

    /// Compression header injection
    Compression {
        /// Offset in the image
        offset: usize,
        /// Compression type
        compression_type: CompressionType,
    },

    /// Raw byte region
    Raw {
        /// Start offset
        offset: usize,
        /// Length
        length: usize,
        /// Description
        description: String,
    },
}

/// Super block fields that can be mutated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum SuperblockField {
    /// Magic number
    Magic,
    /// Checksum
    Checksum,
    /// Compatible features
    FeatureCompat,
    /// Block size bits
    Blkszbits,
    /// Super block extension slots
    SbExtslots,
    /// Root NID
    RootNid,
    /// Total inodes
    Inos,
    /// Epoch time base
    Epoch,
    /// Fixed nanoseconds
    FixedNsec,
    /// Block count
    Blocks,
    /// Metadata block address
    MetaBlkaddr,
    /// Xattr block address
    XattrBlkaddr,
    /// UUID
    Uuid,
    /// Volume name
    VolumeName,
    /// Incompatible features
    FeatureIncompat,
    /// Extra devices
    ExtraDevices,
    /// Device table offset
    DevtSlotoff,
    /// Directory block bits
    Dirblkbits,
    /// Packed inode NID
    PackedNid,
    /// Metabox NID
    MetaboxNid,
}

/// Inode fields that can be mutated
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum InodeField {
    /// Format flags
    Format,
    /// Xattr count
    XattrIcount,
    /// File mode
    Mode,
    /// Nlink or blocks hi
    Nb,
    /// File size
    Size,
    /// Modification time
    Mtime,
    /// Union field (blocks/startblk/rdev)
    U,
    /// Inode number
    Ino,
    /// User ID
    Uid,
    /// Group ID
    Gid,
    /// Nlink (extended only)
    Nlink,
    /// Mtime nanoseconds (extended only)
    MtimeNsec,
}

/// Inode layout type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum InodeLayout {
    /// Compact 32-byte inode
    Compact,
    /// Extended 64-byte inode
    Extended,
}

/// Extended attribute type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum XattrType {
    /// Inline xattr
    Inline,
    /// Shared xattr
    Shared,
    /// Long prefix
    LongPrefix,
}

/// Compression type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum CompressionType {
    /// LZ4
    Lz4,
    /// LZMA
    Lzma,
    /// DEFLATE
    Deflate,
    /// ZSTD
    Zstd,
}

impl ErofsImageInput {
    /// Create a new EROFS image input
    pub fn new(data: Vec<u8>) -> Self {
        let mut input = Self {
            data,
            injection_points: Vec::new(),
            super_block: None,
            root_inode_offset: None,
            name: None,
        };
        input.parse_injection_points();
        input
    }

    /// Create a new EROFS image input with a name
    pub fn with_name(data: Vec<u8>, name: impl Into<String>) -> Self {
        let mut input = Self::new(data);
        input.name = Some(name.into());
        input
    }

    /// Create an empty image
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Get the raw data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get mutable access to the raw data
    pub fn data_mut(&mut self) -> &mut Vec<u8> {
        self.invalidate_cache();
        &mut self.data
    }

    /// Get the injection points
    pub fn injection_points(&self) -> &[InjectionPoint] {
        &self.injection_points
    }

    /// Get the image size
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the image is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the name of this input
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Set the name of this input
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = Some(name.into());
    }

    /// Parse the super block
    pub fn parse_super_block(&mut self) -> Option<&ErofsSuperBlock> {
        if self.super_block.is_some() {
            return self.super_block.as_ref();
        }

        if self.data.len() < EROFS_SUPER_OFFSET + std::mem::size_of::<ErofsSuperBlock>() {
            return None;
        }

        let sb_data = &self.data[EROFS_SUPER_OFFSET..];
        self.super_block = ErofsSuperBlock::from_bytes(sb_data);
        self.super_block.as_ref()
    }

    /// Get the super block
    pub fn super_block(&self) -> Option<&ErofsSuperBlock> {
        self.super_block.as_ref()
    }

    /// Parse injection points from the image
    pub fn parse_injection_points(&mut self) {
        self.injection_points.clear();

        // Parse super block injection points
        if self.data.len() >= EROFS_SUPER_OFFSET + std::mem::size_of::<ErofsSuperBlock>() {
            self.add_superblock_injection_points(EROFS_SUPER_OFFSET);
        }

        // TODO: Parse inode and directory injection points
        // This would require walking the filesystem structure
    }

    /// Add super block injection points
    fn add_superblock_injection_points(&mut self, offset: usize) {
        let fields = [
            (SuperblockField::Magic, 0, 4),
            (SuperblockField::Checksum, 4, 4),
            (SuperblockField::FeatureCompat, 8, 4),
            (SuperblockField::Blkszbits, 12, 1),
            (SuperblockField::SbExtslots, 13, 1),
            (SuperblockField::RootNid, 14, 2),
            (SuperblockField::Inos, 16, 8),
            (SuperblockField::Epoch, 24, 8),
            (SuperblockField::FixedNsec, 32, 4),
            (SuperblockField::Blocks, 36, 4),
            (SuperblockField::MetaBlkaddr, 40, 4),
            (SuperblockField::XattrBlkaddr, 44, 4),
            (SuperblockField::Uuid, 48, 16),
            (SuperblockField::VolumeName, 64, 16),
            (SuperblockField::FeatureIncompat, 80, 4),
            (SuperblockField::ExtraDevices, 84, 2),
            (SuperblockField::DevtSlotoff, 86, 2),
            (SuperblockField::Dirblkbits, 88, 1),
            (SuperblockField::PackedNid, 96, 8),
            (SuperblockField::MetaboxNid, 112, 8),
        ];

        for (field, field_offset, size) in fields {
            self.injection_points.push(InjectionPoint::Superblock {
                field,
                offset: offset + field_offset,
                size,
            });
        }
    }

    /// Add a raw injection point
    pub fn add_raw_injection_point(&mut self, offset: usize, length: usize, description: &str) {
        self.injection_points.push(InjectionPoint::Raw {
            offset,
            length,
            description: description.to_string(),
        });
    }

    /// Get a specific injection point
    pub fn get_injection_point(&self, index: usize) -> Option<&InjectionPoint> {
        self.injection_points.get(index)
    }

    /// Invalidate cached parsed data
    fn invalidate_cache(&mut self) {
        self.super_block = None;
        self.root_inode_offset = None;
    }

    /// Ensure minimum size, padding with zeros if needed
    pub fn ensure_size(&mut self, min_size: usize) {
        if self.data.len() < min_size {
            self.data.resize(min_size, 0);
        }
    }
}

impl Input for ErofsImageInput {
    fn generate_name(&self, _idx: Option<CorpusId>) -> String {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash_slice(&self.data, &mut hasher);
        let hash = hasher.finish();
        format!("erofs_input_{:016x}", hash)
    }
}

impl Default for ErofsImageInput {
    fn default() -> Self {
        Self::empty()
    }
}

impl std::fmt::Display for ErofsImageInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ErofsImageInput(size={}, injection_points={})",
            self.data.len(),
            self.injection_points.len()
        )
    }
}

impl InjectionPoint {
    /// Get the offset of this injection point
    pub fn offset(&self) -> usize {
        match self {
            InjectionPoint::Superblock { offset, .. } => *offset,
            InjectionPoint::Inode { offset, .. } => *offset,
            InjectionPoint::Dirent { offset, .. } => *offset,
            InjectionPoint::Xattr { offset, .. } => *offset,
            InjectionPoint::Compression { offset, .. } => *offset,
            InjectionPoint::Raw { offset, .. } => *offset,
        }
    }

    /// Get the size of this injection point
    pub fn size(&self) -> usize {
        match self {
            InjectionPoint::Superblock { size, .. } => *size,
            InjectionPoint::Inode { size, .. } => *size,
            InjectionPoint::Dirent { .. } => 12, // ErofsDirent size
            InjectionPoint::Xattr { .. } => 4,   // Minimum xattr entry size
            InjectionPoint::Compression { .. } => 8, // Compression header size
            InjectionPoint::Raw { length, .. } => *length,
        }
    }

    /// Get a description of this injection point
    pub fn description(&self) -> String {
        match self {
            InjectionPoint::Superblock { field, .. } => format!("Superblock::{:?}", field),
            InjectionPoint::Inode {
                nid, field, layout, ..
            } => format!("Inode({})[{:?}]::{:?}", nid, layout, field),
            InjectionPoint::Dirent { entry_index, .. } => format!("Dirent[{}]", entry_index),
            InjectionPoint::Xattr { xattr_type, .. } => format!("Xattr::{:?}", xattr_type),
            InjectionPoint::Compression { compression_type, .. } => {
                format!("Compression::{:?}", compression_type)
            }
            InjectionPoint::Raw {
                description, offset, ..
            } => format!("Raw[{}]@{:#x}", description, offset),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let input = ErofsImageInput::empty();
        assert!(input.is_empty());
        assert_eq!(input.len(), 0);
    }

    #[test]
    fn test_input_from_bytes() {
        let data = vec![0u8; 1024 + std::mem::size_of::<ErofsSuperBlock>()];
        let input = ErofsImageInput::new(data.clone());
        assert_eq!(input.len(), data.len());
    }

    #[test]
    fn test_input_roundtrip() {
        let mut data = vec![0u8; 2048];
        // Set magic number
        let magic = erofs_format::EROFS_SUPER_MAGIC_V1.to_le_bytes();
        data[1024..1028].copy_from_slice(&magic);

        let input = ErofsImageInput::new(data.clone());
        let bytes = input.data();
        let input2 = ErofsImageInput::new(bytes.to_vec());

        assert_eq!(input.data(), input2.data());
    }

    #[test]
    fn test_injection_point_offset() {
        let point = InjectionPoint::Superblock {
            field: SuperblockField::Magic,
            offset: 1024,
            size: 4,
        };
        assert_eq!(point.offset(), 1024);
        assert_eq!(point.size(), 4);
    }

    #[test]
    fn test_hash() {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let input1 = ErofsImageInput::new(vec![1, 2, 3, 4]);
        let input2 = ErofsImageInput::new(vec![1, 2, 3, 4]);
        let input3 = ErofsImageInput::new(vec![4, 3, 2, 1]);

        let mut hasher1 = DefaultHasher::new();
        input1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        input2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        let mut hasher3 = DefaultHasher::new();
        input3.hash(&mut hasher3);
        let hash3 = hasher3.finish();

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }
}
