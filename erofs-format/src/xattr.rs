//! EROFS Extended Attribute (xattr) definitions

use serde::{Deserialize, Serialize};
use std::mem::size_of;

/// Xattr name indexes
pub mod xattr_index {
    /// User namespace
    pub const USER: u8 = 1;
    /// POSIX ACL access
    pub const POSIX_ACL_ACCESS: u8 = 2;
    /// POSIX ACL default
    pub const POSIX_ACL_DEFAULT: u8 = 3;
    /// Trusted namespace
    pub const TRUSTED: u8 = 4;
    /// Lustre namespace
    pub const LUSTRE: u8 = 5;
    /// Security namespace
    pub const SECURITY: u8 = 6;
}

/// Long xattr prefix flag (bit 7)
pub const EROFS_XATTR_LONG_PREFIX: u8 = 0x80;

/// Long xattr prefix mask
pub const EROFS_XATTR_LONG_PREFIX_MASK: u8 = 0x7f;

/// Xattr filter bits
pub const EROFS_XATTR_FILTER_BITS: u32 = 32;

/// Xattr filter default value
pub const EROFS_XATTR_FILTER_DEFAULT: u32 = u32::MAX;

/// Xattr filter seed
pub const EROFS_XATTR_FILTER_SEED: u32 = 0x25BBE08F;

/// Inline xattr header (12 bytes)
///
/// Inline xattrs start with this header followed by shared xattr indices.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ErofsXattrIbodyHeader {
    /// Bit value 1 indicates not-present
    pub h_name_filter: u32,
    /// Number of shared xattrs
    pub h_shared_count: u8,
    /// Reserved
    pub h_reserved2: [u8; 7],
    // Followed by h_shared_count * u32 shared xattr IDs
}

const _: () = assert!(size_of::<ErofsXattrIbodyHeader>() == 12);

/// Xattr entry (4 bytes + name + value)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ErofsXattrEntry {
    /// Length of name
    pub e_name_len: u8,
    /// Attribute name index
    pub e_name_index: u8,
    /// Size of attribute value
    pub e_value_size: u16,
    // Followed by e_name_len bytes of name
    // Followed by e_value_size bytes of value
}

const _: () = assert!(size_of::<ErofsXattrEntry>() == 4);

/// Long xattr name prefix
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ErofsXattrLongPrefix {
    /// Base xattr name prefix index
    pub base_index: u8,
    /// Infix string (null-terminated)
    pub infix: [u8; 0], // Flexible array
}

impl ErofsXattrIbodyHeader {
    /// Create a new inline xattr header
    pub fn new() -> Self {
        Self {
            h_name_filter: EROFS_XATTR_FILTER_DEFAULT,
            h_shared_count: 0,
            h_reserved2: [0u8; 7],
        }
    }

    /// Calculate inline xattr size
    pub fn inline_size(&self) -> usize {
        if self.h_shared_count == 0 {
            0
        } else {
            size_of::<Self>() + (self.h_shared_count as usize - 1) * 4
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

impl Default for ErofsXattrIbodyHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl ErofsXattrEntry {
    /// Create a new xattr entry
    pub fn new(name_index: u8, name: &str, value_size: u16) -> Self {
        Self {
            e_name_len: name.len() as u8,
            e_name_index: name_index,
            e_value_size,
        }
    }

    /// Check if this entry has a long prefix
    pub fn has_long_prefix(&self) -> bool {
        (self.e_name_index & EROFS_XATTR_LONG_PREFIX) != 0
    }

    /// Get the actual name index (without long prefix flag)
    pub fn actual_index(&self) -> u8 {
        self.e_name_index & EROFS_XATTR_LONG_PREFIX_MASK
    }

    /// Calculate total entry size (aligned)
    pub fn total_size(&self) -> usize {
        let size = size_of::<Self>() + self.e_name_len as usize + self.e_value_size as usize;
        // Align to 4 bytes
        (size + 3) & !3
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

impl Default for ErofsXattrEntry {
    fn default() -> Self {
        Self {
            e_name_len: 0,
            e_name_index: 0,
            e_value_size: 0,
        }
    }
}

/// Calculate inline xattr body size
pub fn xattr_ibody_size(xattr_icount: u16) -> usize {
    if xattr_icount == 0 {
        0
    } else {
        size_of::<ErofsXattrIbodyHeader>() + (xattr_icount as usize - 1) * 4
    }
}

/// Align size to xattr entry alignment (4 bytes)
pub fn xattr_align(size: usize) -> usize {
    (size + size_of::<ErofsXattrEntry>() - 1) & !(size_of::<ErofsXattrEntry>() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xattr_ibody_header_size() {
        assert_eq!(size_of::<ErofsXattrIbodyHeader>(), 12);
    }

    #[test]
    fn test_xattr_entry_size() {
        assert_eq!(size_of::<ErofsXattrEntry>(), 4);
    }

    #[test]
    fn test_xattr_ibody_size() {
        assert_eq!(xattr_ibody_size(0), 0);
        assert_eq!(xattr_ibody_size(1), 12);
        assert_eq!(xattr_ibody_size(2), 16);
    }

    #[test]
    fn test_xattr_align() {
        assert_eq!(xattr_align(1), 4);
        assert_eq!(xattr_align(4), 4);
        assert_eq!(xattr_align(5), 8);
        assert_eq!(xattr_align(16), 16);
    }

    #[test]
    fn test_xattr_entry_total_size() {
        let entry = ErofsXattrEntry::new(xattr_index::USER, "test", 10);
        // 4 (header) + 4 (name) + 10 (value) = 18, aligned to 20
        assert_eq!(entry.total_size(), 20);
    }
}
