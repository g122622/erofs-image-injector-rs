//! File System Harness
//!
//! Harness for exercising erofsfuse through file system operations.

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use tracing::{debug, trace};

/// Result of a file system operation
#[derive(Debug, Clone)]
pub struct OperationResult {
    /// Operation type
    pub op_type: String,
    /// Path operated on
    pub path: String,
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Harness for exercising erofsfuse
pub struct FilesystemHarness {
    /// Maximum depth for directory traversal
    max_depth: usize,

    /// Maximum file size to read
    max_file_size: usize,

    /// Whether to read xattrs
    read_xattrs: bool,

    /// Results of operations
    results: Vec<OperationResult>,
}

impl FilesystemHarness {
    /// Create a new harness
    pub fn new() -> Self {
        Self {
            max_depth: 10,
            max_file_size: 1024 * 1024, // 1 MB
            read_xattrs: true,
            results: Vec::new(),
        }
    }

    /// Set maximum traversal depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set maximum file size
    pub fn with_max_file_size(mut self, size: usize) -> Self {
        self.max_file_size = size;
        self
    }

    /// Enable/disable xattr reading
    pub fn with_xattrs(mut self, enabled: bool) -> Self {
        self.read_xattrs = enabled;
        self
    }

    /// Run the harness on a mount point
    pub fn run(&mut self, mount_point: &Path) -> Vec<OperationResult> {
        self.results.clear();

        // Start by reading root directory
        self.read_directory(mount_point, 0);

        self.results.clone()
    }

    /// Read and traverse a directory
    fn read_directory(&mut self, dir: &Path, depth: usize) {
        if depth >= self.max_depth {
            return;
        }

        trace!("Reading directory: {:?}", dir);
        let dir_path = dir.to_string_lossy().to_string();

        match fs::read_dir(dir) {
            Ok(entries) => {
                self.results.push(OperationResult {
                    op_type: "readdir".to_string(),
                    path: dir_path,
                    success: true,
                    error: None,
                });

                for entry in entries.flatten() {
                    let path = entry.path();
                    let path_str = path.to_string_lossy().to_string();

                    // Try to get metadata
                    match fs::metadata(&path) {
                        Ok(metadata) => {
                            self.results.push(OperationResult {
                                op_type: "stat".to_string(),
                                path: path_str.clone(),
                                success: true,
                                error: None,
                            });

                            if metadata.is_dir() {
                                // Recurse into directory
                                self.read_directory(&path, depth + 1);
                            } else if metadata.is_file() {
                                // Read file contents
                                self.read_file(&path, metadata.len() as usize);
                            } else if metadata.is_symlink() {
                                // Read symlink target
                                self.read_symlink(&path);
                            }
                        }
                        Err(e) => {
                            self.results.push(OperationResult {
                                op_type: "stat".to_string(),
                                path: path_str.clone(),
                                success: false,
                                error: Some(e.to_string()),
                            });
                        }
                    }

                    // Read extended attributes
                    if self.read_xattrs {
                        self.read_xattr(&path);
                    }
                }
            }
            Err(e) => {
                self.results.push(OperationResult {
                    op_type: "readdir".to_string(),
                    path: dir_path,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    /// Read a file's contents
    fn read_file(&mut self, path: &Path, file_size: usize) {
        let path_str = path.to_string_lossy().to_string();

        trace!("Reading file: {:?}", path);

        // Limit read size
        let read_size = std::cmp::min(file_size, self.max_file_size);

        match fs::File::open(path) {
            Ok(mut file) => {
                let mut buffer = vec![0u8; read_size];
                match file.read(&mut buffer) {
                    Ok(bytes_read) => {
                        self.results.push(OperationResult {
                            op_type: "read".to_string(),
                            path: path_str.clone(),
                            success: true,
                            error: None,
                        });

                        // Optionally, try to read more if file is larger
                        if file_size > read_size {
                            // Seek to a random position and read more
                            use std::io::Seek;
                            if let Ok(_) = std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(read_size as u64 / 2)) {
                                let _ = file.read(&mut buffer);
                            }
                        }
                    }
                    Err(e) => {
                        self.results.push(OperationResult {
                            op_type: "read".to_string(),
                            path: path_str.clone(),
                            success: false,
                            error: Some(e.to_string()),
                        });
                    }
                }
            }
            Err(e) => {
                self.results.push(OperationResult {
                    op_type: "open".to_string(),
                    path: path_str.clone(),
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    /// Read a symlink target
    fn read_symlink(&mut self, path: &Path) {
        let path_str = path.to_string_lossy().to_string();

        trace!("Reading symlink: {:?}", path);

        match fs::read_link(path) {
            Ok(target) => {
                self.results.push(OperationResult {
                    op_type: "readlink".to_string(),
                    path: path_str.clone(),
                    success: true,
                    error: None,
                });
                debug!("Symlink {:?} -> {:?}", path, target);
            }
            Err(e) => {
                self.results.push(OperationResult {
                    op_type: "readlink".to_string(),
                    path: path_str.clone(),
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    /// Read extended attributes
    fn read_xattr(&mut self, path: &Path) {
        let path_str = path.to_string_lossy().to_string();

        // Note: Extended attribute support requires platform-specific code
        // For Linux, we could use xattr crate
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            // Try to get file metadata as a simple xattr test
            match fs::metadata(path) {
                Ok(metadata) => {
                    // This exercises stat which may trigger xattr parsing
                    let _ = metadata.ino();
                    let _ = metadata.uid();
                    let _ = metadata.gid();
                }
                Err(e) => {
                    debug!("Failed to get metadata for xattr test: {}", e);
                }
            }
        }
    }

    /// Get the results
    pub fn results(&self) -> &[OperationResult] {
        &self.results
    }

    /// Check if any operations crashed (detected by executor)
    pub fn had_crash(&self) -> bool {
        // This is detected by the executor, not here
        false
    }

    /// Get statistics
    pub fn stats(&self) -> HarnessStats {
        let total = self.results.len();
        let successful = self.results.iter().filter(|r| r.success).count();
        let failed = total - successful;

        HarnessStats {
            total_operations: total,
            successful_operations: successful,
            failed_operations: failed,
        }
    }
}

impl Default for FilesystemHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics from the harness
#[derive(Debug, Clone)]
pub struct HarnessStats {
    /// Total operations performed
    pub total_operations: usize,
    /// Successful operations
    pub successful_operations: usize,
    /// Failed operations
    pub failed_operations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_harness_creation() {
        let harness = FilesystemHarness::new();
        assert_eq!(harness.max_depth, 10);
        assert_eq!(harness.max_file_size, 1024 * 1024);
    }

    #[test]
    fn test_harness_run() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        fs::create_dir_all(temp_dir.path().join("dir1")).unwrap();
        fs::write(temp_dir.path().join("file1.txt"), b"test content").unwrap();
        fs::write(temp_dir.path().join("dir1/file2.txt"), b"nested content").unwrap();

        let mut harness = FilesystemHarness::new();
        let results = harness.run(temp_dir.path());

        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.op_type == "readdir"));
        assert!(results.iter().any(|r| r.op_type == "read"));
    }
}
