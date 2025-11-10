//! R2 client for fetching sparse cache indexes and files
//!
//! This module handles communication with the Cloudflare R2 storage
//! to download dependency indexes and cache files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use lemma_download::DownloadClient;

/// Default R2 base URL for sparse mathlib cache
/// TODO: Replace with actual R2 URL when deployed
const DEFAULT_R2_BASE_URL: &str = "https://sparse-cache.example.com";

/// Dependency index structure from R2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyIndex {
    /// Format version for future compatibility
    pub version: u32,

    /// Lean toolchain version (e.g., "leanprover/lean4:v4.8.0-rc1")
    pub lean_version: String,

    /// Mathlib commit hash
    pub mathlib_commit: String,

    /// Platform identifier (e.g., "linux-x86_64")
    pub platform: String,

    /// Timestamp when index was created
    pub created_at: String,

    /// Module information: module_name -> ModuleInfo
    pub modules: HashMap<String, ModuleInfo>,
}

/// Information about a single module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Relative path to .olean file (e.g., "Mathlib/Algebra/Group/Basic.olean")
    pub path: String,

    /// File size in bytes
    pub size: u64,

    /// SHA256 hash of the file
    pub sha256: String,

    /// Direct dependencies of this module
    pub dependencies: Vec<String>,
}

/// R2 client for fetching sparse cache
pub struct R2Client {
    download_client: DownloadClient,
    base_url: String,
}

impl R2Client {
    /// Create a new R2 client
    pub fn new() -> Result<Self> {
        let download_client = DownloadClient::new()?;

        // Allow override via environment variable
        let base_url = std::env::var("LEMMA_SPARSE_CACHE_URL")
            .unwrap_or_else(|_| DEFAULT_R2_BASE_URL.to_string());

        Ok(Self {
            download_client,
            base_url,
        })
    }

    /// Fetch the dependency index for a specific mathlib commit
    pub fn fetch_index(&self, commit: &str, platform: &str) -> Result<DependencyIndex> {
        let url = format!(
            "{}/mathlib/{}/{}/index.json",
            self.base_url, commit, platform
        );

        println!("      Downloading index from: {}", url);

        self.download_client.download_json(&url).with_context(|| {
            format!(
                "Failed to download dependency index for commit {} on {}",
                commit, platform
            )
        })
    }

    /// Download a single .olean file
    pub fn download_olean(
        &self,
        commit: &str,
        platform: &str,
        module_path: &str,
        dest: &Path,
    ) -> Result<()> {
        let url = format!(
            "{}/mathlib/{}/{}/{}",
            self.base_url, commit, platform, module_path
        );

        self.download_client.download_file(&url, dest)?;

        Ok(())
    }

    /// Download the full cache archive
    pub fn download_full_archive(&self, commit: &str, platform: &str, dest: &Path) -> Result<()> {
        let url = format!(
            "{}/mathlib/{}/{}/cache.tar.zst",
            self.base_url, commit, platform
        );

        println!("      Downloading full cache archive from: {}", url);

        self.download_client.download_file(&url, dest)?;

        Ok(())
    }
}

/// Detect current platform
pub fn detect_platform() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    format!("{}-{}", os, arch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = detect_platform();
        // Just verify it returns something reasonable
        assert!(platform.contains('-'));
    }

    #[test]
    fn test_r2_client_creation() {
        let client = R2Client::new();
        assert!(client.is_ok());
    }
}
