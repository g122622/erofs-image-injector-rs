//! Field Locator for EROFS Images
//!
//! This module provides functions to precisely locate fields within EROFS images
//! for targeted mutations.

use erofs_format::{ErofsDirent, ErofsInodeCompact, ErofsInodeExtended, ErofsSuperBlock};
use erofs_format::EROFS_SUPER_OFFSET;
use erofs_input::{DirentPart, FieldType, InodeField, SuperblockField, TargetLocation};
use std::mem::size_of;

/// Locator for EROFS image fields
pub struct FieldLocator;

impl FieldLocator {
    /// Locate a field in the image based on the target specification
    pub fn locate(data: &[u8], target: &FieldType) -> Option<TargetLocation> {
        match target {
            FieldType::Superblock(field) => Self::locate_superblock_field(data, field),
            FieldType::Inode(field) => Self::locate_first_inode_field(data, field),
            FieldType::Dirent => Self::locate_first_dirent(data),
            FieldType::Xattr => Self::locate_first_xattr(data),
            FieldType::Compression => Self::locate_first_compression(data),
            FieldType::Raw => None,
        }
    }

    /// Locate a superblock field
    pub fn locate_superblock_field(data: &[u8], field: &SuperblockField) -> Option<TargetLocation> {
        if data.len() < EROFS_SUPER_OFFSET + size_of::<ErofsSuperBlock>() {
            return None;
        }

        let base = EROFS_SUPER_OFFSET;
        let (offset, size) = Self::get_superblock_field_offset(field);

        Some(TargetLocation::new(base + offset, size))
    }

    /// Get the offset and size of a superblock field within the superblock
    fn get_superblock_field_offset(field: &SuperblockField) -> (usize, usize) {
        match field {
            // Superblock layout (from erofs-format/src/superblock.rs):
            // magic: u32 @ 0
            // checksum: u32 @ 4
            // feature_compat: u32 @ 8
            // blkszbits: u8 @ 12
            // sb_extslots: u8 @ 13
            // rootnid_or_blocks_hi: union @ 14 (2 bytes)
            // inos: u64 @ 16
            // epoch: u64 @ 24
            // fixed_nsec: u32 @ 32
            // blocks_lo: u32 @ 36
            // meta_blkaddr: u32 @ 40
            // xattr_blkaddr: u32 @ 44
            // uuid: [u8; 16] @ 48
            // volume_name: [u8; 16] @ 64
            // feature_incompat: u32 @ 80
            // u1: union @ 84 (2 bytes)
            // extra_devices: u16 @ 86
            // devt_slotoff: u16 @ 88
            // dirblkbits: u8 @ 90
            // xattr_prefix_count: u8 @ 91
            // xattr_prefix_start: u32 @ 92
            // packed_nid: u64 @ 96
            // xattr_filter_reserved: u8 @ 104
            // ishare_xattr_prefix_id: u8 @ 105
            // reserved: [u8; 2] @ 106
            // build_time: u32 @ 108
            // rootnid_8b: u64 @ 112
            // reserved2: u64 @ 120
            // metabox_nid: u64 @ 128
            // reserved3: u64 @ 136
            SuperblockField::Magic => (0, 4),
            SuperblockField::Checksum => (4, 4),
            SuperblockField::FeatureCompat => (8, 4),
            SuperblockField::Blkszbits => (12, 1),
            SuperblockField::SbExtslots => (13, 1),
            SuperblockField::RootNid => (14, 2),
            SuperblockField::Inos => (16, 8),
            SuperblockField::Epoch => (24, 8),
            SuperblockField::FixedNsec => (32, 4),
            SuperblockField::Blocks => (36, 4),
            SuperblockField::MetaBlkaddr => (40, 4),
            SuperblockField::XattrBlkaddr => (44, 4),
            SuperblockField::Uuid => (48, 16),
            SuperblockField::VolumeName => (64, 16),
            SuperblockField::FeatureIncompat => (80, 4),
            SuperblockField::ExtraDevices => (84, 2),
            SuperblockField::DevtSlotoff => (88, 2),
            SuperblockField::Dirblkbits => (90, 1),
            SuperblockField::PackedNid => (96, 8),
            SuperblockField::MetaboxNid => (128, 8),
        }
    }

