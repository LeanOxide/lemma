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

/// Toolchain installer
pub struct Installer {
    download_client: DownloadClient,
    release_client: ReleaseServerClient,
}

/// Channel represents the different release channels for Lean
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Channel {
    /// Stable channel
    Stable,
    /// Beta channel
    Beta,
    /// Nightly channel
    Nightly,
    /// Specific version (e.g., "v4.24.0", "v4.15.0-rc1")
    Version(String),
}

impl Channel {
    /// Parse a channel from a string
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "stable" | "latest" => Ok(Channel::Stable),
            "beta" => Ok(Channel::Beta),
            "nightly" => Ok(Channel::Nightly),
            _ => {
                // Validate that it looks like a version
                if s.is_empty() {
                    anyhow::bail!("Empty channel name");
                }
                Ok(Channel::Version(s.to_string()))
            }
        }
    }

    /// Get the display name for this channel
    pub fn name(&self) -> String {
        match self {
            Channel::Stable => "stable".to_string(),
            Channel::Beta => "beta".to_string(),
            Channel::Nightly => "nightly".to_string(),
            Channel::Version(v) => v.clone(),
        }
    }

    /// Returns true if this is a channel that should auto-update
    pub fn is_tracking_channel(&self) -> bool {
        matches!(self, Channel::Stable | Channel::Beta | Channel::Nightly)
    }
}

/// Toolchain descriptor - describes an official Lean toolchain
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolchainDesc {
    /// The release channel
    pub channel: Channel,
    /// Optional date for dated releases (e.g., "2024-01-15")
    /// Currently not used for Lean, but reserved for future compatibility
    pub date: Option<String>,
}

impl ToolchainDesc {
    /// Parse a toolchain specification into a descriptor
    ///
    /// Supported formats:
    /// - "stable" -> stable channel
    /// - "beta" -> beta channel
    /// - "nightly" -> nightly channel
    /// - "v4.24.0" -> specific version
    /// - "lean-4.24.0" -> specific version (strips "lean-" prefix)
    pub fn parse(input: &str) -> Result<Self> {
        if input.is_empty() {
            anyhow::bail!("Empty toolchain specification");
        }

        // Strip common "lean-" prefix if present
        let input = input.strip_prefix("lean-").unwrap_or(input);

        // For now, we don't support dated releases, so date is always None
        let channel = Channel::parse(input)?;

        Ok(ToolchainDesc {
            channel,
            date: None,
        })
    }

    /// Get the display name for this toolchain
    pub fn name(&self) -> String {
        self.channel.name()
    }

    /// Returns true if this toolchain should auto-update
    pub fn is_tracking(&self) -> bool {
        self.channel.is_tracking_channel()
    }
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
        let descriptor = ToolchainDesc::parse(toolchain)?;

        println!(
            "{} Installing toolchain: {}",
            "=>".green().bold(),
            descriptor.name()
        );

        // Check if already installed
        let install_path = self.toolchain_path(&descriptor.name())?;
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
        let release = self.fetch_release(&descriptor)?;

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
        let temp_install = tmp_dir.join(descriptor.name());

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
            descriptor.name(),
            install_path.display()
        );

        // Verify installation by checking for lean binary
        let lean_bin = self.find_lean_binary(&install_path)?;
        println!("   Lean binary: {}", lean_bin.display());

        // Save update hash for tracking
        let version_hash = release.name.as_str();
        let _ = Config::save_update_hash(&descriptor.name(), version_hash); // Best effort

        Ok(())
    }

    /// Fetch release information from release.lean-lang.org
    pub fn fetch_release(&self, descriptor: &ToolchainDesc) -> Result<Release> {
        match &descriptor.channel {
            Channel::Stable => self.release_client.get_latest_stable(),
            Channel::Beta => self.release_client.get_latest_beta(),
            Channel::Nightly => self.release_client.get_latest_nightly(),
            Channel::Version(tag) => self.release_client.find_release(tag),
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
    fn test_parse_channel_stable() {
        let channel = Channel::parse("stable").unwrap();
        assert_eq!(channel, Channel::Stable);
        assert_eq!(channel.name(), "stable");
        assert!(channel.is_tracking_channel());
    }

    #[test]
    fn test_parse_channel_beta() {
        let channel = Channel::parse("beta").unwrap();
        assert_eq!(channel, Channel::Beta);
        assert_eq!(channel.name(), "beta");
        assert!(channel.is_tracking_channel());
    }

    #[test]
    fn test_parse_channel_nightly() {
        let channel = Channel::parse("nightly").unwrap();
        assert_eq!(channel, Channel::Nightly);
        assert_eq!(channel.name(), "nightly");
        assert!(channel.is_tracking_channel());
    }

    #[test]
    fn test_parse_channel_version() {
        let channel = Channel::parse("v4.24.0").unwrap();
        assert_eq!(channel, Channel::Version("v4.24.0".to_string()));
        assert_eq!(channel.name(), "v4.24.0");
        assert!(!channel.is_tracking_channel());
    }

    #[test]
    fn test_parse_toolchain_stable() {
        let desc = ToolchainDesc::parse("stable").unwrap();
        assert_eq!(desc.channel, Channel::Stable);
        assert_eq!(desc.name(), "stable");
        assert!(desc.is_tracking());
    }

    #[test]
    fn test_parse_toolchain_beta() {
        let desc = ToolchainDesc::parse("beta").unwrap();
        assert_eq!(desc.channel, Channel::Beta);
        assert_eq!(desc.name(), "beta");
        assert!(desc.is_tracking());
    }

    #[test]
    fn test_parse_toolchain_nightly() {
        let desc = ToolchainDesc::parse("nightly").unwrap();
        assert_eq!(desc.channel, Channel::Nightly);
        assert_eq!(desc.name(), "nightly");
        assert!(desc.is_tracking());
    }

    #[test]
    fn test_parse_toolchain_version() {
        let desc = ToolchainDesc::parse("v4.5.0").unwrap();
        assert_eq!(desc.channel, Channel::Version("v4.5.0".to_string()));
        assert_eq!(desc.name(), "v4.5.0");
        assert!(!desc.is_tracking());
    }

    #[test]
    fn test_parse_toolchain_with_lean_prefix() {
        let desc = ToolchainDesc::parse("lean-4.24.0").unwrap();
        assert_eq!(desc.channel, Channel::Version("4.24.0".to_string()));
        assert_eq!(desc.name(), "4.24.0");
    }

    #[test]
    fn test_parse_toolchain_latest_alias() {
        let desc = ToolchainDesc::parse("latest").unwrap();
        assert_eq!(desc.channel, Channel::Stable);
    }

    #[test]
    fn test_parse_empty_toolchain() {
        assert!(ToolchainDesc::parse("").is_err());
    }
}
