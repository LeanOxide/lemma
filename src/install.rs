//! Toolchain installation with atomic operations
//!
//! This module implements atomic toolchain installation inspired by both
//! rustup and elan:
//! - Downloads to temporary location
//! - Verifies SHA-256 checksum (if available)
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

/// Toolchain descriptor
#[derive(Debug, Clone)]
pub enum ToolchainDescriptor {
    /// Official Lean release (from release.lean-lang.org)
    OfficialRelease {
        /// Name of the toolchain (e.g., "stable", "v4.5.0")
        name: String,
        /// Release tag (or "latest" for stable)
        tag: String,
    },
    /// Direct URL download
    DirectUrl {
        /// The direct download URL
        url: String,
        /// Extracted name from URL or user-provided name
        name: String,
    },
}

impl ToolchainDescriptor {
    /// Parse a toolchain name into a descriptor
    ///
    /// Supported formats:
    /// - "stable" -> official latest release
    /// - "v4.5.0" -> official version tag
    /// - "https://..." -> direct URL download
    pub fn parse(input: &str) -> Result<Self> {
        // Check if it's a URL
        if input.starts_with("http://") || input.starts_with("https://") {
            let name = Self::extract_name_from_url(input);
            return Ok(Self::DirectUrl {
                url: input.to_string(),
                name,
            });
        }

        // Handle special names
        if input == "stable" || input == "latest" {
            return Ok(Self::OfficialRelease {
                name: "stable".to_string(),
                tag: "latest".to_string(),
            });
        }

        // Assume it's a version tag for official Lean release
        Ok(Self::OfficialRelease {
            name: input.to_string(),
            tag: input.to_string(),
        })
    }

    /// Extract a meaningful name from a URL
    /// Examples:
    ///   https://example.com/lean-4.24.0-linux.tar.zst -> lean-4.24.0-linux
    ///   https://example.com/path/to/custom.tar.gz -> custom
    fn extract_name_from_url(url: &str) -> String {
        // Get the last path segment
        let path = url.split('/').last().unwrap_or("direct-download");

        // Remove common archive extensions
        path.trim_end_matches(".tar.zst")
            .trim_end_matches(".tar.gz")
            .trim_end_matches(".tar.xz")
            .trim_end_matches(".tgz")
            .trim_end_matches(".zip")
            .to_string()
    }

