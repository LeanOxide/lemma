//! Link command - Create a symlink to a custom toolchain

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::config::Config;
use crate::settings::GlobalSettings;

pub fn execute(name: &str, path: &str, settings: &GlobalSettings) -> Result<()> {
    let source_path = Path::new(path);

    // Validate source path exists and is a directory
    if !source_path.exists() {
        anyhow::bail!("Source path does not exist: {}", path);
    }

    if !source_path.is_dir() {
        anyhow::bail!("Source path is not a directory: {}", path);
    }

    // Check if it looks like a valid toolchain (has bin/ directory)
    let bin_dir = source_path.join("bin");
    if !bin_dir.exists() || !bin_dir.is_dir() {
        if settings.use_colors() {
            println!(
                "{} Warning: Source directory doesn't have a 'bin' subdirectory. This might not be a valid toolchain.",
                "⚠".yellow().bold()
            );
        } else {
            println!("⚠ Warning: Source directory doesn't have a 'bin' subdirectory. This might not be a valid toolchain.");
        }
    }

    // Parse the toolchain name to get the sanitized directory name
    let toolchain_desc = crate::toolchain::ToolchainDesc::parse(name)?;
    let dir_name = toolchain_desc.to_directory_name();

    // Get target path
    let toolchains_dir = Config::toolchains_dir()?;
    let target_path = toolchains_dir.join(&dir_name);

    // Check if toolchain with this name already exists
    if target_path.exists() {
        anyhow::bail!(
            "A toolchain named '{}' already exists at: {}",
            name,
            target_path.display()
        );
    }

    // Ensure toolchains directory exists
    fs::create_dir_all(&toolchains_dir).context("Failed to create toolchains directory")?;

    // Create symlink
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source_path, &target_path).with_context(|| {
            format!(
                "Failed to create symlink from {} to {}",
                path,
                target_path.display()
            )
        })?;
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(source_path, &target_path).with_context(|| {
            format!(
                "Failed to create symlink from {} to {}",
                path,
                target_path.display()
            )
        })?;
    }

    if settings.use_colors() {
        println!(
            "{} Successfully linked toolchain '{}' to {}",
            "✓".green().bold(),
            name,
            path
        );
    } else {
        println!("✓ Successfully linked toolchain '{}' to {}", name, path);
    }

    println!("   Target: {}", target_path.display());

    Ok(())
}
