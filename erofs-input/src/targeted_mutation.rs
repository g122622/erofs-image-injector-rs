//! Targeted Mutation Configuration
//!
//! This module provides configuration types for precise, targeted mutations
//! that allow fuzzing specific fields or byte ranges within EROFS images.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::{InodeField, SuperblockField};

/// Configuration for targeted mutation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetedMutationConfig {
    /// The target to mutate
    pub target: MutationTarget,
    /// The mutation strategy to apply
    pub strategy: MutationStrategy,
    /// Maximum number of mutations to apply
    #[serde(default = "default_max_mutations")]
    pub max_mutations: usize,
}

fn default_max_mutations() -> usize {
    1
}

impl Default for TargetedMutationConfig {
    fn default() -> Self {
        Self {
            target: MutationTarget::AbsoluteRange { start: 0, length: 4 },
            strategy: MutationStrategy::BitFlip { count: 1 },
            max_mutations: 1,
        }
    }
}

/// Target specification for mutation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationTarget {
    /// Target a specific field with optional surrounding bytes
    FieldRange {
        /// The field to target
        field: FieldType,
        /// Number of bytes before the field to include
        offset_before: usize,
        /// Number of bytes after the field to include
        offset_after: usize,
    },
    /// Target an absolute byte range
    AbsoluteRange {
        /// Start offset in bytes
        start: usize,
        /// Length in bytes
        length: usize,
    },
    /// Target a specific inode by index
    InodeByIndex {
        /// Index of the inode (0-based)
        index: usize,
        /// The field within the inode to target
        field: InodeField,
        /// Bytes before the field
        offset_before: usize,
        /// Bytes after the field
        offset_after: usize,
    },
    /// Target a specific directory entry
    DirentByIndex {
        /// Index of the directory entry (0-based)
        index: usize,
        /// Target the entire dirent or specific field
        target_part: DirentPart,
    },
    /// Target a data block
    DataBlock {
        /// Block number
        block_num: usize,
        /// Offset within the block
        offset_in_block: usize,
        /// Length to mutate
        length: usize,
    },
}

/// Field types that can be targeted for mutation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FieldType {
    /// Superblock field
    Superblock(SuperblockField),
    /// Inode field
    Inode(InodeField),
    /// Directory entry
    Dirent,
    /// Extended attribute
    Xattr,
    /// Compression header
    Compression,
    /// Raw data
    Raw,
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::Superblock(field) => write!(f, "superblock.{}", field_name(field)),
            FieldType::Inode(field) => write!(f, "inode.{}", inode_field_name(field)),
            FieldType::Dirent => write!(f, "dirent"),
            FieldType::Xattr => write!(f, "xattr"),
            FieldType::Compression => write!(f, "compression"),
            FieldType::Raw => write!(f, "raw"),
        }
    }
}

/// Part of a directory entry to target
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DirentPart {
    /// The entire directory entry
    All,
    /// Node ID field
    Nid,
    /// Name offset field
    NameOff,
    /// File type field
    FileType,
    /// Name data
    Name,
}

/// Mutation strategy to apply
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MutationStrategy {
    /// Flip random bits
    BitFlip {
        /// Number of bits to flip
        count: usize,
    },
    /// Replace with specific bytes
    ByteReplace {
        /// Values to use for replacement
        values: Vec<u8>,
    },
    /// Arithmetic mutation (add/subtract small values)
    Arithmetic {
        /// Minimum delta
        min_delta: i8,
        /// Maximum delta
        max_delta: i8,
    },
    /// Use interesting values (edge cases)
    InterestingValues {
        /// Size of the field (1, 2, 4, or 8 bytes)
        size: usize,
    },
    /// Boundary values (0, max, min, etc.)
    Boundary {
        /// Pre-defined boundary values to try
        values: Vec<Vec<u8>>,
    },
    /// Random bytes
    Random,
    /// Set to zero
    Zero,
    /// Set to max (0xFF)
    Max,
}

