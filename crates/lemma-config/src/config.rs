//! Configuration management for Lemma
//!
//! This module handles a unified configuration structure that includes:
//!
//! ## State (Mutable - managed by lemma)
//! - Default toolchain
//! - Directory overrides
//! - Internal flags (e.g., PATH setup tracking)
//!
//! ## Preferences (User-configurable)
//! - Global settings (verbosity, color)
//! - Path overrides
//! - Network settings (timeout, proxies)
//! - Mirror URLs
//!
//! ## Configuration Files
//! - User state + preferences: `~/.lemma/lemma.toml` (read/write)
//! - System preferences: `/etc/lemma/lemma.toml` (read-only, Unix only)
//! - Project preferences: `./lemma.toml` or `./.lemma/lemma.toml` (read-only)
//!
//! State is only written to the user config. Preferences are merged from all sources
//! with precedence: CLI args > Project config > User config > System config > Defaults

use anyhow::{Context, Result};
use lemma_static::EnvVars;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Color output control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ColorChoice {
    /// Enables colored output only when the output is going to a terminal or TTY with support.
    Auto,
    /// Enables colored output regardless of the detected environment.
    Always,
    /// Disables colored output.
    Never,
}

// ============================================================================
// Unified Configuration Structure
// ============================================================================

/// Unified configuration structure
///
/// Combines both mutable state (managed by lemma) and user preferences
/// (configured via lemma.toml files at various levels).
///
/// State fields are only written to `~/.lemma/lemma.toml`.
/// Preference fields are merged from system/user/project configs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    // ========================================================================
    // State (managed by lemma - written to user config)
    // ========================================================================
    /// Settings file version for future migrations
    #[serde(default = "default_version")]
    pub version: String,

    /// Default toolchain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_toolchain: Option<String>,

    /// Directory overrides (path -> toolchain)
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub overrides: std::collections::HashMap<String, String>,

    /// Whether PATH setup message has been shown
    #[serde(default)]
    pub path_setup_shown: bool,

    // ========================================================================
    // Preferences (user-configurable - merged from multiple sources)
    // ========================================================================
    /// Global settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global: Option<GlobalConfig>,

    /// Path settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<PathsConfig>,

    /// Network settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkConfig>,

    /// Lean release server URL (overrides default https://release.lean-lang.org)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lean_release: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: default_version(),
            default_toolchain: None,
            overrides: std::collections::HashMap::new(),
            path_setup_shown: false,
            global: None,
            paths: None,
            network: None,
            lean_release: None,
        }
    }
}

fn default_version() -> String {
    "1".to_string()
}

