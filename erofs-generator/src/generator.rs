//! EROFS Image Generator
//!
//! Generate EROFS seed images with various configurations.

use std::io::{Cursor, Write};
use std::path::Path;

use erofs_format::{
    align_up, ErofsDataLayout, ErofsFileType, ErofsInodeCompact, ErofsSuperBlock,
    ErofsSuperBlockU1, ErofsDirent, InodeNb, InodeU, RootNidOrBlocksHi,
    EROFS_NAME_LEN, EROFS_SUPER_MAGIC_V1, EROFS_SUPER_OFFSET,
};
use erofs_input::ErofsImageInput;
use tempfile::TempDir;
use tracing::{debug, info};

use crate::{GeneratorError, GeneratorResult, MkfsWrapper};

/// Configuration for image generation
#[derive(Debug, Clone)]
pub struct ImageConfig {
    /// Block size in bytes (must be power of 2)
    pub block_size: u32,

    /// Volume name
    pub volume_name: String,

    /// Number of inodes to pre-allocate
    pub inode_count: u64,

    /// Enable compression
    pub compression: Option<CompressionConfig>,

    /// Root directory mode
    pub root_mode: u16,

    /// Root directory uid
    pub root_uid: u32,

    /// Root directory gid
    pub root_gid: u32,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            block_size: 4096,
            volume_name: "erofs".to_string(),
            inode_count: 1,
            compression: None,
            root_mode: 0o40755,
            root_uid: 0,
            root_gid: 0,
        }
    }
}

/// Compression configuration
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,

    /// Compression level
    pub level: u32,
}

/// Compression algorithm
#[derive(Debug, Clone, Copy)]
pub enum CompressionAlgorithm {
    /// LZ4 compression
    Lz4,
    /// LZ4HC compression (high compression)
    Lz4hc,
}

/// EROFS image generator
pub struct ErofsImageGenerator {
    /// Image configuration
    config: ImageConfig,

    /// mkfs.erofs wrapper (optional)
    mkfs_wrapper: Option<MkfsWrapper>,

    /// Temporary directories for content
    temp_dirs: Vec<TempDir>,
}

impl ErofsImageGenerator {
    /// Create a new generator with default configuration
    pub fn new() -> Self {
        Self {
            config: ImageConfig::default(),
            mkfs_wrapper: None,
            temp_dirs: Vec::new(),
        }
    }

    /// Create a generator with custom configuration
    pub fn with_config(config: ImageConfig) -> Self {
        Self {
            config,
            mkfs_wrapper: None,
            temp_dirs: Vec::new(),
        }
    }

    /// Set the mkfs.erofs wrapper
    pub fn with_mkfs(mut self, mkfs_path: &Path) -> GeneratorResult<Self> {
        self.mkfs_wrapper = Some(MkfsWrapper::new(mkfs_path)?);
        Ok(self)
    }

    /// Generate a minimal valid EROFS image
    pub fn generate_minimal(&self) -> GeneratorResult<ErofsImageInput> {
        info!("Generating minimal EROFS image");

        let block_size = self.config.block_size;
        let block_size_bits = (block_size as f32).log2() as u8;

        // Calculate image size (minimum: superblock + 1 block for root inode)
        let min_size = EROFS_SUPER_OFFSET + std::mem::size_of::<ErofsSuperBlock>() + block_size as usize;
        let mut data = vec![0u8; min_size];

        // Create super block
        let mut sb = ErofsSuperBlock::new();
        sb.magic = EROFS_SUPER_MAGIC_V1;
        sb.blkszbits = block_size_bits;
        sb.blocks_lo = 2; // superblock + root inode block
        sb.meta_blkaddr = 1; // Metadata starts at block 1
        sb.rootnid_or_blocks_hi = RootNidOrBlocksHi { rootnid_2b: 0 };
        sb.inos = 1;
        sb.feature_compat = 0;
        sb.feature_incompat = 0;

        // Set volume name
        let name_bytes = self.config.volume_name.as_bytes();
        let name_len = std::cmp::min(name_bytes.len(), 16);
        sb.volume_name[..name_len].copy_from_slice(&name_bytes[..name_len]);

        // Write super block
        let sb_bytes = sb.to_bytes();
        data[EROFS_SUPER_OFFSET..EROFS_SUPER_OFFSET + sb_bytes.len()]
            .copy_from_slice(&sb_bytes);

        // Create root inode (at block 1, offset 0)
        let inode_offset = block_size as usize; // Block 1
        let mut root_inode = ErofsInodeCompact::new();
        root_inode.i_format = (ErofsDataLayout::FlatPlain as u16) << 1; // Version 0, FlatPlain layout
        root_inode.i_mode = self.config.root_mode;
        root_inode.i_nb = InodeNb { nlink: 2 }; // . and ..
        root_inode.i_size = block_size;
        root_inode.i_mtime = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        root_inode.i_uid = self.config.root_uid as u16;
        root_inode.i_gid = self.config.root_gid as u16;

        let inode_bytes = root_inode.to_bytes();
        data[inode_offset..inode_offset + inode_bytes.len()].copy_from_slice(&inode_bytes);

        debug!("Generated minimal image of {} bytes", data.len());
        Ok(ErofsImageInput::new(data))
    }

