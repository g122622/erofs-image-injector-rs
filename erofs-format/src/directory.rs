//! EROFS Directory entry definitions

use serde::{Deserialize, Serialize};
use std::mem::size_of;

/// Directory entry (12 bytes)
///
/// Directory entries are sorted alphabetically, allowing binary search.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ErofsDirent {
    /// Node ID (with METABOX bit for metabox entries)
    pub nid: u64,
    /// Start offset of file name in directory block
    pub nameoff: u16,
    /// File type (EROFS_FT_*)
    pub file_type: u8,
    /// Reserved
    pub reserved: u8,
}

// Compile-time size check
const _: () = assert!(size_of::<ErofsDirent>() == 12);

/// Dirent NID metabox bit
pub const EROFS_DIRENT_NID_METABOX_BIT: u8 = 63;

/// Mask for dirent NID (excluding metabox bit)
pub const EROFS_DIRENT_NID_MASK: u64 = (1u64 << EROFS_DIRENT_NID_METABOX_BIT) - 1;

/// File types for directory entries
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErofsFileType {
    /// Unknown file type
    Unknown = 0,
    /// Regular file
    RegFile = 1,
    /// Directory
    Dir = 2,
    /// Character device
    ChrDev = 3,
    /// Block device
    BlkDev = 4,
    /// FIFO
    Fifo = 5,
    /// Socket
    Sock = 6,
    /// Symbolic link
    Symlink = 7,
    /// Maximum file type value
    Max = 8,
}

impl Default for ErofsFileType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl TryFrom<u8> for ErofsFileType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::RegFile),
            2 => Ok(Self::Dir),
            3 => Ok(Self::ChrDev),
            4 => Ok(Self::BlkDev),
            5 => Ok(Self::Fifo),
            6 => Ok(Self::Sock),
            7 => Ok(Self::Symlink),
            _ => Err(()),
        }
    }
}

impl ErofsDirent {
    /// Create a new directory entry
    pub fn new() -> Self {
        Self {
            nid: 0,
            nameoff: 0,
            file_type: ErofsFileType::Unknown as u8,
            reserved: 0,
        }
    }

    /// Check if this entry is in the metabox
    pub fn is_metabox(&self) -> bool {
        (self.nid >> EROFS_DIRENT_NID_METABOX_BIT) != 0
    }

    /// Get the actual NID (without metabox bit)
    pub fn actual_nid(&self) -> u64 {
        self.nid & EROFS_DIRENT_NID_MASK
    }

    /// Get file type
    pub fn file_type(&self) -> ErofsFileType {
        ErofsFileType::try_from(self.file_type).unwrap_or(ErofsFileType::Unknown)
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

impl Default for ErofsDirent {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert file type to mode bits (for stat)
pub fn erofs_ftype_to_mode(ftype: ErofsFileType, perm: u16) -> u16 {
    let type_bits = match ftype {
        ErofsFileType::RegFile => 0o100000, // S_IFREG
        ErofsFileType::Dir => 0o040000,     // S_IFDIR
        ErofsFileType::ChrDev => 0o020000,  // S_IFCHR
        ErofsFileType::BlkDev => 0o060000,  // S_IFBLK
        ErofsFileType::Fifo => 0o010000,    // S_IFIFO
        ErofsFileType::Sock => 0o140000,    // S_IFSOCK
        ErofsFileType::Symlink => 0o120000, // S_IFLNK
        ErofsFileType::Unknown | ErofsFileType::Max => 0,
    };
    (type_bits as u16) | (perm & 0o7777)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirent_size() {
        assert_eq!(size_of::<ErofsDirent>(), 12);
    }

    #[test]
    fn test_dirent_metabox() {
        let mut dirent = ErofsDirent::new();
        dirent.nid = 123;
        assert!(!dirent.is_metabox());
        assert_eq!(dirent.actual_nid(), 123);

        // Set metabox bit
        dirent.nid |= 1u64 << EROFS_DIRENT_NID_METABOX_BIT;
        assert!(dirent.is_metabox());
        assert_eq!(dirent.actual_nid(), 123);
    }

    #[test]
    fn test_file_type_conversion() {
        assert_eq!(
            ErofsFileType::try_from(1).unwrap(),
            ErofsFileType::RegFile
        );
        assert_eq!(ErofsFileType::try_from(2).unwrap(), ErofsFileType::Dir);
        assert!(ErofsFileType::try_from(99).is_err());
    }
}
