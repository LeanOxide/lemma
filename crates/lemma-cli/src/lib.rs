//! CLI argument definitions for Lemma
//!
//! This crate contains all CLI interface definitions including:
//! - Command-line argument structures (clap-based)
//! - Help text and documentation
//! - No business logic - pure interface definitions

pub mod cli;
pub mod help;

// Re-export commonly used types
pub use cli::{Cli, Commands, GlobalArgs, OverrideCommands, SelfCommands, ToolchainCommands};
