//! Link command - Create a symlink to a custom toolchain

use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::Path;

use lemma_config::Config;
use lemma_config::GlobalSettings;
use lemma_output::Printer;

pub fn execute(name: &str, path: &str, _settings: &GlobalSettings, printer: &Printer) -> Result<()> {
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
        printer.warning("Source directory doesn't have a 'bin' subdirectory. This might not be a valid toolchain.")?;
    }

    // Parse the toolchain name to get the sanitized directory name
    let toolchain_desc = lemma_toolchain::ToolchainDesc::parse(name)?;
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

    printer.success(format!("Successfully linked toolchain '{}' to {}", name, path))?;
    writeln!(printer.stdout(), "   Target: {}", target_path.display())?;

    Ok(())
}
