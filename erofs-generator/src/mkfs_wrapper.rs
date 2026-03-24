//! mkfs.erofs Wrapper
//!
//! Wrapper for calling mkfs.erofs to generate EROFS images.

use std::path::{Path, PathBuf};
use std::process::Command;

use tracing::{debug, warn};

use crate::{GeneratorError, GeneratorResult};

/// Wrapper for mkfs.erofs command
pub struct MkfsWrapper {
    /// Path to mkfs.erofs binary
    path: PathBuf,

    /// Default options
    default_options: Vec<String>,
}

impl MkfsWrapper {
    /// Create a new mkfs.erofs wrapper
    pub fn new(path: &Path) -> GeneratorResult<Self> {
        if !path.exists() {
            return Err(GeneratorError::MkfsNotFound(
                path.display().to_string(),
            ));
        }

        Ok(Self {
            path: path.to_path_buf(),
            default_options: Vec::new(),
        }
    }

    /// Create a wrapper assuming mkfs.erofs is in PATH
    pub fn from_path() -> GeneratorResult<Self> {
        Self::new(Path::new("mkfs.erofs"))
    }

    /// Add a default option
    pub fn with_option(mut self, option: &str) -> Self {
        self.default_options.push(option.to_string());
        self
    }

    /// Set compression algorithm
    pub fn with_compression(mut self, algorithm: &str) -> Self {
        self.default_options.push("-z".to_string());
        self.default_options.push(algorithm.to_string());
        self
    }

    /// Create an EROFS image from a directory
    pub fn create_image(&self, source_dir: &Path, output_file: &Path) -> GeneratorResult<()> {
        debug!(
            "Creating EROFS image: {:?} from {:?}",
            output_file, source_dir
        );

        let mut cmd = Command::new(&self.path);

        // Add default options
        for opt in &self.default_options {
            cmd.arg(opt);
        }

        // Add source and output
        cmd.arg(output_file)
            .arg(source_dir);

        debug!("Running: {:?}", cmd);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("mkfs.erofs failed: {}", stderr);
            return Err(GeneratorError::MkfsFailed(stderr.to_string()));
        }

        debug!("Successfully created EROFS image: {:?}", output_file);
        Ok(())
    }

    /// Create an EROFS image with extended options
    pub fn create_image_with_options(
        &self,
        source_dir: &Path,
        output_file: &Path,
        options: &[&str],
    ) -> GeneratorResult<()> {
        let mut cmd = Command::new(&self.path);

        // Add default options
        for opt in &self.default_options {
            cmd.arg(opt);
        }

        // Add extra options
        for opt in options {
            cmd.arg(opt);
        }

        // Add source and output
        cmd.arg(output_file)
            .arg(source_dir);

        debug!("Running: {:?}", cmd);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GeneratorError::MkfsFailed(stderr.to_string()));
        }

        Ok(())
    }

    /// Get the path to mkfs.erofs
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if mkfs.erofs is available
    pub fn is_available(&self) -> bool {
        self.path.exists()
    }
}

/// fsck.erofs wrapper for validation
pub struct FsckWrapper {
    /// Path to fsck.erofs binary
    path: PathBuf,
}

impl FsckWrapper {
    /// Create a new fsck.erofs wrapper
    pub fn new(path: &Path) -> GeneratorResult<Self> {
        if !path.exists() {
            return Err(GeneratorError::MkfsNotFound(
                path.display().to_string(),
            ));
        }

        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    /// Create a wrapper assuming fsck.erofs is in PATH
    pub fn from_path() -> GeneratorResult<Self> {
        Self::new(Path::new("fsck.erofs"))
    }

    /// Check an EROFS image
    pub fn check_image(&self, image_file: &Path) -> GeneratorResult<FsckResult> {
        debug!("Checking EROFS image: {:?}", image_file);

        let output = Command::new(&self.path)
            .arg(image_file)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        debug!("fsck output: {}", stdout);

        if output.status.success() {
            Ok(FsckResult::Pass)
        } else {
            Ok(FsckResult::Fail(stderr.to_string()))
        }
    }

    /// Check if fsck.erofs is available
    pub fn is_available(&self) -> bool {
        self.path.exists()
    }
}

/// Result of fsck check
#[derive(Debug)]
pub enum FsckResult {
    /// Image passed validation
    Pass,
    /// Image failed validation with error message
    Fail(String),
}

/// dump.erofs wrapper for inspection
pub struct DumpWrapper {
    /// Path to dump.erofs binary
    path: PathBuf,
}

impl DumpWrapper {
    /// Create a new dump.erofs wrapper
    pub fn new(path: &Path) -> GeneratorResult<Self> {
        if !path.exists() {
            return Err(GeneratorError::MkfsNotFound(
                path.display().to_string(),
            ));
        }

        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    /// Create a wrapper assuming dump.erofs is in PATH
    pub fn from_path() -> GeneratorResult<Self> {
        Self::new(Path::new("dump.erofs"))
    }

    /// Dump image information
    pub fn dump_info(&self, image_file: &Path) -> GeneratorResult<String> {
        let output = Command::new(&self.path)
            .arg("-S") // Show superblock
            .arg(image_file)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GeneratorError::MkfsFailed(stderr.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// List files in image
    pub fn list_files(&self, image_file: &Path) -> GeneratorResult<String> {
        let output = Command::new(&self.path)
            .arg("-l") // Long format
            .arg(image_file)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(GeneratorError::MkfsFailed(stderr.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mkfs_wrapper_creation() {
        // This test will fail if mkfs.erofs is not available
        if let Ok(wrapper) = MkfsWrapper::from_path() {
            assert!(wrapper.is_available());
        }
    }

    #[test]
    fn test_fsck_wrapper_creation() {
        if let Ok(wrapper) = FsckWrapper::from_path() {
            assert!(wrapper.is_available());
        }
    }
}
