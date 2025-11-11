//! Error types for lakefile parsing

use std::path::PathBuf;
use thiserror::Error;

/// Result type for lakefile operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur when parsing or validating lakefiles
#[derive(Error, Debug)]
pub enum Error {
    /// Lakefile was not found in the project directory
    #[error("No lakefile found in {0}")]
    LakefileNotFound(PathBuf),

    /// IO error reading lakefile
    #[error("Failed to read lakefile at {0}: {1}")]
    IoError(PathBuf, #[source] std::io::Error),

    /// Failed to parse TOML
    #[error("Failed to parse lakefile.toml: {0}")]
    TomlParseError(#[from] toml::de::Error),

    /// Validation error
    #[error("Invalid lakefile: {0}")]
    ValidationError(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid field value
    #[error("Invalid value for field {0}: {1}")]
    InvalidField(String, String),

    /// Duplicate target name
    #[error("Duplicate target name: {0}")]
    DuplicateTarget(String),

    /// Invalid dependency specification
    #[error("Invalid dependency specification: {0}")]
    InvalidDependency(String),

    /// Unsupported lakefile format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Invalid version string
    #[error("Invalid version: {0}")]
    InvalidVersion(#[from] semver::Error),
}
