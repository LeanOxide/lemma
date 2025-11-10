//! Download and caching functionality for Lemma
//!
//! This crate handles:
//! - Downloading toolchains from release servers
//! - HTTP client management
//! - Release channel resolution

pub mod download;
pub mod release;

// Re-export commonly used types
pub use download::DownloadClient;
pub use release::{Release, ReleaseServerClient};
