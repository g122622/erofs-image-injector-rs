//! Utility functions for EROFS format handling

use std::io::{Read, Write};

use crate::{ErofsSuperBlock, EROFS_SUPER_OFFSET};

/// EROFS format error types
#[derive(Debug, thiserror::Error)]
pub enum ErofsError {
    /// Invalid magic number
    #[error("Invalid EROFS magic number: expected {expected:#x}, got {actual:#x}")]
    InvalidMagic {
        expected: u32,
        actual: u32,
    },

    /// Invalid super block offset
    #[error("Invalid super block offset: data too small")]
    InvalidOffset,

    /// Invalid block size
    #[error("Invalid block size bits: {0}")]
    InvalidBlockSize(u8),

    /// Invalid checksum
    #[error("Checksum mismatch: expected {expected:#x}, got {actual:#x}")]
    ChecksumMismatch {
        expected: u32,
        actual: u32,
    },

    /// Invalid inode
    #[error("Invalid inode at offset {offset:#x}")]
    InvalidInode {
        offset: u64,
    },

    /// Invalid directory entry
    #[error("Invalid directory entry at offset {offset:#x}")]
    InvalidDirent {
        offset: u64,
    },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),
}

/// Result type for EROFS operations
pub type ErofsResult<T> = Result<T, ErofsError>;

/// Parse an EROFS super block from a reader
pub fn parse_super_block<R: Read>(reader: &mut R) -> ErofsResult<ErofsSuperBlock> {
    // Skip to super block offset
    let mut skip_buf = vec![0u8; EROFS_SUPER_OFFSET];
    reader.read_exact(&mut skip_buf)?;

    // Read super block
    let mut sb_buf = vec![0u8; std::mem::size_of::<ErofsSuperBlock>()];
    reader.read_exact(&mut sb_buf)?;

    // Parse
    ErofsSuperBlock::from_bytes(&sb_buf).ok_or(ErofsError::Parse(
        "Failed to parse super block".to_string(),
    ))
}

/// Write an EROFS super block to a writer
pub fn write_super_block<W: Write>(writer: &mut W, sb: &ErofsSuperBlock) -> ErofsResult<()> {
    // Write padding to super block offset
    let padding = vec![0u8; EROFS_SUPER_OFFSET];
    writer.write_all(&padding)?;

    // Write super block
    let sb_bytes = sb.to_bytes();
    writer.write_all(&sb_bytes)?;

    Ok(())
}