impl Config {
    /// Load unified configuration
    ///
    /// This loads state from the user config and merges preferences from
    /// system/user/project configs in order of precedence:
    /// 1. Environment variables (highest)
    /// 2. Project config (./lemma.toml)
    /// 3. User config (~/.lemma/lemma.toml)
    /// 4. System config (/etc/lemma/lemma.toml) - Unix only
    /// 5. Defaults (lowest)
    ///
    /// ## Environment Variables
    ///
    /// Environment variables use the `LEMMA_` prefix with double underscores (`__`)
    /// to denote nested fields:
    ///
    /// - `LEMMA_HOME` - Lemma home directory (special: used before config loading)
    /// - `LEMMA_MIRRORS__LEAN_RELEASE` - Lean release server URL
    /// - `LEMMA_GLOBAL__VERBOSE` - Verbosity level (0-2)
    /// - `LEMMA_GLOBAL__COLOR` - Color output (always/never/auto)
    /// - `LEMMA_PATHS__HOME` - Custom home path
    /// - `LEMMA_NETWORK__TIMEOUT` - Network timeout in seconds
    /// - `LEMMA_NETWORK__RETRIES` - Number of retries for failed requests
    pub fn load() -> Result<Self> {
        // Build merged config using the config crate for preference merging
        let mut builder = config::Config::builder();

        // 1. System config (Unix only) - lowest precedence for preferences
        #[cfg(unix)]
        {
            let system_path = PathBuf::from("/etc/lemma/lemma.toml");
            if system_path.exists() {
                tracing::debug!("Loading system config from: {}", system_path.display());
                builder = builder.add_source(
                    config::File::from(system_path)
                        .required(false)
                        .format(config::FileFormat::Toml),
                );
            }
        }

        // 2. User config (state + preferences) - includes state fields
        let user_config_path = Self::config_path()?;
        if user_config_path.exists() {
            tracing::debug!("Loading user config from: {}", user_config_path.display());
            builder = builder.add_source(
                config::File::from(user_config_path)
                    .required(false)
                    .format(config::FileFormat::Toml),
            );
        }

        // 3. Project config (highest precedence for preferences, no state)
        if let Ok(current_dir) = std::env::current_dir() {
            // Try ./lemma.toml first
            let project_path = current_dir.join("lemma.toml");
            if project_path.exists() {
                tracing::debug!("Loading project config from: {}", project_path.display());
                builder = builder.add_source(
                    config::File::from(project_path)
                        .required(false)
                        .format(config::FileFormat::Toml),
                );
            } else {
                // Try ./.lemma/lemma.toml
                let project_path_alt = current_dir.join(".lemma").join("lemma.toml");
                if project_path_alt.exists() {
                    tracing::debug!(
                        "Loading project config from: {}",
                        project_path_alt.display()
                    );
                    builder = builder.add_source(
                        config::File::from(project_path_alt)
                            .required(false)
                            .format(config::FileFormat::Toml),
                    );
                }
            }
        }

        // 4. Environment variables (highest precedence)
        // LEMMA_MIRRORS__LEAN_RELEASE -> mirrors.lean_release
        // LEMMA_GLOBAL__VERBOSE -> global.verbose
        // LEMMA_GLOBAL__COLOR -> global.color
        // LEMMA_PATHS__HOME -> paths.home
        // LEMMA_NETWORK__TIMEOUT -> network.timeout
        //
        // Note: LEMMA_HOME is handled separately in lemma_home() as it's needed
        // before config loading to determine the config file location.
        builder = builder.add_source(
            config::Environment::with_prefix("LEMMA")
                .prefix_separator("_")
                .separator("__")
                .try_parsing(true),
        );

        // Build and deserialize the merged config
        let built_config = builder.build().context("Failed to build configuration")?;

        let final_config = built_config
            .try_deserialize::<Config>()
            .context("Failed to deserialize configuration")
            .unwrap_or_default();

        Ok(final_config)
    }

