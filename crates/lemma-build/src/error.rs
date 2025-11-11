//! Error types for the build system

use thiserror::Error;

/// Result type for build operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during the build process
#[derive(Error, Debug)]
pub enum Error {
    /// Error loading or parsing lakefile
    #[error("Lakefile error: {0}")]
    Lakefile(#[from] lemma_lakefile::Error),

    /// Dependency graph error
    #[error("Dependency graph error: {0}")]
    Graph(#[from] lemma_graph::Error),

    /// Module resolution error
    #[error("Module resolution error: {0}")]
    ModuleResolution(String),

    /// Build cache error
    #[error("Build cache error: {0}")]
    BuildCache(String),

    /// Compilation error
    #[error("Compilation failed: {0}")]
    Compilation(String),

    /// Linking error
    #[error("Linking failed: {0}")]
    Linking(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error (for cache serialization)
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// General error
    #[error("{0}")]
    Other(String),
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Other(s)
    }
}
