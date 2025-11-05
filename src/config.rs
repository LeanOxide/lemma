//! Configuration management for Lemma
//!
//! Like elan and rustup, lemma keeps a minimal settings file containing:
//! - Default toolchain
//! - Directory overrides
//! - Internal flags (e.g., PATH setup tracking)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Main configuration structure (matches elan/rustup simplicity)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Settings file version for future migrations
    #[serde(default = "default_version")]
    pub version: String,

    /// Default toolchain
    pub default_toolchain: Option<String>,

    /// Directory overrides (path -> toolchain)
    #[serde(default)]
    pub overrides: std::collections::HashMap<String, String>,

    /// Whether PATH setup message has been shown
    #[serde(default)]
    pub path_setup_shown: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_version(),
            default_toolchain: None,
            overrides: std::collections::HashMap::new(),
            path_setup_shown: false,
        }
    }
}

fn default_version() -> String {
    "1".to_string()
}

impl Config {
    /// Load configuration from file
    pub fn load() -> Result<Self> {
        let settings_path = Self::settings_path()?;

        let config = if settings_path.exists() {
            let content =
                fs::read_to_string(&settings_path).context("Failed to read settings file")?;
            toml::from_str(&content).context("Failed to parse settings file")?
        } else {
            Self::default()
        };

        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let settings_path = Self::settings_path()?;

        // Ensure parent directory exists
        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent).context("Failed to create lemma directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize settings")?;

        fs::write(&settings_path, content).context("Failed to write settings file")?;

        Ok(())
    }

    /// Get the path to the settings file
    pub fn settings_path() -> Result<PathBuf> {
        let home = Self::lemma_home()?;
        Ok(home.join("settings.toml"))
    }

    /// Get the Lemma home directory
    pub fn lemma_home() -> Result<PathBuf> {
        if let Ok(home) = std::env::var("LEMMA_HOME") {
            return Ok(PathBuf::from(home));
        }

        let home = dirs::home_dir().context("Could not determine home directory")?;

        Ok(home.join(".lemma"))
    }

    /// Get the toolchains directory
    pub fn toolchains_dir() -> Result<PathBuf> {
        Ok(Self::lemma_home()?.join("toolchains"))
    }

    /// Get the temporary directory for downloads and extractions
    pub fn tmp_dir() -> Result<PathBuf> {
        Ok(Self::lemma_home()?.join("tmp"))
    }

    /// Get the update hashes directory for tracking installed versions
    pub fn update_hashes_dir() -> Result<PathBuf> {
        Ok(Self::lemma_home()?.join("update-hashes"))
    }

    /// Set a directory override
    pub fn set_override(&mut self, path: PathBuf, toolchain: String) -> Result<()> {
        let canonical_path = path.canonicalize().context("Failed to canonicalize path")?;
        self.overrides
            .insert(canonical_path.display().to_string(), toolchain);
        Ok(())
    }

    /// Remove a directory override
    pub fn remove_override(&mut self, path: &PathBuf) -> Result<bool> {
        let canonical_path = path.canonicalize().context("Failed to canonicalize path")?;
        Ok(self
            .overrides
            .remove(&canonical_path.display().to_string())
            .is_some())
    }

    /// Find override for a directory by walking up the tree
    pub fn find_override(&self, start_path: &PathBuf) -> Option<(String, String)> {
        let mut current = start_path.as_path();

        loop {
            if let Ok(canonical) = current.canonicalize() {
                let path_str = canonical.display().to_string();
                if let Some(toolchain) = self.overrides.get(&path_str) {
                    return Some((path_str, toolchain.clone()));
                }
            }

            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        None
    }

    /// Save the update hash for a toolchain
    pub fn save_update_hash(toolchain_name: &str, version_hash: &str) -> Result<()> {
        let update_hashes_dir = Self::update_hashes_dir()?;
        fs::create_dir_all(&update_hashes_dir)
            .context("Failed to create update-hashes directory")?;

        let hash_file = update_hashes_dir.join(toolchain_name);
        fs::write(&hash_file, version_hash).context("Failed to write update hash")?;

        Ok(())
    }

    /// Get the update hash for a toolchain
    pub fn get_update_hash(toolchain_name: &str) -> Result<Option<String>> {
        let update_hashes_dir = Self::update_hashes_dir()?;
        let hash_file = update_hashes_dir.join(toolchain_name);

        if hash_file.exists() {
            let content = fs::read_to_string(&hash_file).context("Failed to read update hash")?;
            Ok(Some(content.trim().to_string()))
        } else {
            Ok(None)
        }
    }

    /// Ensure lemma is properly set up (directories, proxy binaries, etc.)
    /// This is called automatically on first use
    pub fn ensure_setup() -> Result<()> {
        // Create all required directories
        let lemma_home = Self::lemma_home()?;
        let bin_dir = lemma_home.join("bin");
        let toolchains_dir = Self::toolchains_dir()?;
        let tmp_dir = Self::tmp_dir()?;
        let update_hashes_dir = Self::update_hashes_dir()?;

        fs::create_dir_all(&bin_dir).context("Failed to create bin directory")?;
        fs::create_dir_all(&toolchains_dir).context("Failed to create toolchains directory")?;
        fs::create_dir_all(&tmp_dir).context("Failed to create tmp directory")?;
        fs::create_dir_all(&update_hashes_dir)
            .context("Failed to create update-hashes directory")?;

        // Install proxy binaries if they don't exist
        Self::ensure_proxy_binaries(&bin_dir)?;

        // Load or create settings
        let mut config = Self::load().unwrap_or_default();

        // Show PATH setup message once
        if !config.path_setup_shown {
            use colored::Colorize;
            println!();
            println!(
                "{} Lemma is setting up for the first time...",
                "=>".green().bold()
            );
            println!();
            println!(
                "{} Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):",
                "Note:".yellow().bold()
            );
            println!("   export PATH=\"{}:$PATH\"", bin_dir.display());
            println!();

            config.path_setup_shown = true;
            config.save()?;
        }

        Ok(())
    }

    /// Ensure proxy binaries are installed
    fn ensure_proxy_binaries(bin_dir: &PathBuf) -> Result<()> {
        // List of tools to proxy
        const PROXY_TOOLS: &[&str] = &[
            "lean",
            "lake",
            "leanpkg",
            "leanchecker",
            "leanc",
            "leanmake",
        ];

        // Get the path to the current lemma executable
        let lemma_exe =
            std::env::current_exe().context("Failed to determine lemma executable path")?;

        for tool in PROXY_TOOLS {
            let tool_path = bin_dir.join(tool);

            // Skip if already exists and is valid
            if tool_path.exists() || tool_path.symlink_metadata().is_ok() {
                continue;
            }

            // Try to create symlink first (preferred), fall back to hardlink
            #[cfg(unix)]
            {
                use std::os::unix::fs::symlink;
                if symlink(&lemma_exe, &tool_path).is_err() {
                    // Symlink failed, try hardlink
                    fs::hard_link(&lemma_exe, &tool_path)
                        .with_context(|| format!("Failed to create link for '{}'", tool))?;
                }
            }

            #[cfg(not(unix))]
            {
                // On non-Unix systems (Windows), try hardlink
                fs::hard_link(&lemma_exe, &tool_path)
                    .with_context(|| format!("Failed to create link for '{}'", tool))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.version, "1");
        assert_eq!(config.default_toolchain, None);
        assert_eq!(config.overrides.len(), 0);
        assert_eq!(config.path_setup_shown, false);
    }

    #[test]
    fn test_serialize_config() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("version"));
        assert!(toml_str.contains("path_setup_shown"));
    }
}
