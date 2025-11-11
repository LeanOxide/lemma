//! Toolchain registry management
//!
//! This module provides a centralized abstraction for managing installed toolchains,
//! including listing, querying, and checking toolchain status.

use anyhow::{Context, Result};
use lemma_toolchain::ToolchainDesc;
use std::fs;
use std::path::{Path, PathBuf};

use crate::Config;

/// Information about an installed toolchain
#[derive(Debug, Clone)]
pub struct InstalledToolchain {
    /// The canonical toolchain name (e.g., "leanprover/lean4:v4.24.0")
    pub name: String,
    /// The directory name on disk (e.g., "v4.24.0-linux")
    pub dir_name: String,
    /// Full path to the toolchain directory
    pub path: PathBuf,
    /// Parsed toolchain descriptor (None if parsing failed)
    pub desc: Option<ToolchainDesc>,
}

/// Manages installed toolchains and their metadata
///
/// The ToolchainRegistry provides a unified interface for:
/// - Listing installed toolchains
/// - Checking toolchain status (active, default, installed)
/// - Getting toolchain paths
///
/// # Example
///
/// ```rust,ignore
/// use lemma_config::{Config, registry::ToolchainRegistry};
///
/// let config = Config::load()?;
/// let registry = ToolchainRegistry::new(&config.lemma_home());
///
/// // List all installed toolchains
/// for tc in registry.list_installed()? {
///     println!("{} at {}", tc.name, tc.path.display());
/// }
///
/// // Check if a specific toolchain is installed
/// let desc = ToolchainDesc::parse("stable")?;
/// if registry.is_installed(&desc)? {
///     println!("stable is installed");
/// }
/// ```
pub struct ToolchainRegistry {
    toolchains_dir: PathBuf,
}

impl ToolchainRegistry {
    /// Create a new toolchain registry
    ///
    /// # Arguments
    ///
    /// * `lemma_home` - The LEMMA_HOME directory (e.g., `~/.lemma`)
    pub fn new(lemma_home: &Path) -> Self {
        Self {
            toolchains_dir: lemma_home.join("toolchains"),
        }
    }

    /// Get the path to the toolchains directory
    pub fn toolchains_dir(&self) -> &Path {
        &self.toolchains_dir
    }

    /// List all installed toolchains
    ///
    /// Returns a vector of installed toolchains, sorted by name.
    /// Temporary directories (ending with `.tmp`) are excluded.
    pub fn list_installed(&self) -> Result<Vec<InstalledToolchain>> {
        if !self.toolchains_dir.exists() {
            return Ok(Vec::new());
        }

        let entries = fs::read_dir(&self.toolchains_dir).with_context(|| {
            format!(
                "Failed to read toolchains directory: {}",
                self.toolchains_dir.display()
            )
        })?;

        let mut toolchains = Vec::new();
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Skip non-directories
            if !path.is_dir() {
                continue;
            }

            let dir_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Skip temp directories
            if dir_name.ends_with(".tmp") {
                continue;
            }

            // Parse the directory name to get the canonical toolchain descriptor
            let (name, desc) = match ToolchainDesc::from_directory_name(&dir_name) {
                Ok(desc) => {
                    let name = desc.to_string();
                    (name, Some(desc))
                }
                Err(_) => (dir_name.clone(), None), // Fallback for unparseable directories
            };

            toolchains.push(InstalledToolchain {
                name,
                dir_name,
                path,
                desc,
            });
        }

