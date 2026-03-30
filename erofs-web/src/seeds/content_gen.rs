//! File content generation for EROFS seeds
//!
//! Generates file content using various strategies including AFL-style generation.

use rand::{Rng, RngCore, SeedableRng};
use rand::rngs::StdRng;
use sha2::{Digest, Sha256};

use crate::types::{AflContentConfig, EntropyLevel, PatternContentConfig};

/// Content generator for creating file contents
pub struct ContentGenerator {
    rng: StdRng,
}

impl ContentGenerator {
    /// Create a new content generator with a random seed
    pub fn new() -> Self {
        Self {
            rng: StdRng::from_entropy(),
        }
    }

    /// Create a content generator with a specific seed for reproducibility
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    /// Generate text content
    pub fn generate_text(&mut self, content: &str) -> Vec<u8> {
        content.as_bytes().to_vec()
    }

    /// Generate binary content with specified size and entropy
    pub fn generate_binary(&mut self, size_range: (usize, usize), entropy: EntropyLevel) -> Vec<u8> {
        let (min_size, max_size) = size_range;
        let size = if min_size >= max_size {
            min_size
        } else {
            self.rng.gen_range(min_size..=max_size)
        };

        let mut data = vec![0u8; size];
        match entropy {
            EntropyLevel::Low => {
                // Low entropy: mostly zeros with some random bytes
                let non_zero_count = size / 10;
                for _ in 0..non_zero_count {
                    let idx = self.rng.gen_range(0..size);
                    data[idx] = self.rng.gen();
                }
            }
            EntropyLevel::Medium => {
                // Medium entropy: use patterns
                for (i, byte) in data.iter_mut().enumerate() {
                    *byte = ((i as u8).wrapping_mul(self.rng.gen::<u8>())) ^ self.rng.gen::<u8>();
                }
            }
            EntropyLevel::High => {
                // High entropy: fully random
                self.rng.fill_bytes(&mut data);
            }
        }
        data
    }

    /// Generate AFL-style content
    pub fn generate_afl(&mut self, config: &AflContentConfig) -> Vec<u8> {
        let (min_size, max_size) = config.size_range;
        let size = if min_size >= max_size {
            min_size
        } else {
            self.rng.gen_range(min_size..=max_size)
        };

        let mut data = vec![0u8; size];

        // Fill with AFL-style patterns
        self.fill_afl_patterns(&mut data);

        // Inject specific pattern if provided
        if let Some(ref pattern) = config.pattern_injection {
            self.inject_pattern(&mut data, pattern);
        }

        // Add AFL header if requested
        if config.with_header.unwrap_or(false) {
            data = self.add_afl_header(data);
        }

        data
    }

    /// Generate pattern-based content
    pub fn generate_pattern(&mut self, config: &PatternContentConfig) -> Vec<u8> {
        let pattern_bytes = config.pattern.as_bytes();

        if let Some(size) = config.size {
            let mut data = vec![0u8; size];
            for (i, byte) in data.iter_mut().enumerate() {
                *byte = pattern_bytes[i % pattern_bytes.len()];
            }
            data
        } else {
            let repeat_count = config.repeat_count.unwrap_or(1);
            let mut data = Vec::with_capacity(pattern_bytes.len() * repeat_count);
            for _ in 0..repeat_count {
                data.extend_from_slice(pattern_bytes);
            }
            data
        }
    }

    /// Generate content with file header
    pub fn generate_with_header(&mut self, header_type: FileHeader, size_range: (usize, usize)) -> Vec<u8> {
        let header = match header_type {
            FileHeader::Png => vec![
                0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            ],
            FileHeader::Jpeg => vec![0xFF, 0xD8, 0xFF], // JPEG signature
            FileHeader::Gif => vec![0x47, 0x49, 0x46, 0x38], // GIF signature
            FileHeader::Elf => vec![
                0x7F, 0x45, 0x4C, 0x46, // ELF magic
                0x02, // 64-bit
                0x01, // Little endian
                0x01, // ELF version
            ],
            FileHeader::Zip => vec![0x50, 0x4B, 0x03, 0x04], // ZIP signature
            FileHeader::Tar => {
                // TAR header with minimal valid fields
                let mut header = vec![0u8; 512];
                // Name
                header[0..4].copy_from_slice(b"file");
                // Mode
                header[100..108].copy_from_slice(b"0000644");
                // UID
                header[108..116].copy_from_slice(b"0000000");
                // GID
                header[116..124].copy_from_slice(b"0000000");
                // Size (encoded in octal)
                header[124..136].copy_from_slice(b"00000000000");
                // Mtime
                header[136..148].copy_from_slice(b"00000000000");
                // Checksum placeholder
                header[148..156].copy_from_slice(b"        ");
                // Type (regular file)
                header[156] = b'0';
                header
            }
        };

        let (min_size, max_size) = size_range;
        let content_size = if min_size > header.len() {
            if min_size >= max_size {
                min_size - header.len()
            } else {
                self.rng.gen_range(min_size - header.len()..=max_size.saturating_sub(header.len()))
            }
        } else {
            0
        };

        let mut data = header;
        data.extend(self.generate_binary((content_size, content_size), EntropyLevel::Medium));
        data
    }

