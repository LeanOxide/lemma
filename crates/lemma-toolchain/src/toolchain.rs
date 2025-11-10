//! Toolchain resolution and utilities
//!
//! This module provides shared functionality for resolving which toolchain
//! to use based on various sources (environment variables, overrides, project
//! files, and defaults).

use anyhow::{Context, Result};
use regex::Regex;
use std::fmt;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// Default origin for Lean toolchains (GitHub repository path)
pub const DEFAULT_ORIGIN: &str = "leanprover/lean4";

/// Cached regex for parsing toolchain descriptors
static TOOLCHAIN_REGEX: OnceLock<Regex> = OnceLock::new();

/// Describes a toolchain, either local or remote
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolchainDesc {
    /// A local/linked toolchain
    Local { name: String },
    /// A remote toolchain from a GitHub repository
    Remote {
        /// The GitHub repository (e.g., "leanprover/lean4")
        origin: String,
        /// The release tag or version (e.g., "v4.25.0-rc2", "stable")
        release: String,
    },
}

impl ToolchainDesc {
    /// Parse a toolchain descriptor from a string
    ///
    /// Supports formats:
    /// - Full: "leanprover/lean4:4.25.0-rc2"
    /// - Short: "stable", "v4.24.0", "4.24.0"
    /// - Local: any name for custom/linked toolchains
    ///
    /// The origin defaults to "leanprover/lean4" if not specified.
    pub fn parse(name: &str) -> Result<Self> {
        let re = TOOLCHAIN_REGEX.get_or_init(|| {
            // Pattern matches: [origin:]release
            // origin format: owner/repo where:
            // - Parts don't start/end with hyphens
            // - No consecutive hyphens or special chars (conflicts with -- and --- separators)
            // release: alphanumeric with single hyphens/periods allowed
            // This prevents conflicts with directory name encoding
            let pattern = r"^(?:([a-zA-Z0-9_]+(?:-[a-zA-Z0-9_]+)*/[a-zA-Z0-9_]+(?:-[a-zA-Z0-9_]+)*)[:])?([a-zA-Z0-9]+(?:[.-][a-zA-Z0-9]+)*)$";
            Regex::new(pattern).unwrap()
        });

        if let Some(captures) = re.captures(name) {
            let origin = captures
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_else(|| DEFAULT_ORIGIN.to_string());

            let mut release = captures.get(2).unwrap().as_str().to_string();

            // Prepend 'v' to releases that start with a number
            if release.starts_with(|c: char| c.is_numeric()) {
                release = format!("v{}", release);
            }

            Ok(Self::Remote { origin, release })
        } else {
            // If it doesn't match the pattern, treat it as a local/custom toolchain
            // This allows arbitrary names for linked toolchains
            Ok(Self::Local {
                name: name.to_string(),
            })
        }
    }

    /// Get the directory name for this toolchain
    ///
    /// For remote toolchains, sanitizes the name by replacing:
    /// - '/' with '--'
    /// - ':' with '---'
    ///
    /// For local toolchains, returns the name as-is
    pub fn to_directory_name(&self) -> String {
        match self {
            Self::Local { name } => name.clone(),
            Self::Remote { .. } => self.to_string().replace('/', "--").replace(':', "---"),
        }
    }

    /// Parse from a directory name (reverse of to_directory_name)
    pub fn from_directory_name(dir_name: &str) -> Result<Self> {
        let name = dir_name.replace("---", ":").replace("--", "/");
        Self::parse(&name)
    }

    /// Get the release version string
    pub fn release(&self) -> &str {
        match self {
            Self::Local { name } => name,
            Self::Remote { release, .. } => release,
        }
    }

    /// Check if this is a local toolchain
    #[allow(dead_code)]
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local { .. })
    }

    /// Check if this toolchain should auto-update (is a tracking channel)
    pub fn is_tracking_channel(&self) -> bool {
        match self {
            Self::Local { .. } => false,
            Self::Remote { release, .. } => {
                matches!(release.as_str(), "stable" | "beta" | "nightly" | "latest")
            }
        }
    }

    /// Get the origin for a remote toolchain, or None for local
    #[allow(dead_code)]
    pub fn origin(&self) -> Option<&str> {
        match self {
            Self::Remote { origin, .. } => Some(origin),
            Self::Local { .. } => None,
        }
    }
}

impl fmt::Display for ToolchainDesc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local { name } => write!(f, "{}", name),
            Self::Remote { origin, release } => write!(f, "{}:{}", origin, release),
        }
    }
}

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
                    // Parse and normalize the toolchain descriptor
                    match ToolchainDesc::parse(toolchain) {
                        Ok(desc) => {
                            return Ok(Some((
                                desc.to_string(),
                                toolchain_file.display().to_string(),
                            )));
                        }
                        Err(e) => {
                            // Log the parse error but continue searching
                            eprintln!(
                                "Warning: Invalid toolchain format in {}: {}",
                                toolchain_file.display(),
                                e
                            );
                        }
                    }
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
                        // Parse and normalize the version from leanpkg.toml
                        match ToolchainDesc::parse(version) {
                            Ok(desc) => {
                                return Ok(Some((
                                    desc.to_string(),
                                    leanpkg_file.display().to_string(),
                                )));
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Invalid lean_version format in {}: {}",
                                    leanpkg_file.display(),
                                    e
                                );
                            }
                        }
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
