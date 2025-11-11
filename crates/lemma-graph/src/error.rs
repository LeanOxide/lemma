//! Error types for graph operations

use thiserror::Error;

/// Result type for graph operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during graph operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Error {
    /// A cycle was detected in the dependency graph
    #[error("Cyclic dependency detected: {0}")]
    CyclicDependency(String),

    /// A node was not found in the graph
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// Attempted to add a duplicate node
    #[error("Duplicate node: {0}")]
    DuplicateNode(String),

    /// Invalid graph operation
    #[error("Invalid graph operation: {0}")]
    InvalidOperation(String),
}
