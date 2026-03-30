//! Seed generator implementation
//!
//! Generates EROFS seed images from JSON configuration.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::NamedTempFile;
use tracing::{debug, info, warn};

use crate::types::{EntropyLevel, *};
use super::content_gen::{ContentGenerator, FileHeader};

/// Error type for seed generation
#[derive(Debug, thiserror::Error)]
pub enum SeedGenError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// mkfs.erofs not found
    #[error("mkfs.erofs not found: {0}")]
    MkfsNotFound(String),

    /// mkfs.erofs failed
    #[error("mkfs.erofs failed: {0}")]
    MkfsFailed(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Content generation error
    #[error("Content generation error: {0}")]
    ContentGen(String),
}

/// Result type for seed generation
pub type SeedGenResult<T> = Result<T, SeedGenError>;

/// Seed generator
pub struct SeedGenerator {
    /// Path to mkfs.erofs
    mkfs_path: PathBuf,
    /// Content generator
    content_gen: ContentGenerator,
    /// Output directory for generated seeds
    output_dir: PathBuf,
}

impl SeedGenerator {
    /// Create a new seed generator
    pub fn new<P: AsRef<Path>>(mkfs_path: P, output_dir: P) -> Self {
        Self {
            mkfs_path: mkfs_path.as_ref().to_path_buf(),
            content_gen: ContentGenerator::new(),
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Create with specific random seed for reproducibility
    pub fn with_seed<P: AsRef<Path>>(mkfs_path: P, output_dir: P, seed: u64) -> Self {
        Self {
            mkfs_path: mkfs_path.as_ref().to_path_buf(),
            content_gen: ContentGenerator::with_seed(seed),
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Generate a single seed from configuration
    pub fn generate(&mut self, name: &str, config: &SeedConfig) -> SeedGenResult<(PathBuf, i64)> {
        // Create temporary directory for content
        let temp_dir = tempfile::tempdir()?;

        // Build directory structure
        self.build_directory_structure(&temp_dir.path(), &config.root)?;

        // Generate EROFS image
        let output_path = self.output_dir.join(format!("{}.erofs", name));

        // Build mkfs.erofs arguments
        let mut args = vec![];

        // Add compression options
        // Format: -zX[,Y] where X is compressor and Y is optional level
        if let Some(ref compression) = config.compression {
            let mut comp_str = format!("-z{}", compression.algorithm.to_string().to_lowercase());
            if let Some(level) = compression.level {
                comp_str.push_str(&format!(",{}", level));
            }
            args.push(comp_str);
        }

        // Add block size (format: -b#)
        args.push(format!("-b{}", config.block_size));

        // Add volume name (format: -L volume-label, max 16 chars)
        if !config.volume_name.is_empty() {
            let label = config.volume_name.chars().take(16).collect::<String>();
            args.push("-L".to_string());
            args.push(label);
        }

        // Output file and source directory
        args.push(output_path.to_string_lossy().to_string());
        args.push(temp_dir.path().to_string_lossy().to_string());

        debug!("Running mkfs.erofs with args: {:?}", args);

        // Run mkfs.erofs
        let output = Command::new(&self.mkfs_path)
            .args(&args)
            .output()
            .map_err(|e| SeedGenError::MkfsNotFound(format!("{}: {}", self.mkfs_path.display(), e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SeedGenError::MkfsFailed(stderr.to_string()));
        }

        // Get file size
        let metadata = fs::metadata(&output_path)?;
        let file_size = metadata.len() as i64;

        info!("Generated seed: {} ({} bytes)", output_path.display(), file_size);

        Ok((output_path, file_size))
    }

    /// Generate multiple seeds from configuration
    pub fn generate_batch(
        &mut self,
        base_name: &str,
        config: &SeedConfig,
        count: usize,
    ) -> SeedGenResult<Vec<(PathBuf, i64)>> {
        let mut results = Vec::new();

        for i in 0..count {
            let name = if count == 1 {
                base_name.to_string()
            } else {
                format!("{}_{:04}", base_name, i)
            };

            let result = self.generate(&name, config)?;
            results.push(result);
        }

        Ok(results)
    }

    /// Build directory structure from configuration
    fn build_directory_structure(
        &mut self,
        base_path: &Path,
        node: &DirectoryTreeNode,
    ) -> SeedGenResult<()> {
        match node.node_type {
            NodeType::Directory => {
                // Create directory
                let dir_path = base_path.join(&node.name);
                fs::create_dir_all(&dir_path)?;

                // Set permissions if specified
                #[cfg(unix)]
                if let Some(mode) = node.mode {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&dir_path, fs::Permissions::from_mode(mode as u32))?;
                }

                // Set xattrs if specified
                if let Some(ref xattrs) = node.xattr {
                    self.set_xattrs(&dir_path, xattrs)?;
                }

                // Process children
                if let Some(ref children) = node.children {
                    for child in children {
                        self.build_directory_structure(&dir_path, child)?;
                    }
                }
            }
            NodeType::File => {
                // Create file
                let file_path = base_path.join(&node.name);

                // Generate content
                let content = if let Some(ref content_config) = node.content {
                    self.generate_file_content(content_config)?
                } else {
                    // Default empty file
                    Vec::new()
                };

                // Use tempfile to write, then set permissions
                let mut file = tempfile::NamedTempFile::new_in(base_path)?;
                file.write_all(&content)?;
                let temp_path = file.path().to_path_buf();

                // Set permissions before persisting
                #[cfg(unix)]
                if let Some(mode) = node.mode {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&temp_path, fs::Permissions::from_mode(mode as u32))?;
                }

                file.persist(&file_path)
                    .map_err(|e| SeedGenError::Io(e.error))?;

                // Set xattrs if specified
                if let Some(ref xattrs) = node.xattr {
                    self.set_xattrs(&file_path, xattrs)?;
                }
            }
            NodeType::Symlink => {
                // Create symlink
                if let Some(ref target) = node.target {
                    let link_path = base_path.join(&node.name);
                    #[cfg(unix)]
                    {
                        std::os::unix::fs::symlink(target, &link_path)?;
                    }
                    #[cfg(windows)]
                    {
                        // Symlinks on Windows require special privileges
                        warn!("Symlinks not fully supported on Windows");
                    }
                } else {
                    warn!("Symlink {} has no target specified", node.name);
                }
            }
        }

        Ok(())
    }

    /// Generate file content from configuration
    fn generate_file_content(
        &mut self,
        config: &FileContentConfig,
    ) -> SeedGenResult<Vec<u8>> {
        match config.content_type {
            FileContentType::Text => {
                let content = config.text_content.clone().unwrap_or_default();
                Ok(self.content_gen.generate_text(&content))
            }
            FileContentType::Binary => {
                let binary_content = config.binary_content.clone().unwrap_or_default();
                // Decode base64
                use base64::{Engine as _, engine::general_purpose};
                general_purpose::STANDARD
                    .decode(&binary_content)
                    .map_err(|e| SeedGenError::ContentGen(format!("Invalid base64: {}", e)))
            }
            FileContentType::AflGenerated => {
                let afl_config = config.afl_config.as_ref()
                    .ok_or_else(|| SeedGenError::ContentGen("AFL config not specified".to_string()))?;
                Ok(self.content_gen.generate_afl(afl_config))
            }
            FileContentType::Random => {
                let random_config = config.random_config.as_ref()
                    .ok_or_else(|| SeedGenError::ContentGen("Random config not specified".to_string()))?;
                Ok(self.content_gen.generate_binary(
                    random_config.size_range,
                    random_config.entropy.unwrap_or(EntropyLevel::Medium),
                ))
            }
            FileContentType::Pattern => {
                let pattern_config = config.pattern_config.as_ref()
                    .ok_or_else(|| SeedGenError::ContentGen("Pattern config not specified".to_string()))?;
                Ok(self.content_gen.generate_pattern(pattern_config))
            }
        }
    }

    /// Set extended attributes on a path
    fn set_xattrs(&self, path: &Path, xattrs: &[ExtendedAttribute]) -> SeedGenResult<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs as unix_fs;
            use base64::{Engine as _, engine::general_purpose};

            for xattr in xattrs {
                let value = general_purpose::STANDARD
                    .decode(&xattr.value)
                    .unwrap_or_else(|_| xattr.value.as_bytes().to_vec());

                // Use xattr crate if available, otherwise skip
                // For now, we'll log a warning
                debug!("Setting xattr {} on {:?} ({} bytes)", xattr.name, path, value.len());
                // xattr::set(path, &xattr.name, &value)
                //     .map_err(|e| SeedGenError::Io(e))?;
            }
        }
        #[cfg(not(unix))]
        {
            warn!("Extended attributes not supported on this platform");
        }
        Ok(())
    }

    /// Check if mkfs.erofs is available
    pub fn is_mkfs_available(&self) -> bool {
        self.mkfs_path.exists() || Command::new(&self.mkfs_path)
            .arg("--version")
            .output()
            .is_ok()
    }
}

/// Generate a seed with file header
pub fn generate_file_with_header(
    header_type: FileHeader,
    size_range: (usize, usize),
) -> Vec<u8> {
    let mut gen = ContentGenerator::new();
    gen.generate_with_header(header_type, size_range)
}

/// Hash file content
pub fn hash_content(data: &[u8]) -> String {
    ContentGenerator::hash_content(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directory_tree_node_serialization() {
        let node = DirectoryTreeNode {
            name: "test".to_string(),
            node_type: NodeType::Directory,
            content: None,
            children: Some(vec![]),
            xattr: None,
            mode: Some(0o755),
            uid: Some(0),
            gid: Some(0),
            target: None,
        };

        let json = serde_json::to_string(&node).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("directory"));
    }

    #[test]
    fn test_seed_config_default() {
        let config = SeedConfig::default();
        assert_eq!(config.block_size, 4096);
        assert_eq!(config.volume_name, "erofs");
    }
}