    /// Generate an image using mkfs.erofs
    pub fn generate_with_mkfs(&mut self, content_dir: &Path) -> GeneratorResult<ErofsImageInput> {
        let wrapper = self.mkfs_wrapper.as_ref()
            .ok_or_else(|| GeneratorError::MkfsNotFound("mkfs.erofs wrapper not configured".to_string()))?;

        info!("Generating EROFS image using mkfs.erofs from {:?}", content_dir);

        // Create temp file for output
        let output_file = tempfile::NamedTempFile::new()?;
        let output_path = output_file.path();

        // Run mkfs.erofs
        wrapper.create_image(content_dir, output_path)?;

        // Read the generated image
        let data = std::fs::read(output_path)?;
        debug!("Generated image of {} bytes using mkfs.erofs", data.len());

        Ok(ErofsImageInput::new(data))
    }

    /// Generate an image with a simple directory structure
    pub fn generate_simple(&mut self) -> GeneratorResult<ErofsImageInput> {
        if let Some(ref wrapper) = self.mkfs_wrapper {
            // Create temp directory with simple content
            let temp_dir = tempfile::tempdir()?;
            self.temp_dirs.push(temp_dir.try_into()?);

            let content_dir = &self.temp_dirs.last().unwrap();

            // Create a simple directory structure
            std::fs::create_dir_all(content_dir.path().join("dir1"))?;
            std::fs::write(content_dir.path().join("file1.txt"), b"Hello, EROFS!")?;
            std::fs::write(content_dir.path().join("dir1/file2.txt"), b"Nested file")?;

            self.generate_with_mkfs(content_dir.path())
        } else {
            // Fall back to minimal image
            self.generate_minimal()
        }
    }

    /// Generate a corpus of seed images
    pub fn generate_corpus(&mut self, output_dir: &Path, count: usize) -> GeneratorResult<Vec<std::path::PathBuf>> {
        std::fs::create_dir_all(output_dir)?;

        let mut paths = Vec::new();

        for i in 0..count {
            let image = if i == 0 {
                // First image is minimal
                self.generate_minimal()?
            } else {
                // Generate with varying configurations
                let image = self.generate_simple()?;
                image
            };

            let path = output_dir.join(format!("seed_{:04}.erofs", i));
            std::fs::write(&path, image.data())?;
            paths.push(path);
        }

        info!("Generated {} seed images in {:?}", count, output_dir);
        Ok(paths)
    }
}

impl Default for ErofsImageGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a minimal EROFS image directly in memory
pub fn create_minimal_image() -> GeneratorResult<ErofsImageInput> {
    let generator = ErofsImageGenerator::new();
    generator.generate_minimal()
}

/// Create an EROFS image from a directory
pub fn create_image_from_dir(content_dir: &Path, mkfs_path: &Path) -> GeneratorResult<ErofsImageInput> {
    let mut generator = ErofsImageGenerator::new().with_mkfs(mkfs_path)?;
    generator.generate_with_mkfs(content_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_image() {
        let generator = ErofsImageGenerator::new();
        let image = generator.generate_minimal().unwrap();

        assert!(!image.is_empty());
        assert!(image.len() >= EROFS_SUPER_OFFSET);

        // Check magic number
        let data = image.data();
        let magic = u32::from_le_bytes([
            data[EROFS_SUPER_OFFSET],
            data[EROFS_SUPER_OFFSET + 1],
            data[EROFS_SUPER_OFFSET + 2],
            data[EROFS_SUPER_OFFSET + 3],
        ]);
        assert_eq!(magic, EROFS_SUPER_MAGIC_V1);
    }

    #[test]
    fn test_image_config_default() {
        let config = ImageConfig::default();
        assert_eq!(config.block_size, 4096);
        assert_eq!(config.volume_name, "erofs");
        assert!(config.compression.is_none());
    }
}