    /// Locate the first inode's field (root inode)
    pub fn locate_first_inode_field(data: &[u8], field: &InodeField) -> Option<TargetLocation> {
        // First, we need to find the root inode
        let root_inode_offset = Self::find_root_inode_offset(data)?;
        Self::locate_inode_field_at(data, root_inode_offset, field, true)
    }

    /// Find the offset of the root inode
    pub fn find_root_inode_offset(data: &[u8]) -> Option<usize> {
        if data.len() < EROFS_SUPER_OFFSET + size_of::<ErofsSuperBlock>() {
            return None;
        }

        // Parse superblock to get meta_blkaddr and rootnid
        let sb_data = &data[EROFS_SUPER_OFFSET..];
        let sb = ErofsSuperBlock::from_bytes(sb_data)?;

        // Get block size
        let block_size = sb.block_size() as usize;

        // Get root nid (in 32-bit mode)
        let root_nid = sb.root_nid() as usize;

        // Root inode is at meta_blkaddr * block_size + root_nid * inode_size
        // For compact inodes (32 bytes), the calculation is:
        // offset = meta_blkaddr * block_size + root_nid * 32
        let meta_blkaddr = sb.meta_blkaddr as usize;
        let inode_offset = meta_blkaddr * block_size + root_nid * size_of::<ErofsInodeCompact>();

        if inode_offset + size_of::<ErofsInodeCompact>() > data.len() {
            return None;
        }

        Some(inode_offset)
    }

    /// Locate an inode field at a specific offset
    pub fn locate_inode_field_at(
        data: &[u8],
        inode_offset: usize,
        field: &InodeField,
        is_compact: bool,
    ) -> Option<TargetLocation> {
        let inode_size = if is_compact {
            size_of::<ErofsInodeCompact>()
        } else {
            size_of::<ErofsInodeExtended>()
        };

        if inode_offset + inode_size > data.len() {
            return None;
        }

        let (offset, size) = Self::get_inode_field_offset(field, is_compact);
        Some(TargetLocation::new(inode_offset + offset, size))
    }

    /// Get the offset and size of an inode field
    fn get_inode_field_offset(field: &InodeField, is_compact: bool) -> (usize, usize) {
        // Compact inode layout (32 bytes):
        // i_format: u16 @ 0
        // i_xattr_icount: u16 @ 2
        // i_mode: u16 @ 4
        // i_nb: union @ 6 (2 bytes)
        // i_size: u32 @ 8
        // i_mtime: u32 @ 12
        // i_u: union @ 16 (4 bytes)
        // i_ino: u32 @ 20
        // i_uid: u16 @ 24
        // i_gid: u16 @ 26
        // i_reserved: u32 @ 28
        //
        // Extended inode layout (64 bytes):
        // i_format: u16 @ 0
        // i_xattr_icount: u16 @ 2
        // i_mode: u16 @ 4
        // i_nb: union @ 6 (2 bytes)
        // i_size: u64 @ 8
        // i_u: union @ 16 (4 bytes)
        // i_ino: u32 @ 20
        // i_uid: u32 @ 24
        // i_gid: u32 @ 28
        // i_mtime: u64 @ 32
        // i_mtime_nsec: u32 @ 40
        // i_nlink: u32 @ 44
        // i_reserved2: [u8; 16] @ 48

        match field {
            InodeField::Format => (0, 2),
            InodeField::XattrIcount => (2, 2),
            InodeField::Mode => (4, 2),
            InodeField::Nb => (6, 2),
            InodeField::Size => {
                if is_compact {
                    (8, 4)
                } else {
                    (8, 8)
                }
            }
            InodeField::Mtime => {
                if is_compact {
                    (12, 4)
                } else {
                    (32, 8)
                }
            }
            InodeField::U => (16, 4),
            InodeField::Ino => {
                if is_compact {
                    (20, 4)
                } else {
                    // For extended, actually at 20 as well, but let's keep consistency
                    (20, 4)
                }
            }
            InodeField::Uid => {
                if is_compact {
                    (24, 2)
                } else {
                    (24, 4)
                }
            }
            InodeField::Gid => {
                if is_compact {
                    (26, 2)
                } else {
                    (28, 4)
                }
            }
            InodeField::Nlink => {
                if is_compact {
                    // nlink is stored in i_nb for compact inodes
                    (6, 2)
                } else {
                    (44, 4)
                }
            }
            InodeField::MtimeNsec => {
                if is_compact {
                    // Not present in compact inode
                    (0, 0)
                } else {
                    (40, 4)
                }
            }
        }
    }

