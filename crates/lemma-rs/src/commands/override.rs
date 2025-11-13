//! Override command - Manage directory-specific toolchain overrides

use anyhow::{Context, Result};
use std::env;
use std::path::PathBuf;

use lemma_cli::OverrideCommands;
use lemma_config::Config;
use lemma_config::GlobalSettings;
use lemma_output::Printer;
use lemma_toolchain::ToolchainDesc;

pub fn execute(
    command: OverrideCommands,
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    match command {
        OverrideCommands::Set { toolchain, path } => set_override(&toolchain, path, settings, printer),
        OverrideCommands::Unset { path } => unset_override(path, settings, printer),
        OverrideCommands::List => list_overrides(settings, printer),
    }
}

/// Set a directory override
fn set_override(toolchain: &str, path: Option<String>, settings: &GlobalSettings, printer: &Printer) -> Result<()> {
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

    printer.success(format!("Override set for directory: {}", canonical_path.display()))?;
    if settings.is_verbose() {
        printer.hint(format!("Toolchain: {}", canonical_name))?;
    }

    Ok(())
}

/// Remove a directory override
fn unset_override(path: Option<String>, _settings: &GlobalSettings, printer: &Printer) -> Result<()> {
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

        printer.success(format!("Override removed for directory: {}", canonical_path.display()))?;
    } else {
        let canonical_path = target_path
            .canonicalize()
            .context("Failed to canonicalize path")?;

        printer.warning(format!("No override found for directory: {}", canonical_path.display()))?;
    }

    Ok(())
}

/// List all directory overrides
fn list_overrides(_settings: &GlobalSettings, printer: &Printer) -> Result<()> {
    let config = Config::load()?;

    if config.overrides.is_empty() {
        printer.hint("No directory overrides configured")?;
        printer.hint("Set an override with: lemma override set <toolchain>")?;
        return Ok(());
    }

    printer.header("Directory overrides")?;

    // Sort by path for consistent output
    let mut overrides: Vec<_> = config.overrides.iter().collect();
    overrides.sort_by_key(|(path, _)| path.as_str());

    for (path, toolchain) in overrides {
        printer.list_item(format!("{} → {}", path, toolchain))?;
    }

    Ok(())
}
