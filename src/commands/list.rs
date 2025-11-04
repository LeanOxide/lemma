//! List command - Show installed toolchains

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use crate::config::Config;

pub fn execute(verbose: bool) -> Result<()> {
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

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip temp directories (ending with .tmp)
        if name.ends_with(".tmp") {
            continue;
        }

        toolchains.push((name, path));
    }

    // Sort toolchains by name
    toolchains.sort_by(|a, b| a.0.cmp(&b.0));

    if toolchains.is_empty() {
        println!("{} No toolchains installed yet.", "=>".yellow().bold());
        println!("   Run 'lemma install stable' to install the stable toolchain.");
        return Ok(());
    }

    println!("{} Installed toolchains:", "=>".green().bold());
    println!();

    for (name, path) in toolchains {
        if verbose {
            // Show detailed information
            let metadata = fs::metadata(&path)?;
            let size = calculate_dir_size(&path)?;
            let modified = metadata.modified()?;

            println!("  {} {}", "•".cyan(), name.bright_white().bold());
            println!("    Path: {}", path.display().to_string().dimmed());
            println!("    Size: {}", format_size(size).dimmed());

            // Try to find lean binary and get version
            if let Ok(version) = get_lean_version(&path) {
                println!("    Version: {}", version.dimmed());
            }

            // Show modification time
            if let Ok(duration) = modified.elapsed() {
                println!("    Installed: {}", format_duration(duration).dimmed());
            }
            println!();
        } else {
            // Simple list
            println!("  {} {}", "•".cyan(), name.bright_white());
        }
    }

    if !verbose {
        println!();
        println!(
            "{} Use 'lemma list --verbose' for more details",
            "Tip:".dimmed()
        );
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

/// Format duration in human-readable format
fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();

    if secs < 60 {
        format!("{} seconds ago", secs)
    } else if secs < 3600 {
        format!("{} minutes ago", secs / 60)
    } else if secs < 86400 {
        format!("{} hours ago", secs / 3600)
    } else {
        format!("{} days ago", secs / 86400)
    }
}

/// Get Lean version from installation
fn get_lean_version(install_path: &std::path::Path) -> Result<String> {
    // Try to find lean binary
    let lean_bin = install_path.join("bin").join("lean");

    if !lean_bin.exists() {
        return Ok("unknown".to_string());
    }

    // Try to run lean --version
    let output = std::process::Command::new(&lean_bin)
        .arg("--version")
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            if let Ok(version) = String::from_utf8(output.stdout) {
                // Parse version from output (usually "Lean (version 4.x.x, ...)")
                if let Some(version) = version.split_whitespace().nth(2) {
                    return Ok(version.trim_end_matches(',').to_string());
                }
                return Ok(version.lines().next().unwrap_or("unknown").to_string());
            }
        }
    }

    Ok("unknown".to_string())
}
