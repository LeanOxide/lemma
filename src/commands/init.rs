//! Init command - Initialize lemma configuration

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::commands::proxy_mode;
use crate::config::Config;

pub fn execute(no_path: bool, default_toolchain: &str) -> Result<()> {
    println!("{} Initializing lemma...", "=>".green().bold());

    // Create lemma home and bin directory
    let lemma_home = Config::lemma_home()?;
    let bin_dir = lemma_home.join("bin");
    fs::create_dir_all(&bin_dir).context("Failed to create bin directory")?;

    // Create config
    let config = Config::default();
    config.save()?;

    let config_path = Config::config_path()?;
    println!("   Created config at: {}", config_path.display());

    // Install proxy binaries
    println!("\n{} Installing proxy binaries...", "=>".cyan().bold());
    install_proxies(&bin_dir)?;

    if !no_path {
        println!(
            "\n{} Add the following to your shell profile:",
            "Note:".yellow().bold()
        );
        println!("   export PATH=\"{}:$PATH\"", bin_dir.display());
    }

    println!(
        "\n{} Run 'lemma install {}' to install the default toolchain",
        "Next:".green().bold(),
        default_toolchain
    );

    Ok(())
}

/// Install proxy binaries (symlinks to lemma executable)
fn install_proxies(bin_dir: &Path) -> Result<()> {
    // Get the path to the current lemma executable
    let lemma_exe = std::env::current_exe().context("Failed to determine lemma executable path")?;

    for tool in proxy_mode::PROXY_TOOLS {
        let tool_path = bin_dir.join(tool);

        // Remove existing symlink/file if present
        if tool_path.exists() || tool_path.symlink_metadata().is_ok() {
            let _ = fs::remove_file(&tool_path); // Best effort
        }

        // Try to create symlink first (preferred), fall back to hardlink
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            if let Err(e) = symlink(&lemma_exe, &tool_path) {
                // Symlink failed, try hardlink
                if let Err(e2) = fs::hard_link(&lemma_exe, &tool_path) {
                    anyhow::bail!(
                        "Failed to create link for '{}': symlink error: {}, hardlink error: {}",
                        tool,
                        e,
                        e2
                    );
                }
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix systems (Windows), try hardlink
            fs::hard_link(&lemma_exe, &tool_path)
                .with_context(|| format!("Failed to create link for '{}'", tool))?;
        }

        println!("   Installed: {}", tool);
    }

    println!(
        "   {} proxy binaries installed",
        proxy_mode::PROXY_TOOLS.len()
    );

    Ok(())
}
