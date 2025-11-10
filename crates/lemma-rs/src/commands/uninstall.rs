//! Uninstall command - Remove installed toolchains

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use lemma_config::Config;
use lemma_config::GlobalSettings;

pub fn execute(toolchain: &str, settings: &GlobalSettings) -> Result<()> {
    let toolchains_dir = Config::toolchains_dir()?;

    // Parse the toolchain to get the sanitized directory name
    let toolchain_desc = lemma_toolchain::ToolchainDesc::parse(toolchain)?;
    let dir_name = toolchain_desc.to_directory_name();
    let toolchain_path = toolchains_dir.join(&dir_name);

    // Check if toolchain exists
    if !toolchain_path.exists() {
        anyhow::bail!(
            "Toolchain '{}' is not installed.\nRun 'lemma toolchain list' to see installed toolchains.",
            toolchain
        );
    }

    // Load config to check if this is the default toolchain
    let config = Config::load().unwrap_or_default();
    let is_default = config
        .default_toolchain
        .as_ref()
        .map(|d| d == toolchain)
        .unwrap_or(false);

    if is_default {
        if settings.use_colors() {
            println!(
                "{} Warning: '{}' is currently the default toolchain",
                "⚠".yellow().bold(),
                toolchain
            );
        } else {
            println!(
                "⚠ Warning: '{}' is currently the default toolchain",
                toolchain
            );
        }
    }

    // Check if this is a symlink (linked toolchain)
    let metadata = fs::symlink_metadata(&toolchain_path).with_context(|| {
        format!(
            "Failed to read toolchain metadata: {}",
            toolchain_path.display()
        )
    })?;

    let is_symlink = metadata.file_type().is_symlink();

    // Remove the toolchain
    if is_symlink {
        // For symlinks, we just remove the link itself, not the target
        #[cfg(unix)]
        {
            fs::remove_file(&toolchain_path).with_context(|| {
                format!("Failed to remove symlink: {}", toolchain_path.display())
            })?;
        }

        #[cfg(windows)]
        {
            // On Windows, directory symlinks need remove_dir
            if metadata.is_dir() {
                fs::remove_dir(&toolchain_path).with_context(|| {
                    format!("Failed to remove symlink: {}", toolchain_path.display())
                })?;
            } else {
                fs::remove_file(&toolchain_path).with_context(|| {
                    format!("Failed to remove symlink: {}", toolchain_path.display())
                })?;
            }
        }

        if settings.use_colors() {
            println!(
                "{} Removed linked toolchain '{}'",
                "✓".green().bold(),
                toolchain
            );
        } else {
            println!("✓ Removed linked toolchain '{}'", toolchain);
        }
        println!("   (The original directory was not deleted)");
    } else {
        // For regular directories, remove everything
        fs::remove_dir_all(&toolchain_path).with_context(|| {
            format!(
                "Failed to remove toolchain directory: {}",
                toolchain_path.display()
            )
        })?;

        if settings.use_colors() {
            println!(
                "{} Successfully uninstalled toolchain '{}'",
                "✓".green().bold(),
                toolchain
            );
        } else {
            println!("✓ Successfully uninstalled toolchain '{}'", toolchain);
        }
    }

    if is_default && !settings.is_quiet() {
        println!();
        if settings.use_colors() {
            println!(
                "{} You may want to set a new default toolchain with:",
                "Tip:".dimmed()
            );
        } else {
            println!("Tip: You may want to set a new default toolchain with:");
        }
        println!("   lemma default <toolchain>");
    }

    Ok(())
}