impl Default for MutationStrategy {
    fn default() -> Self {
        MutationStrategy::BitFlip { count: 1 }
    }
}

impl MutationStrategy {
    /// Create a bit flip strategy
    pub fn bitflip(count: usize) -> Self {
        MutationStrategy::BitFlip { count }
    }

    /// Create an arithmetic strategy
    pub fn arithmetic(min: i8, max: i8) -> Self {
        MutationStrategy::Arithmetic {
            min_delta: min,
            max_delta: max,
        }
    }

    /// Create an interesting values strategy
    pub fn interesting(size: usize) -> Self {
        MutationStrategy::InterestingValues { size }
    }

    /// Create a boundary values strategy for a field
    pub fn boundary_for_size(size: usize) -> Self {
        let values = match size {
            1 => vec![
                vec![0x00],
                vec![0xFF],
                vec![0x7F],
                vec![0x80],
                vec![0x01],
            ],
            2 => vec![
                vec![0x00, 0x00],
                vec![0xFF, 0xFF],
                vec![0x00, 0x80],
                vec![0xFF, 0x7F],
                vec![0x01, 0x00],
            ],
            4 => vec![
                vec![0x00, 0x00, 0x00, 0x00],
                vec![0xFF, 0xFF, 0xFF, 0xFF],
                vec![0x00, 0x00, 0x00, 0x80],
                vec![0xFF, 0xFF, 0xFF, 0x7F],
            ],
            8 => vec![
                vec![0x00; 8],
                vec![0xFF; 8],
                vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80],
            ],
            _ => vec![vec![0x00; size], vec![0xFF; size]],
        };
        MutationStrategy::Boundary { values }
    }

    /// Parse strategy from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        let s = s.to_lowercase();
        match s.as_str() {
            "bitflip" | "bit" => Ok(MutationStrategy::BitFlip { count: 1 }),
            "arithmetic" | "arith" => Ok(MutationStrategy::Arithmetic {
                min_delta: -16,
                max_delta: 16,
            }),
            "interesting" | "interest" => Ok(MutationStrategy::InterestingValues { size: 4 }),
            "boundary" | "bound" => Ok(MutationStrategy::boundary_for_size(4)),
            "random" | "rand" => Ok(MutationStrategy::Random),
            "zero" => Ok(MutationStrategy::Zero),
            "max" | "allones" => Ok(MutationStrategy::Max),
            _ => Err(format!("Unknown strategy: {}", s)),
        }
    }
}

/// Location of a target in the image
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetLocation {
    /// Start offset in bytes
    pub offset: usize,
    /// Length in bytes
    pub length: usize,
}

impl TargetLocation {
    /// Create a new target location
    pub fn new(offset: usize, length: usize) -> Self {
        Self { offset, length }
    }
}

