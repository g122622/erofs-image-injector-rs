//! EROFS Seed Image Generator
//!
//! This crate provides utilities for generating EROFS seed images
//! using mkfs.erofs or creating minimal valid images directly.

#![deny(missing_docs)]

mod generator;
mod mkfs_wrapper;

pub use generator::*;
pub use mkfs_wrapper::*;

use std::path::Path;

/// Generator error types
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// mkfs.erofs not found
    #[error("mkfs.erofs not found: {0}")]
    MkfsNotFound(String),

    /// mkfs.erofs failed
    #[error("mkfs.erofs failed: {0}")]
    MkfsFailed(String),

    /// Invalid image
    #[error("Invalid image: {0}")]
    InvalidImage(String),

    /// Template error
    #[error("Template error: {0}")]
    Template(String),
}

/// Result type for generator operations
pub type GeneratorResult<T> = Result<T, GeneratorError>;
