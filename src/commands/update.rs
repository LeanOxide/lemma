//! Update command - Update installed toolchains

use anyhow::Result;
use colored::Colorize;
use std::fs;

use crate::config::Config;
use crate::install::Installer;

pub fn execute(toolchain: Option<&str>) -> Result<()> {
    if let Some(name) = toolchain {
        // Update specific toolchain
        update_toolchain(name)
    } else {
        // Update all updateable toolchains
        update_all_toolchains()
    }
}

fn update_toolchain(name: &str) -> Result<()> {
    // Like rustup, `update <toolchain>` is the same as `toolchain install <toolchain>`
    // This will install if not present, or reinstall/update if already installed
    let config = Config::load()?;
    let installer = Installer::new(config)?;

    installer.install(name, true)?;

    Ok(())
}

fn update_all_toolchains() -> Result<()> {
    println!("{} Updating all toolchains...", "=>".green().bold());
    println!();

    let toolchains_dir = Config::toolchains_dir()?;

    if !toolchains_dir.exists() {
        println!("{} No toolchains installed", "=>".yellow().bold());
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(&toolchains_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();

    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        println!("{} No toolchains installed", "=>".yellow().bold());
        return Ok(());
    }

    let mut updated_count = 0;
    let mut skipped_count = 0;

    for entry in entries {
        if let Some(name) = entry.file_name().to_str() {
            let path = entry.path();

            // Skip symlinks (linked toolchains)
            if path.is_symlink() {
                println!("   Skipping {} (linked toolchain)", name);
                skipped_count += 1;
                continue;
            }

            // Skip specific versions
            if is_specific_version(name) {
                println!("   Skipping {} (pinned version)", name);
                skipped_count += 1;
                continue;
            }

            // Update this toolchain
            println!("   Updating {}...", name.bold());
            let config = Config::load()?;
            let installer = Installer::new(config)?;

            match installer.install(name, true) {
                Ok(_) => {
                    updated_count += 1;
                }
                Err(e) => {
                    println!("   {} Failed to update {}: {}", "✗".red(), name, e);
                }
            }
            println!();
        }
    }

    println!();
    println!(
        "{} Updated {} toolchain(s), skipped {}",
        "✓".green().bold(),
        updated_count,
        skipped_count
    );

    Ok(())
}

/// Check if a toolchain name is a specific version (not a channel)
fn is_specific_version(name: &str) -> bool {
    // If it starts with 'v' and has dots, it's likely a version
    // Examples: v4.24.0, v4.15.0-rc1
    if name.starts_with('v') && name.contains('.') {
        return true;
    }

    // Check for version pattern like "X.Y.Z" in the name
    // Examples: lean-4.24.0-linux, custom-1.2.3
    if contains_version_pattern(name) {
        return true;
    }

    // If it contains a full URL, it's from DirectUrl
    if name.contains("http") {
        return true;
    }

    // Channels that should update: stable, latest, nightly (if supported)
    false
}

/// Check if a name contains a semantic version pattern (X.Y.Z)
fn contains_version_pattern(name: &str) -> bool {
    // Look for pattern like "4.24.0" or "1.2.3"
    let parts: Vec<&str> = name
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .collect();

    for part in parts {
        if part.is_empty() {
            continue;
        }

        // Check if this part looks like a version (has at least 2 dots and digits)
        let dots = part.matches('.').count();
        let digits = part.chars().filter(|c| c.is_ascii_digit()).count();

        if dots >= 2 && digits >= 3 {
            return true;
        }
    }

    false
}
