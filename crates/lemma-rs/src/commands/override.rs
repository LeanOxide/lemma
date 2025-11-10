//! Override command - Manage directory-specific toolchain overrides

use anyhow::{Context, Result};
use colored::Colorize;
use std::env;
use std::path::PathBuf;

use crate::cli::OverrideCommands;
use crate::config::Config;
use crate::settings::GlobalSettings;
use crate::toolchain::ToolchainDesc;

pub fn execute(command: OverrideCommands, settings: &GlobalSettings) -> Result<()> {
    match command {
        OverrideCommands::Set { toolchain, path } => set_override(&toolchain, path, settings),
        OverrideCommands::Unset { path } => unset_override(path, settings),
        OverrideCommands::List => list_overrides(settings),
    }
}

/// Set a directory override
fn set_override(toolchain: &str, path: Option<String>, _settings: &GlobalSettings) -> Result<()> {
    let target_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        env::current_dir().context("Failed to get current directory")?
    };

    // Check if directory exists
    if !target_path.exists() {
        anyhow::bail!("Directory does not exist: {}", target_path.display());
    }

    if !target_path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", target_path.display());
    }

    // Parse the toolchain to get canonical format and directory name
    let toolchain_desc = ToolchainDesc::parse(toolchain)?;
    let canonical_name = toolchain_desc.to_string();
    let dir_name = toolchain_desc.to_directory_name();

    // Check if toolchain exists
    let toolchains_dir = Config::toolchains_dir()?;
    let toolchain_path = toolchains_dir.join(&dir_name);
    if !toolchain_path.exists() {
        anyhow::bail!(
            "Toolchain '{}' is not installed.\n\n\
             Install it with: lemma toolchain install {}",
            canonical_name,
            canonical_name
        );
    }

    // Load config and set override (store in canonical format)
    let mut config = Config::load().unwrap_or_default();
    config.set_override(target_path.clone(), canonical_name.clone())?;
    config.save()?;

    let canonical_path = target_path
        .canonicalize()
        .context("Failed to canonicalize path")?;

    println!(
        "{} Override set for directory: {}",
        "✓".green().bold(),
        canonical_path.display()
    );
    println!("   Toolchain: {}", canonical_name);

    Ok(())
}

/// Remove a directory override
fn unset_override(path: Option<String>, _settings: &GlobalSettings) -> Result<()> {
    let target_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        env::current_dir().context("Failed to get current directory")?
    };

    // Load config
    let mut config = Config::load()?;

    // Remove override
    if config.remove_override(&target_path)? {
        config.save()?;

        let canonical_path = target_path
            .canonicalize()
            .context("Failed to canonicalize path")?;

        println!(
            "{} Override removed for directory: {}",
            "✓".green().bold(),
            canonical_path.display()
        );
    } else {
        let canonical_path = target_path
            .canonicalize()
            .context("Failed to canonicalize path")?;

        println!(
            "{} No override found for directory: {}",
            "=>".yellow().bold(),
            canonical_path.display()
        );
    }

    Ok(())
}

/// List all directory overrides
fn list_overrides(_settings: &GlobalSettings) -> Result<()> {
    let config = Config::load()?;

    if config.overrides.is_empty() {
        println!("{} No directory overrides configured", "=>".cyan().bold());
        println!();
        println!("Set an override with: lemma override set <toolchain>");
        return Ok(());
    }

    println!("{}", "Directory overrides".green().bold());
    println!("{}", "-------------------".green().bold());
    println!();

    // Sort by path for consistent output
    let mut overrides: Vec<_> = config.overrides.iter().collect();
    overrides.sort_by_key(|(path, _)| path.as_str());

    for (path, toolchain) in overrides {
        println!("  {} {}", "→".cyan(), path);
        println!("    Toolchain: {}", toolchain);
        println!();
    }

    Ok(())
}