/// Parse a target specification string
///
/// Formats:
/// - `superblock.field_name` - Target a superblock field
/// - `inode.field_name` - Target the first inode's field
/// - `inode[N].field_name` - Target the Nth inode's field
/// - `dirent[N]` - Target the Nth directory entry
/// - `range:OFFSET:LEN` - Target an absolute byte range
pub fn parse_target(spec: &str) -> Result<MutationTarget, String> {
    let spec = spec.trim();

    // Check for range specification
    if spec.starts_with("range:") {
        let parts: Vec<&str> = spec[6..].split(':').collect();
        if parts.len() != 2 {
            return Err("Range format: range:OFFSET:LENGTH".to_string());
        }
        let start = parts[0]
            .parse::<usize>()
            .map_err(|_| "Invalid offset")?;
        let length = parts[1]
            .parse::<usize>()
            .map_err(|_| "Invalid length")?;
        return Ok(MutationTarget::AbsoluteRange { start, length });
    }

    // Parse struct.field format
    let parts: Vec<&str> = spec.split('.').collect();
    if parts.len() == 2 {
        let struct_name = parts[0].to_lowercase();
        let field_name = parts[1].to_lowercase();

        match struct_name.as_str() {
            "superblock" | "sb" => {
                let field = parse_superblock_field(&field_name)?;
                Ok(MutationTarget::FieldRange {
                    field: FieldType::Superblock(field),
                    offset_before: 0,
                    offset_after: 0,
                })
            }
            "inode" => {
                let field = parse_inode_field(&field_name)?;
                Ok(MutationTarget::FieldRange {
                    field: FieldType::Inode(field),
                    offset_before: 0,
                    offset_after: 0,
                })
            }
            "dirent" => {
                Ok(MutationTarget::DirentByIndex {
                    index: 0,
                    target_part: DirentPart::All,
                })
            }
            _ => Err(format!("Unknown struct type: {}", struct_name)),
        }
    } else if parts.len() == 1 {
        // Just a struct name, target entire struct
        match parts[0].to_lowercase().as_str() {
            "superblock" | "sb" => {
                Ok(MutationTarget::AbsoluteRange {
                    start: 1024, // EROFS_SUPER_OFFSET
                    length: 144, // ErofsSuperBlock size
                })
            }
            _ => Err(format!("Unknown struct type: {}", parts[0])),
        }
    } else {
        Err(format!("Invalid target format: {}", spec))
    }
}

/// Parse a superblock field name
fn parse_superblock_field(name: &str) -> Result<SuperblockField, String> {
    let field = match name.to_lowercase().as_str() {
        "magic" => SuperblockField::Magic,
        "checksum" | "crc" => SuperblockField::Checksum,
        "feature_compat" | "compat" => SuperblockField::FeatureCompat,
        "blkszbits" | "block_size_bits" | "blksz" => SuperblockField::Blkszbits,
        "sb_extslots" | "extslots" => SuperblockField::SbExtslots,
        "rootnid" | "root_nid" => SuperblockField::RootNid,
        "inos" | "inode_count" => SuperblockField::Inos,
        "epoch" => SuperblockField::Epoch,
        "fixed_nsec" | "nsec" => SuperblockField::FixedNsec,
        "blocks" | "block_count" => SuperblockField::Blocks,
        "meta_blkaddr" | "meta_block" => SuperblockField::MetaBlkaddr,
        "xattr_blkaddr" | "xattr_block" => SuperblockField::XattrBlkaddr,
        "uuid" => SuperblockField::Uuid,
        "volume_name" | "volume" => SuperblockField::VolumeName,
        "feature_incompat" | "incompat" => SuperblockField::FeatureIncompat,
        "extra_devices" => SuperblockField::ExtraDevices,
        "devt_slotoff" | "device_offset" => SuperblockField::DevtSlotoff,
        "dirblkbits" | "dir_block_bits" => SuperblockField::Dirblkbits,
        "packed_nid" => SuperblockField::PackedNid,
        "metabox_nid" => SuperblockField::MetaboxNid,
        _ => return Err(format!("Unknown superblock field: {}", name)),
    };
    Ok(field)
}

/// Parse an inode field name
fn parse_inode_field(name: &str) -> Result<InodeField, String> {
    let field = match name.to_lowercase().as_str() {
        "i_format" | "format" => InodeField::Format,
        "i_xattr_icount" | "xattr_count" | "xattr_icount" => InodeField::XattrIcount,
        "i_mode" | "mode" => InodeField::Mode,
        "i_nb" | "nb" => InodeField::Nb,
        "i_size" | "size" => InodeField::Size,
        "i_mtime" | "mtime" => InodeField::Mtime,
        "i_u" | "u" | "blocks" | "startblk" => InodeField::U,
        "i_ino" | "ino" | "inode_num" => InodeField::Ino,
        "i_uid" | "uid" => InodeField::Uid,
        "i_gid" | "gid" => InodeField::Gid,
        "i_nlink" | "nlink" => InodeField::Nlink,
        "i_mtime_nsec" | "mtime_nsec" => InodeField::MtimeNsec,
        _ => return Err(format!("Unknown inode field: {}", name)),
    };
    Ok(field)
}