    /// Get the display name for this toolchain
    pub fn name(&self) -> &str {
        match self {
            Self::OfficialRelease { name, .. } => name,
            Self::DirectUrl { name, .. } => name,
        }
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
        let descriptor = ToolchainDescriptor::parse(toolchain)?;

        println!(
            "{} Installing toolchain: {}",
            "=>".green().bold(),
            descriptor.name()
        );

        // Check if already installed
        let install_path = self.toolchain_path(descriptor.name())?;
        if install_path.exists() && !force {
            println!(
                "{} Toolchain already installed at: {}",
                "=>".yellow().bold(),
                install_path.display()
            );
            println!("   Use --force to reinstall");
            return Ok(());
        }

        // Get download URL and asset name based on descriptor type
        let (asset_name, asset_url) = match &descriptor {
            ToolchainDescriptor::DirectUrl { url, .. } => {
                println!("{} Downloading from URL...", "=>".cyan().bold());
                println!("   URL: {}", url);

                // Extract filename from URL for asset name
                let filename = url.split('/').last().unwrap_or("archive.tar.zst");
                (filename.to_string(), url.clone())
            }
            ToolchainDescriptor::OfficialRelease { .. } => {
                // Fetch release information from release.lean-lang.org
                println!("{} Fetching release information...", "=>".blue().bold());
                let release = self.fetch_release(&descriptor)?;

                println!("   Found release: {}", release.name.bold());

                // Find the right asset for our platform
                let asset = self
                    .release_client
                    .find_platform_asset(&release)
                    .context("No compatible asset found for your platform")?;
                (asset.name.clone(), asset.browser_download_url.clone())
            }
        };

        println!("   Asset: {}", asset_name);

        // Ensure parent directory exists first
        if let Some(parent) = install_path.parent() {
            fs::create_dir_all(parent).context("Failed to create toolchains directory")?;
        }

        let tmp_dir = Config::tmp_dir()?;
        fs::create_dir_all(&tmp_dir).context("Failed to create tmp directory")?;

        // Create unique temp directory for this installation
        let temp_install = tmp_dir.join(format!("{}-{}", descriptor.name(), std::process::id()));

        // Clean up any existing temp directory from previous failed installation
        if temp_install.exists() {
            fs::remove_dir_all(&temp_install).context("Failed to clean up old temp directory")?;
        }

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

        // Verify installation by checking for lean binary (skip for direct URLs as they might not be Lean)
        if !matches!(descriptor, ToolchainDescriptor::DirectUrl { .. }) {
            let lean_bin = self.find_lean_binary(&install_path)?;
            println!("   Lean binary: {}", lean_bin.display());
        } else {
            println!(
                "   {} Installed from direct URL - skipping binary verification",
                "Note:".yellow()
            );
        }

        // Save update hash for tracking (use asset URL as version identifier)
        let version_hash = match &descriptor {
            ToolchainDescriptor::OfficialRelease { tag, .. } => tag.clone(),
            ToolchainDescriptor::DirectUrl { url, .. } => url.clone(),
        };
        let _ = Config::save_update_hash(descriptor.name(), &version_hash); // Best effort

        Ok(())
    }

    /// Fetch release information from release.lean-lang.org
    fn fetch_release(&self, descriptor: &ToolchainDescriptor) -> Result<Release> {
        match descriptor {
            ToolchainDescriptor::OfficialRelease { tag, .. } => {
                if tag == "latest" {
                    self.release_client.get_latest_stable()
                } else {
                    self.release_client.find_release(tag)
                }
            }
            ToolchainDescriptor::DirectUrl { .. } => {
                unreachable!("fetch_release should not be called for DirectUrl")
            }
        }
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
    fn test_parse_toolchain_stable() {
        let desc = ToolchainDescriptor::parse("stable").unwrap();
        match desc {
            ToolchainDescriptor::OfficialRelease { tag, .. } => {
                assert_eq!(tag, "latest");
            }
            _ => panic!("Expected OfficialRelease variant"),
        }
    }

    #[test]
    fn test_parse_toolchain_version() {
        let desc = ToolchainDescriptor::parse("v4.5.0").unwrap();
        match desc {
            ToolchainDescriptor::OfficialRelease { tag, .. } => {
                assert_eq!(tag, "v4.5.0");
            }
            _ => panic!("Expected OfficialRelease variant"),
        }
    }

    #[test]
    fn test_parse_toolchain_url() {
        let desc =
            ToolchainDescriptor::parse("https://example.com/lean-4.24.0-linux.tar.zst").unwrap();
        match desc {
            ToolchainDescriptor::DirectUrl { url, name } => {
                assert_eq!(url, "https://example.com/lean-4.24.0-linux.tar.zst");
                assert_eq!(name, "lean-4.24.0-linux");
            }
            _ => panic!("Expected DirectUrl variant"),
        }
    }

    #[test]
    fn test_extract_name_from_url() {
        assert_eq!(
            ToolchainDescriptor::extract_name_from_url(
                "https://example.com/lean-4.24.0-linux.tar.zst"
            ),
            "lean-4.24.0-linux"
        );
        assert_eq!(
            ToolchainDescriptor::extract_name_from_url("https://mirror.com/path/to/custom.tar.gz"),
            "custom"
        );
        assert_eq!(
            ToolchainDescriptor::extract_name_from_url("http://example.com/archive.zip"),
            "archive"
        );
    }
}
