//! Strategy template storage and management
//!
//! This module handles loading, saving, and managing strategy templates
//! stored as TOML files in the user's configuration directory.

mod storage;
pub mod handlers;

pub use crate::strategy_types::*;
pub use storage::*;
