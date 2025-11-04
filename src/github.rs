//! GitHub API client using proper REST API (not HTML scraping!)
//!
//! Unlike elan which scrapes GitHub HTML with fragile regex patterns,
//! this module uses the official GitHub REST API v3.

use crate::config::Config;
use crate::download::DownloadClient;
use crate::errors::GitHubError;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// GitHub release information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}

/// GitHub release asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub content_type: String,
}

/// GitHub API client
pub struct GitHubClient {
    client: DownloadClient,
    api_base: String,
    token: Option<String>,
}

impl GitHubClient {
    /// Create a new GitHub API client
    pub fn new(config: Config) -> Result<Self> {
        let client = DownloadClient::new(config.clone())?;
        let api_base = config.sources.github_api.clone();
        let token = config.sources.github_token.clone();

        Ok(Self {
            client,
            api_base,
            token,
        })
    }

    /// Get a specific release by tag
    pub fn get_release(&self, owner: &str, repo: &str, tag: &str) -> Result<Release> {
        let url = format!(
            "{}/repos/{}/{}/releases/tags/{}",
            self.api_base, owner, repo, tag
        );

        self.fetch_api(&url)
            .with_context(|| format!("Failed to fetch release {} from {}/{}", tag, owner, repo))
    }

    /// Get the latest release
    pub fn get_latest_release(&self, owner: &str, repo: &str) -> Result<Release> {
        let url = format!("{}/repos/{}/{}/releases/latest", self.api_base, owner, repo);

        self.fetch_api(&url)
            .with_context(|| format!("Failed to fetch latest release from {}/{}", owner, repo))
    }

    /// List all releases
    pub fn list_releases(&self, owner: &str, repo: &str) -> Result<Vec<Release>> {
        let url = format!("{}/repos/{}/{}/releases", self.api_base, owner, repo);

        self.fetch_api(&url)
            .with_context(|| format!("Failed to list releases from {}/{}", owner, repo))
    }

    /// Find asset for current platform in a release
    pub fn find_platform_asset<'a>(&self, release: &'a Release) -> Result<&'a Asset> {
        let platform = current_platform();

        // Try to find exact match first
        let asset = release
            .assets
            .iter()
            .find(|a| a.name.contains(&platform))
            .or_else(|| {
                // Try alternative naming conventions
                release
                    .assets
                    .iter()
                    .find(|a| matches_platform(&a.name, &platform))
            })
            .ok_or_else(|| {
                let available = release
                    .assets
                    .iter()
                    .map(|a| format!("  - {}", a.name))
                    .collect::<Vec<_>>()
                    .join("\n");

                GitHubError::NoSuitableAsset {
                    platform: platform.clone(),
                    tag: release.tag_name.clone(),
                    available,
                }
            })?;

        Ok(asset)
    }

    /// Fetch from GitHub API with authentication
    fn fetch_api<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        // For now, use download_json from DownloadClient
        // In a full implementation, we'd add authorization header support
        // to the download client

        // TODO: Add header support to DownloadClient for Authorization header
        // For now, GitHub API still works without auth (with rate limits)

        self.client
            .download_json(url)
            .context("GitHub API request failed")
    }
}

/// Get the current platform identifier
fn current_platform() -> String {
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else {
        "unknown"
    };

    format!("{}-{}", os, arch)
}

/// Check if asset name matches platform with various naming conventions
fn matches_platform(asset_name: &str, platform: &str) -> bool {
    let name_lower = asset_name.to_lowercase();
    let platform_lower = platform.to_lowercase();

    // Handle various naming conventions:
    // - lean-4.0.0-linux-x86_64.tar.gz
    // - lean-linux-x86_64.tar.zst
    // - lean-x86_64-unknown-linux-gnu.tar.gz
    // - lean-darwin-aarch64.zip

    if name_lower.contains(&platform_lower) {
        return true;
    }

    // Check OS separately
    let os = if platform.contains("linux") {
        "linux"
    } else if platform.contains("darwin") {
        Some("darwin").or(Some("macos")).unwrap()
    } else if platform.contains("windows") {
        "windows"
    } else {
        return false;
    };

    // Check arch separately
    let arch = if platform.contains("x86_64") {
        "x86_64"
    } else if platform.contains("aarch64") {
        Some("aarch64").or(Some("arm64")).unwrap()
    } else {
        return false;
    };

    name_lower.contains(os) && name_lower.contains(arch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_platform() {
        let platform = current_platform();
        assert!(!platform.is_empty());
        assert!(platform.contains("-"));
    }

    #[test]
    fn test_matches_platform() {
        assert!(matches_platform(
            "lean-4.0.0-linux-x86_64.tar.gz",
            "linux-x86_64"
        ));

        assert!(matches_platform(
            "lean-darwin-aarch64.zip",
            "darwin-aarch64"
        ));

        assert!(!matches_platform("lean-windows-x86_64.zip", "linux-x86_64"));
    }
}
