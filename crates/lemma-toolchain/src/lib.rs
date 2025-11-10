//! Toolchain management for Lemma
//!
//! This crate handles toolchain-related functionality:
//! - Toolchain descriptor parsing and validation
//! - Project toolchain discovery
//! - Lean version utilities

pub mod toolchain;

// Re-export commonly used types
pub use toolchain::{
    find_project_toolchain, get_lean_version, get_lean_version_or_unknown, ToolchainDesc,
    ToolchainSource, DEFAULT_ORIGIN,
};
