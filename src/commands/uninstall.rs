//! Uninstall command - Remove installed toolchains

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use crate::config::Config;

pub fn execute(toolchain: &str) -> Result<()> {
    let toolchains_dir = Config::toolchains_dir()?;
    let toolchain_path = toolchains_dir.join(toolchain);

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
        println!(
            "{} Warning: '{}' is currently the default toolchain",
            "⚠".yellow().bold(),
            toolchain.bright_white()
        );
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

        println!(
            "{} Removed linked toolchain '{}'",
            "✓".green().bold(),
            toolchain.bright_white()
        );
        println!("   (The original directory was not deleted)");
    } else {
        // For regular directories, remove everything
        fs::remove_dir_all(&toolchain_path).with_context(|| {
            format!(
                "Failed to remove toolchain directory: {}",
                toolchain_path.display()
            )
        })?;

        println!(
            "{} Successfully uninstalled toolchain '{}'",
            "✓".green().bold(),
            toolchain.bright_white()
        );
    }

    if is_default {
        println!();
        println!(
            "{} You may want to set a new default toolchain with:",
            "Tip:".dimmed()
        );
        println!("   lemma default <toolchain>");
    }

    Ok(())
}