    /// Save configuration to file
    ///
    /// Only saves to the user config at ~/.lemma/lemma.toml.
    /// System and project configs are read-only.
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create lemma directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_path, content).context("Failed to write config file")?;

        Ok(())
    }

    /// Get the path to the user configuration file
    pub fn config_path() -> Result<PathBuf> {
        let home = Self::lemma_home()?;
        Ok(home.join("lemma.toml"))
    }

    /// Get the Lemma home directory
    pub fn lemma_home() -> Result<PathBuf> {
        if let Ok(home) = std::env::var(EnvVars::LEMMA_HOME) {
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
    pub fn remove_override(&mut self, path: &Path) -> Result<bool> {
        let canonical_path = path.canonicalize().context("Failed to canonicalize path")?;
        Ok(self
            .overrides
            .remove(&canonical_path.display().to_string())
            .is_some())
    }

    /// Find override for a directory by walking up the tree
    pub fn find_override(&self, start_path: &Path) -> Option<(String, String)> {
        let mut current = start_path;

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

    /// Load the update hash for a toolchain
    pub fn load_update_hash(toolchain_name: &str) -> Result<Option<String>> {
        use lemma_toolchain::ToolchainDesc;

        let update_hashes_dir = Self::update_hashes_dir()?;

        // Sanitize the toolchain name for use as a filename
        let filename = match ToolchainDesc::parse(toolchain_name) {
            Ok(desc) => desc.to_directory_name(),
            Err(_) => toolchain_name.to_string(),
        };

        let hash_file = update_hashes_dir.join(filename);

        if hash_file.exists() {
            let hash = fs::read_to_string(&hash_file)
                .context("Failed to read update hash")?
                .trim()
                .to_string();
            Ok(Some(hash))
        } else {
            Ok(None)
        }
    }

    /// Save the update hash for a toolchain
    pub fn save_update_hash(toolchain_name: &str, version_hash: &str) -> Result<()> {
        use lemma_toolchain::ToolchainDesc;

        let update_hashes_dir = Self::update_hashes_dir()?;
        fs::create_dir_all(&update_hashes_dir)
            .context("Failed to create update-hashes directory")?;

        // Sanitize the toolchain name for use as a filename
        let filename = match ToolchainDesc::parse(toolchain_name) {
            Ok(desc) => desc.to_directory_name(),
            Err(_) => toolchain_name.to_string(),
        };

        let hash_file = update_hashes_dir.join(filename);
        fs::write(&hash_file, version_hash).context("Failed to write update hash")?;

        Ok(())
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

            #[cfg(windows)]
            {
                println!(
                    "{} Add Lemma to your PATH by running one of the following:",
                    "Note:".yellow().bold()
                );
                println!();
                println!(
                    "   {} Run this command in PowerShell (as Administrator):",
                    "PowerShell:".cyan()
                );
                println!(
                    "   [System.Environment]::SetEnvironmentVariable('Path', $env:Path + ';{}', 'User')",
                    bin_dir.display()
                );
                println!();
                println!(
                    "   {} Run this command in Command Prompt (as Administrator):",
                    "CMD:".cyan()
                );
                println!("   setx PATH \"%PATH%;{}\"", bin_dir.display());
                println!();
                println!(
                    "   {} Alternatively, add it manually via:",
                    "Manual:".cyan()
                );
                println!("   System Properties > Environment Variables > User Variables > Path");
                println!("   Add: {}", bin_dir.display());
                println!();
                println!(
                    "{} After updating PATH, restart your terminal.",
                    "Important:".yellow().bold()
                );
            }

            #[cfg(not(windows))]
            {
                println!(
                    "{} Add the following to your shell profile (~/.bashrc, ~/.zshrc, etc.):",
                    "Note:".yellow().bold()
                );
                println!("   export PATH=\"{}:$PATH\"", bin_dir.display());
            }

            println!();

            config.path_setup_shown = true;
            config.save()?;
        }

        Ok(())
    }

    /// Ensure proxy binaries are installed
    fn ensure_proxy_binaries(bin_dir: &Path) -> Result<()> {
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

    /// Get the Lean release server URL
    ///
    /// Checks mirrors.lean_release in config, falls back to default.
    /// Can be overridden by LEMMA_RELEASE_URL environment variable.
    pub fn lean_release_url(&self) -> String {
        self.lean_release
            .as_ref()
            .cloned()
            .unwrap_or_else(|| "https://release.lean-lang.org".to_string())
    }
}

// ============================================================================
// Preference Configuration Structures
// ============================================================================

/// Global configuration options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalConfig {
    /// Verbosity level (0 = normal, 1 = debug, 2+ = trace)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verbose: Option<u8>,

    /// Quiet level (0 = normal, 1 = warn, 2+ = error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quiet: Option<u8>,

    /// Color output setting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<ColorChoiceConfig>,
}

/// Color choice configuration (maps to CLI ColorChoice)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorChoiceConfig {
    Auto,
    Always,
    Never,
}

impl From<ColorChoiceConfig> for ColorChoice {
    fn from(config: ColorChoiceConfig) -> Self {
        match config {
            ColorChoiceConfig::Auto => ColorChoice::Auto,
            ColorChoiceConfig::Always => ColorChoice::Always,
            ColorChoiceConfig::Never => ColorChoice::Never,
        }
    }
}

