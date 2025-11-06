//! Toolchain resolution and utilities
//!
//! This module provides shared functionality for resolving which toolchain
//! to use based on various sources (environment variables, overrides, project
//! files, and defaults).

use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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

/// Find the path to a tool binary in the specified toolchain
///
/// This function looks for a tool (lean, lake, etc.) in the toolchain directory
/// and returns its absolute path. It handles platform-specific executable naming
/// (e.g., .exe on Windows).
pub fn find_tool_binary(toolchain: &str, tool_name: &str) -> Result<PathBuf> {
    let toolchains_dir = Config::toolchains_dir()?;
    let toolchain_path = toolchains_dir.join(toolchain);

    // Check if toolchain exists
    if !toolchain_path.exists() {
        anyhow::bail!(
            "Toolchain '{}' is not installed.\n\n\
             Install it with: lemma toolchain install {}",
            toolchain,
            toolchain
        );
    }

    // Common locations for tool binaries
    let bin_name = if cfg!(target_os = "windows") {
        format!("{}.exe", tool_name)
    } else {
        tool_name.to_string()
    };

    let candidates = vec![
        toolchain_path.join("bin").join(&bin_name),
        toolchain_path.join(&bin_name),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "Tool '{}' not found in toolchain '{}'.\n\
         Expected location: {}",
        tool_name,
        toolchain,
        toolchain_path.join("bin").join(&bin_name).display()
    )
}

/// Get the Lean version from a toolchain installation
///
/// Runs `lean --version` and returns the version string.
/// Returns an error if the lean binary is not found or the command fails.
pub fn get_lean_version(toolchain_path: &Path) -> Result<String> {
    let lean_bin = toolchain_path.join("bin").join("lean");

    if !lean_bin.exists() {
        anyhow::bail!("lean binary not found at {}", lean_bin.display());
    }

    let output = std::process::Command::new(&lean_bin)
        .arg("--version")
        .output()
        .context("Failed to execute lean --version")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        anyhow::bail!("lean --version failed")
    }
}

/// Get the Lean version from a toolchain installation, with fallback
///
/// Similar to `get_lean_version`, but returns "unknown" instead of an error
/// if the version cannot be determined. Useful for displaying toolchain info.
pub fn get_lean_version_or_unknown(toolchain_path: &Path) -> String {
    let lean_bin = toolchain_path.join("bin").join("lean");

    if !lean_bin.exists() {
        return "unknown".to_string();
    }

    let Ok(output) = std::process::Command::new(&lean_bin)
        .arg("--version")
        .output()
    else {
        return "unknown".to_string();
    };

    if !output.status.success() {
        return "unknown".to_string();
    }

    let Ok(version) = String::from_utf8(output.stdout) else {
        return "unknown".to_string();
    };

    // Parse version from output (usually "Lean (version 4.x.x, ...)")
    if let Some(version) = version.split_whitespace().nth(2) {
        version.trim_end_matches(',').to_string()
    } else {
        version.lines().next().unwrap_or("unknown").to_string()
    }
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
