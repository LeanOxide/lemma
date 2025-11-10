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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // Property-based tests using proptest
    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        // Strategy for generating valid toolchain names
        fn toolchain_name_strategy() -> impl Strategy<Value = String> {
            prop_oneof![
                // Tracking channels
                Just("stable".to_string()),
                Just("nightly".to_string()),
                Just("beta".to_string()),
                Just("latest".to_string()),
                // Version numbers
                "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}".prop_map(|s| s),
                "v[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}".prop_map(|s| s),
                // With origin: single hyphens/periods only, no consecutive special chars
                "[a-zA-Z0-9_]+(-[a-zA-Z0-9_]+)*/[a-zA-Z0-9_]+(-[a-zA-Z0-9_]+)*:[a-zA-Z0-9]+([.-][a-zA-Z0-9]+)*".prop_map(|s| s),
            ]
        }

        proptest! {
            /// Property: Parsing and displaying should roundtrip for valid remote toolchains
            #[test]
            fn test_parse_display_roundtrip_remote(version in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}") {
                let input = format!("leanprover/lean4:{}", version);
                let parsed = ToolchainDesc::parse(&input).unwrap();
                let displayed = parsed.to_string();

                // The displayed version might have 'v' prefix added
                let expected = format!("leanprover/lean4:v{}", version);
                prop_assert_eq!(displayed, expected);
            }

            /// Property: Directory name conversion should be reversible
            #[test]
            fn test_directory_name_roundtrip(
                // Generate origins with single hyphens only, no consecutive hyphens
                origin in "[a-zA-Z0-9_]+(-[a-zA-Z0-9_]+)*/[a-zA-Z0-9_]+(-[a-zA-Z0-9_]+)*",
                // Release with single hyphens/periods, no consecutive special chars
                release in "[a-zA-Z0-9]+([.-][a-zA-Z0-9]+)*"
            ) {
                let toolchain = format!("{}:{}", origin, release);
                let desc = ToolchainDesc::parse(&toolchain).unwrap();
                let dir_name = desc.to_directory_name();
                let parsed_back = ToolchainDesc::from_directory_name(&dir_name).unwrap();

                // The roundtrip should preserve the toolchain
                prop_assert_eq!(desc.to_string(), parsed_back.to_string());
            }

            /// Property: Parsing should never panic on arbitrary strings
            #[test]
            fn test_parse_never_panics(input in "\\PC{0,100}") {
                // This should never panic, even on invalid input
                let _ = ToolchainDesc::parse(&input);
            }

            /// Property: Tracking channels should be recognized correctly
            #[test]
            fn test_tracking_channels(channel in "(stable|nightly|beta|latest)") {
                let desc = ToolchainDesc::parse(&channel).unwrap();
                prop_assert!(desc.is_tracking_channel());
            }

            /// Property: Version-specific toolchains should not be tracking channels
            #[test]
            fn test_version_not_tracking(version in "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}") {
                let desc = ToolchainDesc::parse(&version).unwrap();
                prop_assert!(!desc.is_tracking_channel());
            }

            /// Property: All valid toolchain names should parse successfully
            #[test]
            fn test_valid_names_parse(name in toolchain_name_strategy()) {
                let result = ToolchainDesc::parse(&name);
                prop_assert!(result.is_ok());
            }
        }
    }

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
        assert_eq!(version, "leanprover/lean4:v4.5.0");
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
        assert_eq!(version, "leanprover/lean4:stable");
        assert!(path.contains("lean-toolchain"));
    }

    #[test]
    fn test_find_project_toolchain_with_origin() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a lean-toolchain file with full origin:release format
        let toolchain_path = temp_path.join("lean-toolchain");
        fs::write(&toolchain_path, "leanprover/lean4:4.25.0-rc2\n").unwrap();

        // Should find and normalize the version
        let result = find_project_toolchain(temp_path).unwrap();
        assert!(result.is_some());
        let (version, path) = result.unwrap();
        assert_eq!(version, "leanprover/lean4:v4.25.0-rc2");
        assert!(path.contains("lean-toolchain"));
    }

    #[test]
    fn test_toolchain_desc_parse_full_format() {
        let desc = ToolchainDesc::parse("leanprover/lean4:4.25.0").unwrap();
        assert_eq!(desc.to_string(), "leanprover/lean4:v4.25.0");
        assert_eq!(desc.release(), "v4.25.0");
    }

    #[test]
    fn test_toolchain_desc_parse_short_format() {
        let desc = ToolchainDesc::parse("stable").unwrap();
        assert_eq!(desc.to_string(), "leanprover/lean4:stable");
        assert_eq!(desc.release(), "stable");
    }

    #[test]
    fn test_toolchain_desc_to_directory_name() {
        let desc = ToolchainDesc::parse("leanprover/lean4:4.25.0").unwrap();
        assert_eq!(desc.to_directory_name(), "leanprover--lean4---v4.25.0");
    }

    #[test]
    fn test_toolchain_desc_from_directory_name() {
        let desc = ToolchainDesc::from_directory_name("leanprover--lean4---v4.25.0").unwrap();
        assert_eq!(desc.to_string(), "leanprover/lean4:v4.25.0");
    }

    #[test]
    fn test_toolchain_desc_local() {
        // Use a name with characters that don't match the regex pattern
        let desc = ToolchainDesc::parse("my@custom#toolchain").unwrap();
        assert!(desc.is_local());
        assert_eq!(desc.to_string(), "my@custom#toolchain");
        assert_eq!(desc.to_directory_name(), "my@custom#toolchain");
    }

    #[test]
    fn test_toolchain_desc_simple_name() {
        // Simple alphanumeric names are treated as Remote with default origin
        let desc = ToolchainDesc::parse("my-custom-toolchain").unwrap();
        assert!(!desc.is_local());
        assert_eq!(desc.to_string(), "leanprover/lean4:my-custom-toolchain");
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
        // Note: The quotes don't match the regex, so it's treated as a Local toolchain
        assert_eq!(version, r#"v4.5.0-"beta""#);
    }

    #[test]
    fn test_extract_version_from_lean_output() {
        let output1 = "Lean (version 4.24.0, commit 6099ec08, Release)";
        assert_eq!(
            extract_version_from_lean_output(output1),
            Some("4.24.0".to_string())
        );

        let output2 = "Lean (version 4.25.0-rc2, commit abc123, Release)";
        assert_eq!(
            extract_version_from_lean_output(output2),
            Some("4.25.0-rc2".to_string())
        );

        let output3 = "invalid output";
        assert_eq!(extract_version_from_lean_output(output3), None);
    }
}