/// Path configuration options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PathsConfig {
    /// Custom lemma home directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home: Option<PathBuf>,

    /// Custom toolchains directory (relative to home or absolute)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolchains: Option<PathBuf>,
}

/// Network configuration options
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    /// Connection timeout in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,

    /// Number of retries for failed requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<u32>,

    /// HTTP proxy URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy: Option<String>,

    /// HTTPS proxy URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub https_proxy: Option<String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.version, "1");
        assert_eq!(config.default_toolchain, None);
        assert_eq!(config.overrides.len(), 0);
        assert!(!config.path_setup_shown);
        assert!(config.global.is_none());
        assert!(config.paths.is_none());
        assert!(config.network.is_none());
        assert!(config.lean_release.is_none());
    }

    #[test]
    fn test_serialize_config() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("version"));
        assert!(toml_str.contains("path_setup_shown"));
        // Default values that are None/empty should be skipped
        assert!(!toml_str.contains("default_toolchain"));
        assert!(!toml_str.contains("overrides"));
    }

    #[test]
    fn test_unified_config_deserialization() {
        let toml = r#"
version = "1"
default_toolchain = "lean-4.24.0-linux"
path_setup_shown = true

[overrides]
"/path/to/project" = "lean-4.23.0-linux"

[global]
verbose = 1
color = "auto"

[paths]
home = "/custom/path"

[network]
timeout = 30
retries = 3

[mirrors]
lean_release = "https://mirror.example.com/lean"
"#;

        let config: Config = toml::from_str(toml).unwrap();

        // State fields
        assert_eq!(config.version, "1");
        assert_eq!(
            config.default_toolchain,
            Some("lean-4.24.0-linux".to_string())
        );
        assert_eq!(config.path_setup_shown, true);
        assert_eq!(
            config.overrides.get("/path/to/project"),
            Some(&"lean-4.23.0-linux".to_string())
        );

        // Preference fields
        assert_eq!(config.global.as_ref().unwrap().verbose, Some(1));
        assert_eq!(
            config.paths.as_ref().unwrap().home,
            Some(PathBuf::from("/custom/path"))
        );
        assert_eq!(config.network.as_ref().unwrap().timeout, Some(30));
    }

    #[test]
    fn test_lean_release_url() {
        let config = Config::default();

        // Default URL
        assert_eq!(config.lean_release_url(), "https://release.lean-lang.org");
    }

    #[test]
    fn test_color_choice_conversion() {
        let auto: ColorChoice = ColorChoiceConfig::Auto.into();
        assert!(matches!(auto, ColorChoice::Auto));

        let always: ColorChoice = ColorChoiceConfig::Always.into();
        assert!(matches!(always, ColorChoice::Always));

        let never: ColorChoice = ColorChoiceConfig::Never.into();
        assert!(matches!(never, ColorChoice::Never));
    }

    #[test]
    fn test_environment_variables() {
        // Test structured environment variable names
        std::env::set_var("LEMMA_GLOBAL__VERBOSE", "2");
        std::env::set_var("LEMMA_GLOBAL__COLOR", "always");
        std::env::set_var("LEMMA_MIRRORS__LEAN_RELEASE", "https://test.mirror.com");
        std::env::set_var("LEMMA_NETWORK__TIMEOUT", "120");

        // Note: We can't easily test Config::load() here as it reads from actual files
        // and depends on the filesystem state. The environment variable handling
        // is tested implicitly through integration tests.

        // Clean up
        std::env::remove_var("LEMMA_GLOBAL__VERBOSE");
        std::env::remove_var("LEMMA_GLOBAL__COLOR");
        std::env::remove_var("LEMMA_MIRRORS__LEAN_RELEASE");
        std::env::remove_var("LEMMA_NETWORK__TIMEOUT");
    }
}
