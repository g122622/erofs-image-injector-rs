//! Integration tests for EROFS fuzzer

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use erofs_format::{ErofsSuperBlock, EROFS_SUPER_MAGIC_V1, EROFS_SUPER_OFFSET};
    use erofs_input::ErofsImageInput;
    use erofs_mutator::{ErofsBitflipMutator, ErofsSuperblockMutator};

    #[test]
    fn test_format_sizes() {
        // Verify structure sizes match on-disk format
        assert_eq!(std::mem::size_of::<ErofsSuperBlock>(), 144);
    }

    #[test]
    fn test_input_creation() {
        let data = vec![0u8; 2048];
        let input = ErofsImageInput::new(data.clone());
        assert_eq!(input.data(), data);
        assert_eq!(input.len(), 2048);
    }

    #[test]
    fn test_minimal_image() {
        let mut data = vec![0u8; 4096];
        let magic = EROFS_SUPER_MAGIC_V1.to_le_bytes();
        data[EROFS_SUPER_OFFSET..EROFS_SUPER_OFFSET + 4].copy_from_slice(&magic);

        let mut input = ErofsImageInput::new(data);
        assert!(input.parse_super_block().is_some());

        let sb = input.super_block().unwrap();
        assert_eq!(sb.magic, EROFS_SUPER_MAGIC_V1);
    }

    #[test]
    fn test_superblock_mutator() {
        let mut data = vec![0u8; 4096];
        let magic = EROFS_SUPER_MAGIC_V1.to_le_bytes();
        data[EROFS_SUPER_OFFSET..EROFS_SUPER_OFFSET + 4].copy_from_slice(&magic);

        let input = ErofsImageInput::new(data);
        let injection_points = input.injection_points();
        assert!(!injection_points.is_empty());
    }
}