    /// Fill buffer with AFL-style patterns
    fn fill_afl_patterns(&mut self, data: &mut [u8]) {
        // AFL uses several strategies: dictionary tokens, havoc stages, etc.
        // We'll use a simplified approach with interesting values and patterns

        let strategies: [&[u8]; 2] = [
            // Interesting 8-bit values
            &[0x00u8, 0xFF, 0x7F, 0x80, 0x01, 0x7E, 0x81, 0xFE],
            // Boundary values
            &[0x00, 0x01, 0x7F, 0x80, 0xFF],
        ];

        for chunk in data.chunks_mut(8) {
            if self.rng.gen_bool(0.3) {
                // Use interesting values
                let strategy_idx = self.rng.gen_range(0..strategies.len());
                let strategy = strategies[strategy_idx];
                for (i, byte) in chunk.iter_mut().enumerate() {
                    if i < strategy.len() {
                        *byte = strategy[self.rng.gen_range(0..strategy.len())];
                    }
                }
            } else {
                // Random bytes
                self.rng.fill_bytes(chunk);
            }
        }
    }

    /// Inject a pattern into the data
    fn inject_pattern(&mut self, data: &mut [u8], pattern: &str) {
        let pattern_bytes = pattern.as_bytes();
        if pattern_bytes.is_empty() || data.is_empty() {
            return;
        }

        // Find a random position to inject
        let max_pos = data.len().saturating_sub(pattern_bytes.len());
        if max_pos == 0 {
            return;
        }

        let pos = self.rng.gen_range(0..max_pos);
        data[pos..pos + pattern_bytes.len()].copy_from_slice(pattern_bytes);
    }

    /// Add AFL header to data
    fn add_afl_header(&mut self, mut data: Vec<u8>) -> Vec<u8> {
        // AFL header format: simple marker for fuzzing tools
        let header = vec![
            b'A', b'F', b'L', // Magic
            0x01, // Version
            (data.len() & 0xFF) as u8,
            ((data.len() >> 8) & 0xFF) as u8,
            ((data.len() >> 16) & 0xFF) as u8,
            ((data.len() >> 24) & 0xFF) as u8,
        ];

        let mut result = header;
        result.append(&mut data);
        result
    }

    /// Calculate SHA256 hash of content
    pub fn hash_content(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

impl Default for ContentGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// File header type for generating files with valid headers
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileHeader {
    /// PNG image
    Png,
    /// JPEG image
    Jpeg,
    /// GIF image
    Gif,
    /// ELF executable
    Elf,
    /// ZIP archive
    Zip,
    /// TAR archive
    Tar,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_text() {
        let mut gen = ContentGenerator::with_seed(42);
        let data = gen.generate_text("Hello, EROFS!");
        assert_eq!(data, b"Hello, EROFS!");
    }

    #[test]
    fn test_generate_binary() {
        let mut gen = ContentGenerator::with_seed(42);
        let data = gen.generate_binary((100, 200), EntropyLevel::High);
        assert!(data.len() >= 100 && data.len() <= 200);
    }

    #[test]
    fn test_generate_pattern() {
        let mut gen = ContentGenerator::with_seed(42);
        let config = PatternContentConfig {
            pattern: "ABCD".to_string(),
            repeat_count: Some(3),
            size: None,
        };
        let data = gen.generate_pattern(&config);
        assert_eq!(data, b"ABCDABCDABCD");
    }

    #[test]
    fn test_generate_with_header() {
        let mut gen = ContentGenerator::with_seed(42);
        let data = gen.generate_with_header(FileHeader::Png, (100, 100));
        assert!(data.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
        assert_eq!(data.len(), 100);
    }

    #[test]
    fn test_hash_content() {
        let data = b"test content";
        let hash = ContentGenerator::hash_content(data);
        assert_eq!(hash.len(), 64); // SHA256 produces 64 hex characters
    }
}