    /// Locate the first directory entry
    pub fn locate_first_dirent(data: &[u8]) -> Option<TargetLocation> {
        // Directory entries are located in directory data blocks
        // We need to find the root directory first
        let root_inode_offset = Self::find_root_inode_offset(data)?;

        // Check if root inode is a directory
        let mode_offset = root_inode_offset + 4; // i_mode offset
        if mode_offset + 2 > data.len() {
            return None;
        }
        let mode = u16::from_le_bytes([data[mode_offset], data[mode_offset + 1]]);
        let file_type = (mode >> 12) & 0o17;

        // Directory has S_IFDIR (0o4) in the high bits
        if file_type != 0o4 {
            return None;
        }

        // Get the data block address from the inode
        // For flat directory, data starts at i_u (offset 16 in compact inode)
        let i_u_offset = root_inode_offset + 16;
        if i_u_offset + 4 > data.len() {
            return None;
        }
        let startblk = u32::from_le_bytes([
            data[i_u_offset],
            data[i_u_offset + 1],
            data[i_u_offset + 2],
            data[i_u_offset + 3],
        ]);

        // Parse superblock to get block size
        let sb_data = &data[EROFS_SUPER_OFFSET..];
        let sb = ErofsSuperBlock::from_bytes(sb_data)?;
        let block_size = sb.block_size() as usize;

        // Directory entries start at the data block
        let dirent_offset = (startblk as usize) * block_size;
        if dirent_offset + size_of::<ErofsDirent>() > data.len() {
            return None;
        }

        Some(TargetLocation::new(dirent_offset, size_of::<ErofsDirent>()))
    }

    /// Locate a directory entry by index
    pub fn locate_dirent_by_index(
        data: &[u8],
        index: usize,
        target_part: DirentPart,
    ) -> Option<TargetLocation> {
        let first_dirent = Self::locate_first_dirent(data)?;
        let dirent_size = size_of::<ErofsDirent>();

        let dirent_offset = first_dirent.offset + index * dirent_size;
        if dirent_offset + dirent_size > data.len() {
            return None;
        }

        let (offset, size) = match target_part {
            DirentPart::All => (0, dirent_size),
            DirentPart::Nid => (0, 8), // nid is at offset 0, 8 bytes
            DirentPart::NameOff => (8, 2), // nameoff is at offset 8, 2 bytes
            DirentPart::FileType => (10, 1), // file_type is at offset 10, 1 byte
            DirentPart::Name => {
                // Name location requires parsing nameoff, which is complex
                return None;
            }
        };

        Some(TargetLocation::new(dirent_offset + offset, size))
    }

    /// Locate the first extended attribute
    pub fn locate_first_xattr(data: &[u8]) -> Option<TargetLocation> {
        // Xattrs can be inline (after inode) or shared (in xattr block)
        // For simplicity, we look at the root inode's inline xattr
        let root_inode_offset = Self::find_root_inode_offset(data)?;

        // Get xattr count from inode
        let xattr_icount_offset = root_inode_offset + 2;
        if xattr_icount_offset + 2 > data.len() {
            return None;
        }
        let xattr_icount = u16::from_le_bytes([
            data[xattr_icount_offset],
            data[xattr_icount_offset + 1],
        ]);

        if xattr_icount == 0 {
            return None;
        }

        // Inline xattrs follow the inode
        let xattr_offset = root_inode_offset + size_of::<ErofsInodeCompact>();

        // Each xattr entry has a header (4 bytes) + name + value
        // We return just the first 4 bytes as the entry header
        if xattr_offset + 4 > data.len() {
            return None;
        }

        Some(TargetLocation::new(xattr_offset, 4))
    }

