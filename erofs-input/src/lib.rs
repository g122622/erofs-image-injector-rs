//! EROFS Image Input Type for LibAFL
//!
//! This crate provides a custom input type for LibAFL that represents
//! EROFS filesystem images with structure-aware mutation support.

#![deny(missing_docs)]

mod erofs_input;
mod targeted_mutation;

pub use erofs_input::*;
pub use targeted_mutation::*;

// Re-export commonly used types from LibAFL
pub use libafl::inputs::Input;
pub use libafl_bolts::Error;
