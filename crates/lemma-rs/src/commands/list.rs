//! List command - Show installed toolchains

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::io::Write;

use lemma_config::Config;
use lemma_config::GlobalSettings;
use lemma_download::{DownloadClient, ReleaseServerClient};
use lemma_output::Printer;

pub fn execute(
    only_installed: bool,
    only_available: bool,
    _settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    // Load config to get default toolchain
    let config = Config::load().unwrap_or_default();

    // Get active toolchain (from environment, override, project file, or default)
    let active_toolchain = lemma_config::resolve_toolchain(None)?;

    let toolchains_dir = Config::toolchains_dir()?;

    let show_installed = !only_available;
    let show_available = !only_installed;

    // Collect installed toolchains
    let mut installed_toolchains = Vec::new();

    if show_installed && toolchains_dir.exists() {
        installed_toolchains = collect_installed_toolchains(&toolchains_dir)?;
    }

    // Fetch available toolchains from release server
    let mut available_releases = Vec::new();
    if show_available {
        printer.hint("Fetching available releases...")?;

        match fetch_available_releases(&config) {
            Ok(releases) => available_releases = releases,
            Err(e) => {
                printer.warning(format!("Failed to fetch available releases: {}", e))?;
            }
        }
    }

    // Check if we have anything to show
    if installed_toolchains.is_empty() && available_releases.is_empty() {
        printer.status("No toolchains installed yet.")?;
        writeln!(
            printer.stdout(),
            "   Run 'lemma lean install stable' to install the stable toolchain."
        )?;
        return Ok(());
    }

    // Display sections
    if show_installed && !installed_toolchains.is_empty() {
        printer.header("Installed toolchains")?;
        display_installed_toolchains(&installed_toolchains, &active_toolchain, &config, printer)?;
    }

    if show_available && !available_releases.is_empty() {
        if !installed_toolchains.is_empty() {
            writeln!(printer.stdout())?;
        }
        printer.header("Available for download")?;
        display_available_releases(&available_releases, printer)?;
    }

    Ok(())
}

/// Collect installed toolchains from the toolchains directory
fn collect_installed_toolchains(
    toolchains_dir: &std::path::Path,
) -> Result<
    Vec<(
        String,
        String,
        std::path::PathBuf,
        Option<lemma_toolchain::ToolchainDesc>,
    )>,
> {
    let entries = fs::read_dir(toolchains_dir).with_context(|| {
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

    Ok(toolchains)
}

/// Fetch available releases from the release server
fn fetch_available_releases(config: &Config) -> Result<Vec<(String, String)>> {
    let client = DownloadClient::new()?;
    let base_url = config.lean_release_url();
    let release_client = ReleaseServerClient::new(client, base_url);

    let index = release_client.fetch_index()?;
    let mut releases = Vec::new();

    // Add latest stable
    if let Some(stable) = index.stable.first() {
        releases.push(("stable".to_string(), stable.name.clone()));
    }

    // Add latest beta
    if let Some(beta) = index.beta.first() {
        releases.push(("beta".to_string(), beta.name.clone()));
    }

    // Add latest nightly
    if let Some(nightly) = index.nightly.first() {
        releases.push(("nightly".to_string(), nightly.name.clone()));
    }

    // Add recent specific versions from stable (last 5)
    for release in index.stable.iter().take(5) {
        let version = release.name.clone();
        if !releases.iter().any(|(_, v)| v == &version) {
            releases.push((version.clone(), version));
        }
    }

    Ok(releases)
}

/// Display installed toolchains
fn display_installed_toolchains(
    toolchains: &[(
        String,
        String,
        std::path::PathBuf,
        Option<lemma_toolchain::ToolchainDesc>,
    )],
    active_toolchain: &Option<String>,
    config: &Config,
    printer: &Printer,
) -> Result<()> {
    for (name, _dir_name, _path, _desc) in toolchains {
        // Check if this toolchain is active and/or default
        let is_active = active_toolchain.as_ref() == Some(name);
        let is_default = config.default_toolchain.as_ref() == Some(name);

        // Build status string
        let status = match (is_active, is_default) {
            (true, true) => " (active, default)",
            (true, false) => " (active)",
            (false, true) => " (default)",
            (false, false) => "",
        };

        let display = if printer.use_colors() {
            let status_colored = if is_active || is_default {
                status.green().to_string()
            } else {
                status.to_string()
            };
            format!("{}{}", name, status_colored)
        } else {
            format!("{}{}", name, status)
        };

        printer.list_item(display)?;
    }

    Ok(())
}

/// Display available releases from the release server
fn display_available_releases(releases: &[(String, String)], printer: &Printer) -> Result<()> {
    for (channel, version) in releases {
        let display = if channel == version {
            // For specific versions, just show once
            channel.to_string()
        } else {
            // For channels, show channel -> version
            if printer.use_colors() {
                format!("{} {}", channel, format!("({})", version).dimmed())
            } else {
                format!("{} ({})", channel, version)
            }
        };

        printer.list_item(display)?;
    }

    Ok(())
}