    /// Locate the first compression header
    pub fn locate_first_compression(_data: &[u8]) -> Option<TargetLocation> {
        // Compression headers are in compressed data blocks
        // We need to find a file with compressed data
        // This is complex and requires parsing the filesystem structure
        // For now, return None as a placeholder
        None
    }

    /// Locate a data block by block number
    pub fn locate_data_block(
        data: &[u8],
        block_num: usize,
        offset_in_block: usize,
        length: usize,
    ) -> Option<TargetLocation> {
        // Parse superblock to get block size
        let sb_data = &data[EROFS_SUPER_OFFSET..];
        let sb = ErofsSuperBlock::from_bytes(sb_data)?;
        let block_size = sb.block_size() as usize;

        let block_offset = block_num * block_size + offset_in_block;
        if block_offset + length > data.len() {
            return None;
        }

        Some(TargetLocation::new(block_offset, length))
    }

    /// Check if data has valid superblock
    pub fn has_valid_superblock(data: &[u8]) -> bool {
        if data.len() < EROFS_SUPER_OFFSET + 4 {
            return false;
        }
        let magic = u32::from_le_bytes([
            data[EROFS_SUPER_OFFSET],
            data[EROFS_SUPER_OFFSET + 1],
            data[EROFS_SUPER_OFFSET + 2],
            data[EROFS_SUPER_OFFSET + 3],
        ]);
        magic == erofs_format::EROFS_SUPER_MAGIC_V1
    }

    /// Get all field locations for a target specification with offsets
    pub fn locate_with_offsets(
        data: &[u8],
        target: &FieldType,
        offset_before: usize,
        offset_after: usize,
    ) -> Option<TargetLocation> {
        let base = Self::locate(data, target)?;
        let new_offset = base.offset.saturating_sub(offset_before);
        let new_length = base.length + offset_before + offset_after;

        // Make sure we don't exceed data bounds
        let max_length = data.len().saturating_sub(new_offset);
        let final_length = new_length.min(max_length);

        Some(TargetLocation::new(new_offset, final_length))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_locate_superblock_field() {
        let mut data = vec![0u8; 2048];
        // Set magic
        let magic = erofs_format::EROFS_SUPER_MAGIC_V1.to_le_bytes();
        data[EROFS_SUPER_OFFSET..EROFS_SUPER_OFFSET + 4].copy_from_slice(&magic);

        let loc = FieldLocator::locate_superblock_field(&data, &SuperblockField::Magic);
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.offset, EROFS_SUPER_OFFSET);
        assert_eq!(loc.length, 4);

        let loc = FieldLocator::locate_superblock_field(&data, &SuperblockField::Checksum);
        assert!(loc.is_some());
        let loc = loc.unwrap();
        assert_eq!(loc.offset, EROFS_SUPER_OFFSET + 4);
        assert_eq!(loc.length, 4);
    }

    #[test]
    fn test_has_valid_superblock() {
        let mut data = vec![0u8; 2048];
        assert!(!FieldLocator::has_valid_superblock(&data));

        let magic = erofs_format::EROFS_SUPER_MAGIC_V1.to_le_bytes();
        data[EROFS_SUPER_OFFSET..EROFS_SUPER_OFFSET + 4].copy_from_slice(&magic);
        assert!(FieldLocator::has_valid_superblock(&data));
    }

    #[test]
    fn test_locate_data_block() {
        let mut data = vec![0u8; 8192];
        let magic = erofs_format::EROFS_SUPER_MAGIC_V1.to_le_bytes();
        data[EROFS_SUPER_OFFSET..EROFS_SUPER_OFFSET + 4].copy_from_slice(&magic);
        // Set blkszbits to 12 (4096 bytes block)
        data[EROFS_SUPER_OFFSET + 12] = 12;
        // Set meta_blkaddr - use blocks_lo field for now
        // Actually for this test we just check that the function doesn't crash
        // and handles valid superblock correctly

        // The locate_data_block function needs a valid superblock
        let loc = FieldLocator::locate_data_block(&data, 1, 0, 16);
        // May or may not return Some depending on block bounds
        // Just verify it doesn't crash
        let _ = loc;
    }
}
