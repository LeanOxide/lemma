//! Dir command - Show toolchain installation directory

use anyhow::{Context, Result};
use colored::Colorize;

use lemma_config::{Config, GlobalSettings};
use lemma_toolchain::ToolchainDesc;

pub fn execute(toolchain: Option<&str>, settings: &GlobalSettings) -> Result<()> {
    let toolchains_dir = Config::toolchains_dir()?;

    if let Some(toolchain_name) = toolchain {
        // Show directory for a specific toolchain
        let desc = ToolchainDesc::parse(toolchain_name)
            .with_context(|| format!("Invalid toolchain name: {}", toolchain_name))?;

        let dir_name = desc.to_directory_name();
        let toolchain_path = toolchains_dir.join(&dir_name);

        if !toolchain_path.exists() {
            anyhow::bail!(
                "Toolchain '{}' is not installed.\nRun 'lemma lean install {}' to install it.",
                toolchain_name,
                toolchain_name
            );
        }

        // Print the full path to the toolchain
        if settings.use_colors() {
            println!("{}", toolchain_path.display().to_string().cyan());
        } else {
            println!("{}", toolchain_path.display());
        }
    } else {
        // Show the root toolchains directory
        if settings.use_colors() {
            println!("{}", toolchains_dir.display().to_string().cyan());
        } else {
            println!("{}", toolchains_dir.display());
        }
    }

    Ok(())
}
