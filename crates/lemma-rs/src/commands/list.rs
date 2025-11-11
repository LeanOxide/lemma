//! List command - Show installed toolchains

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use lemma_config::Config;
use lemma_config::GlobalSettings;
use lemma_toolchain as toolchain;

pub fn execute(settings: &GlobalSettings) -> Result<()> {
    // Load config to get default toolchain
    let config = Config::load().unwrap_or_default();

    // Get active toolchain (from environment, override, project file, or default)
    let active_toolchain = lemma_config::resolve_toolchain(None)?;

    let toolchains_dir = Config::toolchains_dir()?;

    // Check if toolchains directory exists
    if !toolchains_dir.exists() {
        println!("{} No toolchains installed yet.", "=>".yellow().bold());
        println!("   Run 'lemma install stable' to install the stable toolchain.");
        return Ok(());
    }

    // Read directory contents
    let entries = fs::read_dir(&toolchains_dir).with_context(|| {
        format!(
            "Failed to read toolchains directory: {}",
            toolchains_dir.display()
        )
    })?;

    let mut toolchains = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip non-directories and temp directories
        if !path.is_dir() {
            continue;
        }

        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip temp directories (ending with .tmp)
        if dir_name.ends_with(".tmp") {
            continue;
        }

        // Parse the directory name to get the canonical toolchain name and desc
        let (name, desc) = match lemma_toolchain::ToolchainDesc::from_directory_name(&dir_name) {
            Ok(desc) => {
                let name = desc.to_string();
                (name, Some(desc))
            }
            Err(_) => (dir_name.clone(), None), // Fallback to directory name if parsing fails
        };

        toolchains.push((name, dir_name, path, desc));
    }

    // Sort toolchains by name
    toolchains.sort_by(|a, b| a.0.cmp(&b.0));

    if toolchains.is_empty() {
        println!("{} No toolchains installed yet.", "=>".yellow().bold());
        println!("   Run 'lemma install stable' to install the stable toolchain.");
        return Ok(());
    }

    for (name, dir_name, _path, _desc) in toolchains {
        // Check if this toolchain is active and/or default
        let is_active = active_toolchain.as_ref() == Some(&name);
        let is_default = config.default_toolchain.as_ref() == Some(&name);

        // Build status string
        let _status = match (is_active, is_default) {
            (true, true) => " (active, default)".green(),
            (true, false) => " (active)".green(),
            (false, true) => " (default)".dimmed(),
            (false, false) => "".normal(),
        };

        if settings.is_verbose() {
            // Show detailed information when verbose
            let size = calculate_dir_size(&_path)?;

            if settings.use_colors() {
                // Show the name and status
                println!("{} {}{}", "•".cyan(), name.bold(), _status);
                // Show the unique identifier (directory name) which includes platform
                println!("  Key: {}", dir_name.dimmed());
                println!("  Path: {}", _path.display().to_string().dimmed());
                println!("  Size: {}", format_size(size).dimmed());
            } else {
                println!("• {} {}", name, _status);
                println!("  Key: {}", dir_name);
                println!("  Path: {}", _path.display());
                println!("  Size: {}", format_size(size));
            }

            // Try to find lean binary and get version
            let version = toolchain::get_lean_version_or_unknown(&_path);
            if version != "unknown" {
                if settings.use_colors() {
                    println!("  Version: {}", version.dimmed());
                } else {
                    println!("  Version: {}", version);
                }
            }
            println!();
        } else {
            // Simple list - show both name and unique key
            if settings.use_colors() {
                println!("{} {} {}{}", "•".cyan(), name, format!("({})", dir_name).dimmed(), _status);
            } else {
                println!("• {} ({}){}", name, dir_name, _status);
            }
        }
    }

    if !settings.is_verbose() && !settings.is_quiet() {
        println!();
        if settings.use_colors() {
            println!(
                "{} Use 'lemma lean list -v' for more details",
                "Tip:".dimmed()
            );
        } else {
            println!("Tip: Use 'lemma lean list -v' for more details");
        }
    }

    Ok(())
}

/// Calculate total size of a directory recursively
fn calculate_dir_size(path: &std::path::Path) -> Result<u64> {
    let mut total = 0;

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                total += calculate_dir_size(&path)?;
            } else {
                total += entry.metadata()?.len();
            }
        }
    }

    Ok(total)
}

/// Format size in human-readable format
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_idx])
}