/// Get the name of a superblock field
fn field_name(field: &SuperblockField) -> &'static str {
    match field {
        SuperblockField::Magic => "magic",
        SuperblockField::Checksum => "checksum",
        SuperblockField::FeatureCompat => "feature_compat",
        SuperblockField::Blkszbits => "blkszbits",
        SuperblockField::SbExtslots => "sb_extslots",
        SuperblockField::RootNid => "rootnid",
        SuperblockField::Inos => "inos",
        SuperblockField::Epoch => "epoch",
        SuperblockField::FixedNsec => "fixed_nsec",
        SuperblockField::Blocks => "blocks",
        SuperblockField::MetaBlkaddr => "meta_blkaddr",
        SuperblockField::XattrBlkaddr => "xattr_blkaddr",
        SuperblockField::Uuid => "uuid",
        SuperblockField::VolumeName => "volume_name",
        SuperblockField::FeatureIncompat => "feature_incompat",
        SuperblockField::ExtraDevices => "extra_devices",
        SuperblockField::DevtSlotoff => "devt_slotoff",
        SuperblockField::Dirblkbits => "dirblkbits",
        SuperblockField::PackedNid => "packed_nid",
        SuperblockField::MetaboxNid => "metabox_nid",
    }
}

/// Get the name of an inode field
fn inode_field_name(field: &InodeField) -> &'static str {
    match field {
        InodeField::Format => "i_format",
        InodeField::XattrIcount => "i_xattr_icount",
        InodeField::Mode => "i_mode",
        InodeField::Nb => "i_nb",
        InodeField::Size => "i_size",
        InodeField::Mtime => "i_mtime",
        InodeField::U => "i_u",
        InodeField::Ino => "i_ino",
        InodeField::Uid => "i_uid",
        InodeField::Gid => "i_gid",
        InodeField::Nlink => "i_nlink",
        InodeField::MtimeNsec => "i_mtime_nsec",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_target_superblock() {
        let target = parse_target("superblock.checksum").unwrap();
        match target {
            MutationTarget::FieldRange { field, .. } => {
                assert_eq!(field, FieldType::Superblock(SuperblockField::Checksum));
            }
            _ => panic!("Expected FieldRange"),
        }
    }

    #[test]
    fn test_parse_target_range() {
        let target = parse_target("range:1024:8").unwrap();
        match target {
            MutationTarget::AbsoluteRange { start, length } => {
                assert_eq!(start, 1024);
                assert_eq!(length, 8);
            }
            _ => panic!("Expected AbsoluteRange"),
        }
    }

    #[test]
    fn test_parse_strategy() {
        assert!(matches!(
            MutationStrategy::from_str("bitflip"),
            Ok(MutationStrategy::BitFlip { .. })
        ));
        assert!(matches!(
            MutationStrategy::from_str("arithmetic"),
            Ok(MutationStrategy::Arithmetic { .. })
        ));
        assert!(matches!(
            MutationStrategy::from_str("zero"),
            Ok(MutationStrategy::Zero)
        ));
    }

    #[test]
    fn test_strategy_boundary() {
        let strategy = MutationStrategy::boundary_for_size(4);
        match strategy {
            MutationStrategy::Boundary { values } => {
                assert!(!values.is_empty());
                assert!(values.iter().all(|v| v.len() == 4));
            }
            _ => panic!("Expected Boundary"),
        }
    }

    #[test]
    fn test_target_location() {
        let loc = TargetLocation::new(1024, 4);
        assert_eq!(loc.offset, 1024);
        assert_eq!(loc.length, 4);
    }
}
