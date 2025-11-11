//! Lakefile parsing and data structures
//!
//! This crate provides types and parsers for Lake project configuration files.
//! It supports both `lakefile.toml` and (eventually) `lakefile.lean` formats.

pub mod error;
pub mod parser;
pub mod types;
pub mod validate;

pub use error::{Error, Result};
pub use parser::parse_toml;
pub use types::*;

use std::path::Path;

/// Main entry point for loading a lakefile
///
/// This will automatically detect whether the project uses `lakefile.toml`
/// or `lakefile.lean` and parse accordingly.
pub fn load(project_dir: &Path) -> Result<Lakefile> {
    let toml_path = project_dir.join("lakefile.toml");
    let lean_path = project_dir.join("lakefile.lean");

    if toml_path.exists() {
        let content = std::fs::read_to_string(&toml_path)
            .map_err(|e| Error::IoError(toml_path.clone(), e))?;
        parse_toml(&content)
    } else if lean_path.exists() {
        // TODO: Implement lakefile.lean parsing in Phase 8
        Err(Error::UnsupportedFormat(
            "lakefile.lean parsing is not yet supported. Please use lakefile.toml".to_string(),
        ))
    } else {
        Err(Error::LakefileNotFound(project_dir.to_path_buf()))
    }
}
