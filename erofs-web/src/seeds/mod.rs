//! Seed generation module
//!
//! Provides functionality for generating EROFS seed images from JSON configuration.

mod content_gen;
mod generator;
mod templates;

pub use content_gen::{ContentGenerator, FileHeader};
pub use generator::{SeedGenerator, SeedGenError};
pub use templates::{get_default_templates, get_template_by_id};
