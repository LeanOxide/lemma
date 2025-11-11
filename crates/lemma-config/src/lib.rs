//! Configuration management for Lemma
//!
//! This crate handles all configuration-related functionality:
//! - Loading and merging configuration from multiple sources
//! - Settings resolution (CLI args + env vars + config files)
//! - Configuration file format and validation
//! - Toolchain resolution logic

pub mod config;
pub mod resolution;
pub mod settings;

// Re-export commonly used types
pub use config::{ColorChoice, Config, GlobalConfig, NetworkConfig, PathsConfig};
pub use resolution::{
    find_tool_binary, resolve_toolchain, resolve_toolchain_or_fail, resolve_toolchain_with_source,
};
pub use settings::{CliArgs, GlobalSettings};
