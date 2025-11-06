//! Toolchain resolution and utilities
//!
//! This module provides shared functionality for resolving which toolchain
//! to use based on various sources (environment variables, overrides, project
//! files, and defaults).

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::Path;

use crate::config::Config;

/// Source of the active toolchain
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainSource {
    /// Explicit command-line override (e.g., +toolchain syntax)
    Explicit,
    /// Environment variable LEMMA_TOOLCHAIN
    Environment,
    /// Directory override set via `lemma override set`
    Override(String),
    /// Project file (lean-toolchain or leanpkg.toml)
    ProjectFile(String),
    /// Default toolchain from config
    Default,
}

impl std::fmt::Display for ToolchainSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Explicit => write!(f, "explicit command-line override"),
            Self::Environment => write!(f, "environment variable LEMMA_TOOLCHAIN"),
            Self::Override(path) => write!(f, "directory override at {}", path),
            Self::ProjectFile(path) => write!(f, "project file at {}", path),
            Self::Default => write!(f, "default setting"),
        }
    }
}

/// Resolve the active toolchain with detailed source information
///
/// Returns the toolchain name and the source where it was found
pub fn resolve_toolchain_with_source(
    explicit: Option<&str>,
) -> Result<Option<(String, ToolchainSource)>> {
    // 1. Explicit command-line override
    if let Some(toolchain) = explicit {
        return Ok(Some((toolchain.to_string(), ToolchainSource::Explicit)));
    }

    // 2. Environment variable override
    if let Ok(toolchain) = env::var("LEMMA_TOOLCHAIN") {
        return Ok(Some((toolchain, ToolchainSource::Environment)));
    }

    // Load config once for both override and default checks
    let config = Config::load().context("Failed to load configuration")?;

    // 3. Directory override (walks up from current directory)
    if let Ok(current_dir) = env::current_dir() {
        if let Some((path, toolchain)) = config.find_override(&current_dir) {
            return Ok(Some((toolchain, ToolchainSource::Override(path))));
        }
    }

    // 4. Project-specific configuration files
    if let Ok(current_dir) = env::current_dir() {
        if let Some((toolchain, path)) = find_project_toolchain(&current_dir)? {
            return Ok(Some((toolchain, ToolchainSource::ProjectFile(path))));
        }
    }

    // 5. Default toolchain from config
    if let Some(default) = config.default_toolchain {
        return Ok(Some((default, ToolchainSource::Default)));
    }

    Ok(None)
}

/// Resolve the active toolchain, returning just the name
///
/// This is a convenience wrapper around `resolve_toolchain_with_source`
pub fn resolve_toolchain(explicit: Option<&str>) -> Result<Option<String>> {
    Ok(resolve_toolchain_with_source(explicit)?.map(|(name, _)| name))
}

/// Resolve the active toolchain, returning an error if none is found
///
/// This is useful for commands that require an active toolchain
pub fn resolve_toolchain_or_fail(explicit: Option<&str>) -> Result<String> {
    resolve_toolchain(explicit)?.ok_or_else(|| {
        anyhow::anyhow!(
            "No active toolchain found.\n\n\
             Set a default with: lemma default <toolchain>\n\
             Or install a toolchain with: lemma toolchain install stable"
        )
    })
}

/// Find project-specific toolchain configuration by walking up the directory tree
///
/// Returns the toolchain name and the path to the file where it was found
pub fn find_project_toolchain(start_dir: &Path) -> Result<Option<(String, String)>> {
    let mut current = start_dir;

    loop {
        // Check for lean-toolchain file
        let toolchain_file = current.join("lean-toolchain");
        if toolchain_file.exists() {
            if let Ok(contents) = fs::read_to_string(&toolchain_file) {
                let toolchain = contents.trim();
                if !toolchain.is_empty() {
                    return Ok(Some((
                        toolchain.to_string(),
                        toolchain_file.display().to_string(),
                    )));
                }
            }
        }

        // Check for leanpkg.toml with lean_version
        let leanpkg_file = current.join("leanpkg.toml");
        if leanpkg_file.exists() {
            if let Ok(contents) = fs::read_to_string(&leanpkg_file) {
                // Parse TOML properly to extract lean_version
                if let Ok(parsed) = toml::from_str::<toml::Value>(&contents) {
                    if let Some(version) = parsed.get("lean_version").and_then(|v| v.as_str()) {
                        return Ok(Some((
                            version.to_string(),
                            leanpkg_file.display().to_string(),
                        )));
                    }
                }
            }
        }

        // Move up to parent directory
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_toolchain_source_display() {
        assert_eq!(
            ToolchainSource::Environment.to_string(),
            "environment variable LEMMA_TOOLCHAIN"
        );
        assert_eq!(
            ToolchainSource::Override("/path/to/dir".to_string()).to_string(),
            "directory override at /path/to/dir"
        );
        assert_eq!(
            ToolchainSource::ProjectFile("/path/to/lean-toolchain".to_string()).to_string(),
            "project file at /path/to/lean-toolchain"
        );
        assert_eq!(ToolchainSource::Default.to_string(), "default setting");
    }

    #[test]
    fn test_find_project_toolchain_leanpkg() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a leanpkg.toml with lean_version
        let leanpkg_path = temp_path.join("leanpkg.toml");
        let mut file = fs::File::create(&leanpkg_path).unwrap();
        writeln!(file, r#"lean_version = "v4.5.0""#).unwrap();
        writeln!(file, r#"name = "test-package""#).unwrap();

        // Should find the version
        let result = find_project_toolchain(temp_path).unwrap();
        assert!(result.is_some());
        let (version, path) = result.unwrap();
        assert_eq!(version, "v4.5.0");
        assert!(path.contains("leanpkg.toml"));
    }

    #[test]
    fn test_find_project_toolchain_lean_toolchain() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a lean-toolchain file
        let toolchain_path = temp_path.join("lean-toolchain");
        fs::write(&toolchain_path, "stable\n").unwrap();

        // Should find the version
        let result = find_project_toolchain(temp_path).unwrap();
        assert!(result.is_some());
        let (version, path) = result.unwrap();
        assert_eq!(version, "stable");
        assert!(path.contains("lean-toolchain"));
    }

    #[test]
    fn test_find_project_toolchain_with_escapes() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a leanpkg.toml with escaped characters (proper TOML parsing test)
        let leanpkg_path = temp_path.join("leanpkg.toml");
        let mut file = fs::File::create(&leanpkg_path).unwrap();
        // TOML with a version containing a quote (properly escaped)
        writeln!(file, r#"lean_version = "v4.5.0-\"beta\"" "#).unwrap();

        // Should properly parse the escaped string
        let result = find_project_toolchain(temp_path).unwrap();
        assert!(result.is_some());
        let (version, _) = result.unwrap();
        assert_eq!(version, r#"v4.5.0-"beta""#);
    }
}
