//! Show command - Display active toolchain information

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::config::Config;
use crate::toolchain;

pub fn execute() -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    // Determine the active toolchain
    let active_toolchain = toolchain::resolve_toolchain_with_source(None)?;

    // Display active toolchain
    if let Some((toolchain, src)) = &active_toolchain {
        println!("{}", "Active toolchain".green().bold());
        println!("----------------");
        println!();
        println!("  {}", toolchain);
        println!("  ({})", src.to_string().dimmed());
        println!();

        // Show toolchain path if it exists
        let toolchains_dir = Config::toolchains_dir()?;
        let toolchain_path = toolchains_dir.join(toolchain);
        if toolchain_path.exists() {
            println!("  Path: {}", toolchain_path.display());

            // Try to get lean version
            if let Ok(version) = get_lean_version(&toolchain_path) {
                println!("  Lean: {}", version.trim());
            }
        } else {
            println!("  {} Toolchain not installed", "Warning:".yellow().bold());
        }
        println!();
    } else {
        println!("{} No active toolchain found", "Warning:".yellow().bold());
        println!();
        println!("Set a default with: lemma default <toolchain>");
        println!();
    }

    // Show default toolchain
    if let Some(ref default) = config.default_toolchain {
        println!("{}", "Default toolchain".green().bold());
        println!("-----------------");
        println!("  {}", default);
        println!();
    }

    // List installed toolchains
    println!("{}", "Installed toolchains".green().bold());
    println!("--------------------");

    let toolchains_dir = Config::toolchains_dir()?;
    if toolchains_dir.exists() {
        let mut entries: Vec<_> = fs::read_dir(&toolchains_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        entries.sort_by_key(|e| e.file_name());

        if entries.is_empty() {
            println!("  {}", "No toolchains installed".dimmed());
        } else {
            for entry in entries {
                if let Some(name) = entry.file_name().to_str() {
                    // Check if this is the active toolchain
                    let is_active = active_toolchain
                        .as_ref()
                        .map(|(tc, _)| tc == name)
                        .unwrap_or(false);

                    if is_active {
                        println!("  {} {}", "•".cyan(), name);
                    } else {
                        println!("    {}", name);
                    }
                }
            }
        }
    } else {
        println!("  {}", "No toolchains installed".dimmed());
    }

    println!();

    Ok(())
}

/// Get the Lean version from a toolchain
fn get_lean_version(toolchain_path: &Path) -> Result<String> {
    let lean_bin = toolchain_path.join("bin").join("lean");

    if !lean_bin.exists() {
        anyhow::bail!("lean binary not found");
    }

    let output = std::process::Command::new(&lean_bin)
        .arg("--version")
        .output()
        .context("Failed to execute lean --version")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        anyhow::bail!("lean --version failed")
    }
}
