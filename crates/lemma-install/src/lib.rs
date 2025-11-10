//! Installation functionality for Lemma
//!
//! This crate handles:
//! - Toolchain installation and extraction
//! - Archive format handling (tar.gz, tar.bz2, zip, tar.zst)
//! - Installation verification

pub mod archive;
pub mod install;

// Re-export commonly used types
pub use archive::{extract_archive, extract_tar_zst};
pub use install::Installer;