        // Sort by canonical name for consistent ordering
        toolchains.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(toolchains)
    }

    /// Get the installation path for a toolchain
    ///
    /// Returns the path where the toolchain would be (or is) installed.
    /// Does not check if the toolchain actually exists.
    pub fn get_path(&self, desc: &ToolchainDesc) -> PathBuf {
        let dir_name = desc.to_directory_name();
        self.toolchains_dir.join(dir_name)
    }

    /// Check if a toolchain is installed
    ///
    /// Returns `true` if the toolchain directory exists and contains a `bin` subdirectory.
    pub fn is_installed(&self, desc: &ToolchainDesc) -> bool {
        let path = self.get_path(desc);
        path.exists() && path.join("bin").exists()
    }

    /// Check if a toolchain is the default
    ///
    /// Compares against the default toolchain stored in config.
    pub fn is_default(&self, desc: &ToolchainDesc, config: &Config) -> bool {
        config
            .default_toolchain
            .as_ref()
            .map(|default| default == &desc.to_string())
            .unwrap_or(false)
    }

    /// Check if a toolchain is currently active
    ///
    /// A toolchain is active if it matches the active toolchain name or
    /// if its path matches the resolved toolchain path.
    ///
    /// # Arguments
    ///
    /// * `desc` - The toolchain descriptor to check
    /// * `active_name` - The name of the currently active toolchain
    pub fn is_active(&self, desc: &ToolchainDesc, active_name: &str) -> bool {
        let name = desc.to_string();

        // Direct name match
        if name == active_name {
            return true;
        }

        // Path-based comparison as fallback
        // Try to resolve the active toolchain and compare canonical paths
        if let Ok(lean_path) = crate::find_tool_binary(active_name, "lean") {
            if let Some(bin_dir) = lean_path.parent() {
                if let Some(tc_path) = bin_dir.parent() {
                    let self_path = self.get_path(desc);
                    if let (Ok(self_canonical), Ok(tc_canonical)) =
                        (self_path.canonicalize(), tc_path.canonicalize())
                    {
                        return self_canonical == tc_canonical;
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_registry_creation() {
        let temp = TempDir::new().unwrap();
        let registry = ToolchainRegistry::new(temp.path());
        assert_eq!(registry.toolchains_dir(), temp.path().join("toolchains"));
    }

    #[test]
    fn test_list_installed_empty() {
        let temp = TempDir::new().unwrap();
        let registry = ToolchainRegistry::new(temp.path());
        let toolchains = registry.list_installed().unwrap();
        assert!(toolchains.is_empty());
    }

    #[test]
    fn test_list_installed_with_toolchains() {
        let temp = TempDir::new().unwrap();
        let toolchains_dir = temp.path().join("toolchains");
        fs::create_dir_all(&toolchains_dir).unwrap();

        // Create mock toolchain directories
        fs::create_dir(toolchains_dir.join("v4.24.0-linux")).unwrap();
        fs::create_dir(toolchains_dir.join("stable-linux")).unwrap();
        fs::create_dir(toolchains_dir.join("v4.23.0-linux.tmp")).unwrap(); // Should be skipped

        let registry = ToolchainRegistry::new(temp.path());
        let toolchains = registry.list_installed().unwrap();

        assert_eq!(toolchains.len(), 2);
        assert!(toolchains.iter().any(|tc| tc.name.contains("4.24.0")));
        assert!(toolchains.iter().any(|tc| tc.name.contains("stable")));
        assert!(!toolchains.iter().any(|tc| tc.name.contains("tmp")));
    }

    #[test]
    fn test_get_path() {
        let temp = TempDir::new().unwrap();
        let registry = ToolchainRegistry::new(temp.path());

        let desc = ToolchainDesc::parse("v4.24.0").unwrap();
        let path = registry.get_path(&desc);

        assert!(path
            .to_string_lossy()
            .contains(&format!("toolchains{}v4.24.0", std::path::MAIN_SEPARATOR)));
    }

    #[test]
    fn test_is_installed() {
        let temp = TempDir::new().unwrap();
        let toolchains_dir = temp.path().join("toolchains");
        fs::create_dir_all(&toolchains_dir).unwrap();

        let desc = ToolchainDesc::parse("v4.24.0").unwrap();
        let tc_path = toolchains_dir.join(desc.to_directory_name());
        fs::create_dir_all(tc_path.join("bin")).unwrap();

        let registry = ToolchainRegistry::new(temp.path());
        assert!(registry.is_installed(&desc));

        let not_installed = ToolchainDesc::parse("v4.23.0").unwrap();
        assert!(!registry.is_installed(&not_installed));
    }
}
