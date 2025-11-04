//! Configuration management for Lemma
//!
//! This module handles all configuration, including:
//! - Proxy settings (HTTP, HTTPS, SOCKS5)
//! - Custom registry URLs and mirrors
//! - Network settings (timeout, retry, bandwidth)
//! - Authentication tokens

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Network configuration
    #[serde(default)]
    pub network: NetworkConfig,

    /// Toolchain source configuration
    #[serde(default)]
    pub sources: SourcesConfig,

    /// Default toolchain
    pub default_toolchain: Option<String>,

    /// Directory overrides (path -> toolchain)
    #[serde(default)]
    pub overrides: std::collections::HashMap<String, String>,

    /// Whether PATH setup message has been shown
    #[serde(default)]
    pub path_setup_shown: bool,

    /// Telemetry opt-out
    #[serde(default)]
    pub telemetry: bool,
}

/// Network-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// HTTP proxy URL (e.g., "http://proxy.example.com:8080")
    pub http_proxy: Option<String>,

    /// HTTPS proxy URL
    pub https_proxy: Option<String>,

    /// SOCKS5 proxy URL (e.g., "socks5://127.0.0.1:1080")
    pub socks_proxy: Option<String>,

    /// No proxy domains (comma-separated, e.g., "localhost,.local")
    pub no_proxy: Option<String>,

    /// Proxy authentication (username:password)
    pub proxy_auth: Option<String>,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub connect_timeout: u64,

    /// Read timeout in seconds
    #[serde(default = "default_timeout")]
    pub read_timeout: u64,

    /// Maximum retries for failed downloads
    #[serde(default = "default_retries")]
    pub max_retries: u32,

    /// Retry delay in seconds (uses exponential backoff)
    #[serde(default = "default_retry_delay")]
    pub retry_delay: u64,

    /// Maximum download speed in bytes/sec (0 = unlimited)
    #[serde(default)]
    pub max_download_speed: u64,

    /// Enable download resumption for partial downloads
    #[serde(default = "default_true")]
    pub resume_downloads: bool,

    /// Skip SSL certificate verification (DANGEROUS - use only for testing)
    #[serde(default)]
    pub insecure: bool,
}

/// Toolchain source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    /// Primary release server URL
    #[serde(default = "default_release_url")]
    pub release_url: String,

    /// Mirror URLs (tried in order if primary fails)
    #[serde(default)]
    pub mirrors: Vec<String>,

    /// GitHub API base URL (for custom GitHub Enterprise)
    #[serde(default = "default_github_api")]
    pub github_api: String,

    /// GitHub authentication token for API access
    pub github_token: Option<String>,

    /// Custom registries (name -> URL mapping)
    #[serde(default)]
    pub custom_registries: std::collections::HashMap<String, String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            http_proxy: None,
            https_proxy: None,
            socks_proxy: None,
            no_proxy: None,
            proxy_auth: None,
            connect_timeout: default_timeout(),
            read_timeout: default_timeout(),
            max_retries: default_retries(),
            retry_delay: default_retry_delay(),
            max_download_speed: 0,
            resume_downloads: true,
            insecure: false,
        }
    }
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            release_url: default_release_url(),
            mirrors: vec![],
            github_api: default_github_api(),
            github_token: None,
            custom_registries: std::collections::HashMap::new(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            sources: SourcesConfig::default(),
            default_toolchain: Some("stable".to_string()),
            overrides: std::collections::HashMap::new(),
            path_setup_shown: false,
            telemetry: false,
        }
    }
}

fn default_timeout() -> u64 {
    30
}

fn default_retries() -> u32 {
    3
}

fn default_retry_delay() -> u64 {
    2
}

fn default_true() -> bool {
    true
}

fn default_release_url() -> String {
    "https://release.lean-lang.org".to_string()
}

fn default_github_api() -> String {
    "https://api.github.com".to_string()
}

impl Config {
    /// Load configuration from file, with environment variable overrides
    pub fn load() -> Result<Self> {
        let settings_path = Self::settings_path()?;

        // Migration: check for old config.toml and rename it
        let old_config_path = Self::lemma_home()?.join("config.toml");
        if old_config_path.exists() && !settings_path.exists() {
            fs::rename(&old_config_path, &settings_path)
                .context("Failed to migrate config.toml to settings.toml")?;
        }

        let mut config = if settings_path.exists() {
            let content =
                fs::read_to_string(&settings_path).context("Failed to read settings file")?;
            toml::from_str(&content).context("Failed to parse settings file")?
        } else {
            Self::default()
        };

        // Override with environment variables
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // Standard proxy environment variables
        if let Ok(proxy) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
            self.network.http_proxy = Some(proxy);
        }

        if let Ok(proxy) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
            self.network.https_proxy = Some(proxy);
        }

        if let Ok(proxy) = std::env::var("ALL_PROXY").or_else(|_| std::env::var("all_proxy")) {
            if self.network.http_proxy.is_none() {
                self.network.http_proxy = Some(proxy.clone());
            }
            if self.network.https_proxy.is_none() {
                self.network.https_proxy = Some(proxy);
            }
        }

        if let Ok(no_proxy) = std::env::var("NO_PROXY").or_else(|_| std::env::var("no_proxy")) {
            self.network.no_proxy = Some(no_proxy);
        }

        // Lemma-specific overrides
        if let Ok(token) = std::env::var("LEMMA_GITHUB_TOKEN") {
            self.sources.github_token = Some(token);
        }

        if let Ok(url) = std::env::var("LEMMA_RELEASE_URL") {
            self.sources.release_url = url;
        }
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

    /// Get the path to the config file (deprecated, use settings_path)
    #[deprecated(note = "Use settings_path() instead")]
    pub fn config_path() -> Result<PathBuf> {
        Self::settings_path()
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

    /// Get connect timeout as Duration
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.network.connect_timeout)
    }

    /// Get read timeout as Duration
    pub fn read_timeout(&self) -> Duration {
        Duration::from_secs(self.network.read_timeout)
    }

    /// Get retry delay as Duration
    pub fn retry_delay(&self) -> Duration {
        Duration::from_secs(self.network.retry_delay)
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
        assert_eq!(config.network.connect_timeout, 30);
        assert_eq!(config.network.max_retries, 3);
        assert!(config.network.resume_downloads);
        assert_eq!(config.sources.release_url, "https://release.lean-lang.org");
    }

    #[test]
    fn test_serialize_config() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("connect_timeout"));
        assert!(toml_str.contains("release_url"));
    }
}
