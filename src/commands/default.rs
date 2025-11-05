//! Default command - Set the default toolchain

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::Config;

pub fn execute(toolchain: &str) -> Result<()> {
    let toolchains_dir = Config::toolchains_dir()?;
    let toolchain_path = toolchains_dir.join(toolchain);

    // Check if toolchain exists
    if !toolchain_path.exists() {
        anyhow::bail!(
            "Toolchain '{}' is not installed.\n\nRun 'lemma toolchain install {}' to install it first,\nor run 'lemma toolchain list' to see installed toolchains.",
            toolchain,
            toolchain
        );
    }

    // Load config
    let mut config = Config::load().unwrap_or_default();

    // Check if this is already the default
    if let Some(ref current_default) = config.default_toolchain {
        if current_default == toolchain {
            println!(
                "{} '{}' is already the default toolchain",
                "=>".cyan().bold(),
                toolchain
            );
            return Ok(());
        }
    }

    // Update default toolchain
    let old_default = config.default_toolchain.clone();
    config.default_toolchain = Some(toolchain.to_string());

    // Save config
    config.save().context("Failed to save configuration")?;

    // Show success message
    if let Some(old) = old_default {
        println!(
            "{} Default toolchain changed from '{}' to '{}'",
            "✓".green().bold(),
            old.dimmed(),
            toolchain
        );
    } else {
        println!(
            "{} Default toolchain set to '{}'",
            "✓".green().bold(),
            toolchain
        );
    }

    Ok(())
}
