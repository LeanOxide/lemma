//! Toolchain installation with atomic operations
//!
//! This module implements atomic toolchain installation inspired by both
//! rustup and elan:
//! - Downloads to temporary location
//! - Extracts to temporary directory
//! - Atomically moves to final location
//! - Cleanup on failure

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::archive::extract_archive;
use crate::config::Config;
use crate::download::DownloadClient;
use crate::release::{Release, ReleaseServerClient};
use crate::toolchain::ToolchainDesc;

/// Toolchain installer
pub struct Installer {
    download_client: DownloadClient,
    release_client: ReleaseServerClient,
}

impl Installer {
    /// Create a new installer
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let download_client = DownloadClient::new()?;
        let release_client = ReleaseServerClient::new(download_client.clone(), config.release_url);

        Ok(Self {
            download_client,
            release_client,
        })
    }

    /// Install a toolchain
    pub fn install(&self, toolchain: &str, force: bool) -> Result<()> {
        // Parse as ToolchainDesc to get the canonical name with origin
        let toolchain_desc = ToolchainDesc::parse(toolchain)?;

        println!(
            "{} Installing toolchain: {}",
            "=>".green().bold(),
            toolchain_desc
        );

        // Check if already installed using sanitized directory name
        let install_path = self.toolchain_path(&toolchain_desc.to_directory_name())?;
        if install_path.exists() && !force {
            println!(
                "{} Toolchain already installed at: {}",
                "=>".yellow().bold(),
                install_path.display()
            );
            println!("   Use --force to reinstall");
            return Ok(());
        }

        // Fetch release information from release.lean-lang.org
        println!("{} Fetching release information...", "=>".blue().bold());
        let release = self.fetch_release(&toolchain_desc)?;

        println!("   Found release: {}", release.name.bold());

        // Find the right asset for our platform
        let asset = self
            .release_client
            .find_platform_asset(&release)
            .context("No compatible asset found for your platform")?;

        let asset_name = asset.name.clone();
        let asset_url = asset.browser_download_url.clone();

        println!("   Asset: {}", asset_name);

        // Ensure parent directory exists first
        if let Some(parent) = install_path.parent() {
            fs::create_dir_all(parent).context("Failed to create toolchains directory")?;
        }

        let tmp_dir = Config::tmp_dir()?;
        fs::create_dir_all(&tmp_dir).context("Failed to create tmp directory")?;

        // Create temp directory for this installation
        // NOTE: We don't include process ID so that resuming after Ctrl-C works
        let temp_install = tmp_dir.join(toolchain_desc.to_directory_name());

        // Create temp directory if it doesn't exist
        fs::create_dir_all(&temp_install).context("Failed to create temp directory")?;

        // Download the asset to temp directory
        let download_path = temp_install.join(&asset_name);
        println!("{} Downloading...", "=>".cyan().bold());
        self.download_client
            .download_file(&asset_url, &download_path)?;

        // TODO: Verify checksum if available in release notes or separate file
        // For now, we skip checksum verification as Lean releases don't
        // always provide checksums

        // Extract to temporary subdirectory
        println!("{} Extracting...", "=>".cyan().bold());
        let extract_temp = temp_install.join("extracted");
        fs::create_dir_all(&extract_temp).context("Failed to create extraction directory")?;

        extract_archive(&download_path, &extract_temp).context("Failed to extract archive")?;

        // Atomically move to final location
        println!(
            "{} Installing to: {}",
            "=>".cyan().bold(),
            install_path.display()
        );

        // Remove existing installation if force is enabled
        if install_path.exists() {
            fs::remove_dir_all(&install_path).context("Failed to remove existing installation")?;
        }

        // Atomic rename from temp to final location
        // Both tmp/ and toolchains/ are under ~/.lemma/, so same filesystem
        fs::rename(&extract_temp, &install_path).context("Failed to install toolchain")?;

        // Clean up temp directory
        let _ = fs::remove_dir_all(&temp_install); // Best effort cleanup

        println!(
            "{} Successfully installed {} to {}",
            "✓".green().bold(),
            toolchain_desc,
            install_path.display()
        );

        // Verify installation by checking for lean binary
        let lean_bin = self.find_lean_binary(&install_path)?;
        println!("   Lean binary: {}", lean_bin.display());

        // Save update hash for tracking
        let version_hash = release.name.as_str();
        let _ = Config::save_update_hash(&toolchain_desc.to_string(), version_hash); // Best effort

        Ok(())
    }

    /// Fetch release information from release.lean-lang.org
    pub fn fetch_release(&self, toolchain: &ToolchainDesc) -> Result<Release> {
        let release = toolchain.release();

        match release {
            "stable" | "latest" => self.release_client.get_latest_stable(),
            "beta" => self.release_client.get_latest_beta(),
            "nightly" => self.release_client.get_latest_nightly(),
            tag => self.release_client.find_release(tag),
        }
    }

    /// Check if a toolchain is installed
    pub fn is_installed(&self, toolchain_name: &str) -> Result<bool> {
        let install_path = self.toolchain_path(toolchain_name)?;
        Ok(install_path.exists())
    }

    /// Get the installation path for a toolchain
    fn toolchain_path(&self, name: &str) -> Result<PathBuf> {
        let toolchains_dir = Config::toolchains_dir()?;
        Ok(toolchains_dir.join(name))
    }

    /// Find the lean binary in an installation
    fn find_lean_binary(&self, install_path: &Path) -> Result<PathBuf> {
        // Common locations for lean binary
        let candidates = vec![
            install_path.join("bin").join("lean"),
            install_path.join("bin").join("lean.exe"),
            install_path.join("lean"),
            install_path.join("lean.exe"),
        ];

        for candidate in candidates {
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        anyhow::bail!(
            "Could not find lean binary in installation at {}",
            install_path.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_release_stable() {
        // Just test that parsing works
        let desc = ToolchainDesc::parse("stable").unwrap();
        assert_eq!(desc.release(), "stable");
        assert!(desc.is_tracking_channel());
    }

    #[test]
    fn test_fetch_release_beta() {
        let desc = ToolchainDesc::parse("beta").unwrap();
        assert_eq!(desc.release(), "beta");
        assert!(desc.is_tracking_channel());
    }

    #[test]
    fn test_fetch_release_nightly() {
        let desc = ToolchainDesc::parse("nightly").unwrap();
        assert_eq!(desc.release(), "nightly");
        assert!(desc.is_tracking_channel());
    }

    #[test]
    fn test_fetch_release_version() {
        let desc = ToolchainDesc::parse("v4.24.0").unwrap();
        assert_eq!(desc.release(), "v4.24.0");
        assert!(!desc.is_tracking_channel());
    }

    #[test]
    fn test_fetch_release_with_origin() {
        let desc = ToolchainDesc::parse("leanprover/lean4:4.25.0").unwrap();
        assert_eq!(desc.release(), "v4.25.0");
        assert_eq!(desc.origin(), Some("leanprover/lean4"));
        assert!(!desc.is_tracking_channel());
    }

    #[test]
    fn test_parse_latest_alias() {
        // "latest" is treated as a remote toolchain name
        let desc = ToolchainDesc::parse("latest").unwrap();
        assert_eq!(desc.release(), "latest");
        // It won't be a tracking channel unless we explicitly handle it in install
    }
}
