//! Default command - Set the default toolchain

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::Config;
use crate::settings::GlobalSettings;
use crate::toolchain::ToolchainDesc;

pub fn execute(toolchain: &str, settings: &GlobalSettings) -> Result<()> {
    // Parse the toolchain to get canonical format and directory name
    let toolchain_desc = ToolchainDesc::parse(toolchain)?;
    let canonical_name = toolchain_desc.to_string();
    let dir_name = toolchain_desc.to_directory_name();

    let toolchains_dir = Config::toolchains_dir()?;
    let toolchain_path = toolchains_dir.join(&dir_name);

    // Check if toolchain exists
    if !toolchain_path.exists() {
        anyhow::bail!(
            "Toolchain '{}' is not installed.\n\nRun 'lemma lean install {}' to install it first,\nor run 'lemma lean list' to see installed toolchains.",
            canonical_name,
            canonical_name
        );
    }

    // Load config
    let mut config = Config::load().unwrap_or_default();

    // Check if this is already the default
    if let Some(ref current_default) = config.default_toolchain {
        if current_default == &canonical_name {
            if settings.use_colors() {
                println!(
                    "{} '{}' is already the default toolchain",
                    "=>".cyan().bold(),
                    canonical_name
                );
            } else {
                println!("=> '{}' is already the default toolchain", canonical_name);
            }
            return Ok(());
        }
    }

    // Update default toolchain (store in canonical format)
    let old_default = config.default_toolchain.clone();
    config.default_toolchain = Some(canonical_name.clone());

    // Save config
    config.save().context("Failed to save configuration")?;

    // Show success message
    if let Some(old) = old_default {
        if settings.use_colors() {
            println!(
                "{} Default toolchain changed from '{}' to '{}'",
                "✓".green().bold(),
                old.dimmed(),
                canonical_name
            );
        } else {
            println!(
                "✓ Default toolchain changed from '{}' to '{}'",
                old, canonical_name
            );
        }
    } else {
        if settings.use_colors() {
            println!(
                "{} Default toolchain set to '{}'",
                "✓".green().bold(),
                canonical_name
            );
        } else {
            println!("✓ Default toolchain set to '{}'", canonical_name);
        }
    }

    Ok(())
}
