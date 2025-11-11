//! Upgrade command - Upgrade installed toolchains

use anyhow::Result;
use colored::Colorize;
use std::fs;

use lemma_config::Config;
use lemma_config::GlobalSettings;
use lemma_install::Installer;
use lemma_output::Printer;
use lemma_toolchain::ToolchainDesc;

pub fn execute(
    toolchain: Option<&str>,
    _settings: &GlobalSettings,
    #[allow(unused_variables)] printer: &Printer,
) -> Result<()> {
    if let Some(name) = toolchain {
        // Upgrade specific toolchain
        upgrade_toolchain(name)
    } else {
        // Upgrade all upgradeable toolchains
        upgrade_all_toolchains()
    }
}

fn upgrade_toolchain(name: &str) -> Result<()> {
    let installer = Installer::new()?;

    // Parse the toolchain descriptor first to get canonical name
    let toolchain_desc = ToolchainDesc::parse(name)?;
    let canonical_name = toolchain_desc.to_string();

    // Check if toolchain is installed
    if !installer.is_installed(name)? {
        println!(
            "{} Toolchain '{}' is not installed",
            "=>".yellow().bold(),
            name
        );
        println!("   Use 'lemma toolchain install {}' to install it", name);
        return Ok(());
    }

    println!("{} Checking for upgrades: {}", "=>".cyan().bold(), name);

    // Fetch the latest release information
    let release = installer.fetch_release(&toolchain_desc)?;

    // Load the current installed version hash using canonical name
    let current_hash = Config::load_update_hash(&canonical_name)?;

    // Compare versions
    if let Some(current) = current_hash {
        if current == release.name {
            println!(
                "{} Toolchain '{}' is already up to date ({})",
                "✓".green().bold(),
                name,
                release.name
            );
            return Ok(());
        }

        println!("   Current: {}", current);
        println!("   Latest:  {}", release.name);
    } else {
        println!("   Latest:  {}", release.name);
    }

    // Perform the upgrade
    println!("{} Upgrading toolchain...", "=>".cyan().bold());
    installer.install(name, true)?;

    Ok(())
}

fn upgrade_all_toolchains() -> Result<()> {
    println!("{} Upgrading all toolchains...", "=>".green().bold());
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
        if let Some(dir_name) = entry.file_name().to_str() {
            let path = entry.path();

            // Skip symlinks (linked toolchains)
            if path.is_symlink() {
                println!("   Skipping {} (linked toolchain)", dir_name);
                skipped_count += 1;
                continue;
            }

            // Parse directory name to get the canonical toolchain name
            let name = match lemma_toolchain::ToolchainDesc::from_directory_name(dir_name) {
                Ok(desc) => desc.to_string(),
                Err(_) => dir_name.to_string(),
            };

            // Skip specific versions
            if is_specific_version(&name) {
                println!("   Skipping {} (pinned version)", name);
                skipped_count += 1;
                continue;
            }

            // Check if upgrade is needed
            println!("   Checking {}...", name.bold());
            let installer = Installer::new()?;

            // Parse the toolchain descriptor
            let toolchain_desc = match ToolchainDesc::parse(&name) {
                Ok(d) => d,
                Err(e) => {
                    println!("   {} Failed to parse {}: {}", "✗".red(), name, e);
                    continue;
                }
            };

            // Fetch the latest release information
            let release = match installer.fetch_release(&toolchain_desc) {
                Ok(r) => r,
                Err(e) => {
                    println!(
                        "   {} Failed to fetch release for {}: {}",
                        "✗".red(),
                        name,
                        e
                    );
                    continue;
                }
            };

            // Load the current installed version hash
            let current_hash = Config::load_update_hash(&name)?;

            // Compare versions
            if let Some(current) = current_hash {
                if current == release.name {
                    println!("   {} Already up to date ({})", "✓".green(), release.name);
                    skipped_count += 1;
                    continue;
                }
                println!("   Current: {} → Latest: {}", current, release.name);
            } else {
                println!("   Latest: {}", release.name);
            }

            // Perform the upgrade
            match installer.install(&name, true) {
                Ok(_) => {
                    updated_count += 1;
                }
                Err(e) => {
                    println!("   {} Failed to upgrade {}: {}", "✗".red(), name, e);
                }
            }
            println!();
        }
    }

    println!();
    println!(
        "{} Upgraded {} toolchain(s), skipped {}",
        "✓".green().bold(),
        updated_count,
        skipped_count
    );

    Ok(())
}

/// Check if a toolchain name is a specific version (not a channel)
fn is_specific_version(name: &str) -> bool {
    // Use toolchain descriptor parsing for accurate detection
    if let Ok(desc) = ToolchainDesc::parse(name) {
        // Only tracking channels (stable, beta, nightly) should auto-upgrade
        // Return true if it's NOT a tracking channel (i.e., should skip upgrades)
        !desc.is_tracking_channel()
    } else {
        // Fallback to heuristic detection for edge cases
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

        // If parsing failed and doesn't look like a channel, treat as specific version
        !matches!(name, "stable" | "beta" | "nightly")
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_specific_version_channels() {
        // Channels should NOT be treated as specific versions (they should update)
        assert!(!is_specific_version("stable"));
        assert!(!is_specific_version("beta"));
        assert!(!is_specific_version("nightly"));
    }

    #[test]
    fn test_is_specific_version_versions() {
        // Version tags should be treated as specific versions (they should NOT update)
        assert!(is_specific_version("v4.24.0"));
        assert!(is_specific_version("v4.15.0-rc1"));
        assert!(is_specific_version("lean-4.24.0-linux"));
        assert!(is_specific_version("custom-1.2.3"));
    }

    #[test]
    fn test_is_specific_version_edge_cases() {
        // Edge cases should be handled correctly
        assert!(is_specific_version("some-v4.24.0-package"));
        // Unknown channel names should be treated as specific versions (conservative approach)
        assert!(is_specific_version("unknown-channel-name"));
    }
}
