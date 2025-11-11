//! Release server client for fetching Lean releases
//!
//! This module handles fetching releases from the official Lean release server
//! at release.lean-lang.org, matching elan's behavior. It falls back to GitHub
//! API for custom repositories.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::download::DownloadClient;

/// Release information from release.lean-lang.org
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseIndex {
    pub version: String,
    #[serde(default)]
    pub stable: Vec<Release>,
    #[serde(default)]
    pub beta: Vec<Release>,
    #[serde(default)]
    pub nightly: Vec<Release>,
}

/// Individual release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub name: String,
    pub created_at: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

/// Release asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
}

/// Client for fetching from release.lean-lang.org
pub struct ReleaseServerClient {
    client: DownloadClient,
    base_url: String,
}

impl ReleaseServerClient {
    /// Create a new release server client
    pub fn new(client: DownloadClient, base_url: String) -> Self {
        Self { client, base_url }
    }

    /// Fetch the release index
    pub fn fetch_index(&self) -> Result<ReleaseIndex> {
        self.client
            .download_json(&self.base_url)
            .with_context(|| format!("Failed to fetch release index from {}", self.base_url))
    }

    /// Find a release by name in the index
    pub fn find_release(&self, name: &str) -> Result<Release> {
        let index = self.fetch_index()?;

        // Check stable releases first
        if let Some(release) = index.stable.iter().find(|r| r.name == name) {
            return Ok(release.clone());
        }

        // Check beta releases
        if let Some(release) = index.beta.iter().find(|r| r.name == name) {
            return Ok(release.clone());
        }

        // Check nightly releases
        if let Some(release) = index.nightly.iter().find(|r| r.name == name) {
            return Ok(release.clone());
        }

        anyhow::bail!("Release '{}' not found in index", name)
    }

    /// Get the latest stable release
    pub fn get_latest_stable(&self) -> Result<Release> {
        let index = self.fetch_index()?;

        index
            .stable
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No stable releases found"))
    }

    /// Get the latest beta release
    pub fn get_latest_beta(&self) -> Result<Release> {
        let index = self.fetch_index()?;

        index
            .beta
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No beta releases found"))
    }

    /// Get the latest nightly release
    pub fn get_latest_nightly(&self) -> Result<Release> {
        let index = self.fetch_index()?;

        index
            .nightly
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No nightly releases found"))
    }

    /// Find the appropriate asset for the current platform
    pub fn find_platform_asset<'a>(&self, release: &'a Release) -> Result<&'a Asset> {
        let platform = lemma_platform::current_platform();

        // The release server uses naming like: lean-4.24.0-linux.tar.zst
        // We need to match: linux, linux_aarch64, darwin, darwin_aarch64, windows
        let name_substring = format!("{}.tar", platform); // e.g., "linux.tar" or "linux_aarch64.tar"

        release
            .assets
            .iter()
            .find(|a| a.name.contains(&name_substring))
            .ok_or_else(|| {
                let available = release
                    .assets
                    .iter()
                    .map(|a| format!("  - {}", a.name))
                    .collect::<Vec<_>>()
                    .join("\n");

                anyhow::anyhow!(
                    "No suitable asset found for platform '{}' in release {}\n\nAvailable assets:\n{}",
                    platform,
                    release.name,
                    available
                )
            })
    }
}

// Re-export current_platform from lemma_platform for backwards compatibility
pub use lemma_platform::current_platform;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_platform() {
        let platform = current_platform();
        assert!(!platform.is_empty());
        // Should be one of: linux, linux_aarch64, linux_x86, darwin, darwin_aarch64, windows
        assert!(
            platform == "linux"
                || platform == "linux_aarch64"
                || platform == "linux_x86"
                || platform == "darwin"
                || platform == "darwin_aarch64"
                || platform == "windows"
        );
    }

    #[test]
    fn test_parse_release_index() {
        let json = r#"{
            "version": "1",
            "stable": [{
                "name": "v4.24.0",
                "created_at": "2025-09-22T12:15:32Z",
                "assets": [{
                    "name": "lean-4.24.0-linux.tar.zst",
                    "browser_download_url": "https://releases.lean-lang.org/lean4/v4.24.0/lean-4.24.0-linux.tar.zst"
                }]
            }]
        }"#;

        let index: ReleaseIndex = serde_json::from_str(json).unwrap();
        assert_eq!(index.version, "1");
        assert_eq!(index.stable.len(), 1);
        assert_eq!(index.stable[0].name, "v4.24.0");
    }
}