/// Calculate CRC32C checksum for super block validation
pub fn crc32c(data: &[u8]) -> u32 {
    // Use CRC32C polynomial (reversed)
    const CRC32C_POLY: u32 = 0x82F63B78;

    let mut crc = 0xFFFFFFFFu32;
    for byte in data {
        crc ^= *byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ CRC32C_POLY;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Calculate the checksum for a super block (excluding the checksum field itself)
pub fn super_block_checksum(data: &[u8]) -> u32 {
    if data.len() < std::mem::size_of::<ErofsSuperBlock>() {
        return 0;
    }

    // Checksum is calculated over the entire super block except the checksum field
    // The checksum field is at offset 4 (after magic), size 4
    let mut checksum_data = Vec::with_capacity(data.len());

    // Add magic (bytes 0-3)
    checksum_data.extend_from_slice(&data[0..4]);

    // Add rest of super block starting from offset 8 (after checksum field)
    checksum_data.extend_from_slice(&data[8..std::mem::size_of::<ErofsSuperBlock>()]);

    crc32c(&checksum_data)
}

/// Validate block size bits (must be between 9 and 16, typically 12 for 4KB)
pub fn validate_block_size_bits(bits: u8) -> ErofsResult<u32> {
    if bits < 9 || bits > 16 {
        return Err(ErofsError::InvalidBlockSize(bits));
    }
    Ok(1u32 << bits)
}

/// Align size up to the given alignment
pub fn align_up(size: usize, alignment: usize) -> usize {
    (size + alignment - 1) & !(alignment - 1)
}

/// Align size down to the given alignment
pub fn align_down(size: usize, alignment: usize) -> usize {
    size & !(alignment - 1)
}

/// Check if a size is aligned to the given alignment
pub fn is_aligned(size: usize, alignment: usize) -> bool {
    size % alignment == 0
}

/// Convert block address to byte offset
pub fn block_to_offset(block: u64, block_size_bits: u8) -> u64 {
    block << block_size_bits
}

/// Convert byte offset to block address
pub fn offset_to_block(offset: u64, block_size_bits: u8) -> u64 {
    offset >> block_size_bits
}

/// Get inode offset from nid
pub fn nid_to_offset(nid: u64, meta_blkaddr: u32, block_size_bits: u8) -> u64 {
    let meta_offset = (meta_blkaddr as u64) << block_size_bits;
    // Inode offset within block depends on inode format (compact vs extended)
    meta_offset + nid * 32 // Simplified, assumes compact inodes
}

/// Read a u16 from bytes (little-endian)
pub fn read_u16_le(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([bytes[0], bytes[1]])
}

/// Read a u32 from bytes (little-endian)
pub fn read_u32_le(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// Read a u64 from bytes (little-endian)
pub fn read_u64_le(bytes: &[u8]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
    ])
}

/// Write a u16 to bytes (little-endian)
pub fn write_u16_le(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

/// Write a u32 to bytes (little-endian)
pub fn write_u32_le(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// Write a u64 to bytes (little-endian)
pub fn write_u64_le(value: u64) -> [u8; 8] {
    value.to_le_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32c() {
        // Test vector
        let data = b"123456789";
        let checksum = crc32c(data);
        assert_eq!(checksum, 0xE3069283);
    }

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(4, 4), 4);
        assert_eq!(align_up(5, 4), 8);
        assert_eq!(align_up(4095, 4096), 4096);
    }

    #[test]
    fn test_align_down() {
        assert_eq!(align_down(0, 4), 0);
        assert_eq!(align_down(1, 4), 0);
        assert_eq!(align_down(4, 4), 4);
        assert_eq!(align_down(5, 4), 4);
        assert_eq!(align_down(4097, 4096), 4096);
    }

    #[test]
    fn test_is_aligned() {
        assert!(is_aligned(0, 4));
        assert!(is_aligned(4, 4));
        assert!(is_aligned(4096, 4096));
        assert!(!is_aligned(1, 4));
        assert!(!is_aligned(5, 4));
    }

    #[test]
    fn test_block_conversions() {
        assert_eq!(block_to_offset(0, 12), 0);
        assert_eq!(block_to_offset(1, 12), 4096);
        assert_eq!(block_to_offset(2, 12), 8192);

        assert_eq!(offset_to_block(0, 12), 0);
        assert_eq!(offset_to_block(4096, 12), 1);
        assert_eq!(offset_to_block(8192, 12), 2);
        assert_eq!(offset_to_block(4095, 12), 0);
    }

    #[test]
    fn test_validate_block_size_bits() {
        assert!(validate_block_size_bits(9).is_ok()); // 512 bytes
        assert!(validate_block_size_bits(12).is_ok()); // 4096 bytes
        assert!(validate_block_size_bits(16).is_ok()); // 65536 bytes
        assert!(validate_block_size_bits(8).is_err()); // Too small
        assert!(validate_block_size_bits(17).is_err()); // Too large
    }

    #[test]
    fn test_read_write_le() {
        assert_eq!(read_u16_le(&[0x01, 0x02]), 0x0201);
        assert_eq!(read_u32_le(&[0x01, 0x02, 0x03, 0x04]), 0x04030201);
        assert_eq!(
            read_u64_le(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]),
            0x0807060504030201
        );

        assert_eq!(write_u16_le(0x0201), [0x01, 0x02]);
        assert_eq!(write_u32_le(0x04030201), [0x01, 0x02, 0x03, 0x04]);
        assert_eq!(
            write_u64_le(0x0807060504030201),
            [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }
}
