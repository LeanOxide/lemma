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
        let config_path = Self::config_path()?;

        let mut config = if config_path.exists() {
            let content = fs::read_to_string(&config_path).context("Failed to read config file")?;
            toml::from_str(&content).context("Failed to parse config file")?
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
        let config_path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        fs::write(&config_path, content).context("Failed to write config file")?;

        Ok(())
    }

    /// Get the path to the config file
    pub fn config_path() -> Result<PathBuf> {
        let home = Self::lemma_home()?;
        Ok(home.join("config.toml"))
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
