//! List command - Show installed toolchains

use anyhow::Result;
use colored::Colorize;

use lemma_config::{Config, GlobalSettings, ToolchainRegistry};
use lemma_download::{DownloadClient, ReleaseServerClient};
use lemma_output::Printer;

pub fn execute(
    only_installed: bool,
    only_available: bool,
    lean_downloads_json_url: Option<&str>,
    _settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    // Load config to get default toolchain
    let config = Config::load().unwrap_or_default();

    // Get active toolchain (from environment, override, project file, or default)
    let active_toolchain = lemma_config::resolve_toolchain(None)?;

    // Create toolchain registry
    let registry = ToolchainRegistry::new(&Config::lemma_home()?);

    let show_installed = !only_available;
    let show_available = !only_installed;

    // Collect installed toolchains
    let installed_toolchains = if show_installed {
        registry.list_installed()?
    } else {
        Vec::new()
    };

    // Fetch available toolchains from release server
    let mut available_releases = Vec::new();
    if show_available {
        printer.hint("Fetching available releases...")?;

        match fetch_available_releases(lean_downloads_json_url, &config) {
            Ok(releases) => available_releases = releases,
            Err(e) => {
                printer.warning(format!("Failed to fetch available releases: {}", e))?;
            }
        }
    }

    // Check if we have anything to show
    if installed_toolchains.is_empty() && available_releases.is_empty() {
        printer.status("No toolchains installed yet.")?;
        printer.hint("Run 'lemma toolchain install stable' to install the stable toolchain.")?;
        return Ok(());
    }

    // Display sections
    if show_installed && !installed_toolchains.is_empty() {
        printer.header("Installed toolchains")?;
        display_installed_toolchains(
            &installed_toolchains,
            &active_toolchain,
            &config,
            &registry,
            printer,
        )?;
    }

    if show_available && !available_releases.is_empty() {
        printer.header("Available for download")?;
        display_available_releases(&available_releases, printer)?;
    }

    Ok(())
}

/// Fetch available releases from the release server
fn fetch_available_releases(
    lean_downloads_json_url: Option<&str>,
    config: &Config,
) -> Result<Vec<(String, String)>> {
    let client = DownloadClient::new()?;
    let base_url = config.resolve_release_url(lean_downloads_json_url);
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
    toolchains: &[lemma_config::InstalledToolchain],
    active_toolchain: &Option<String>,
    config: &Config,
    registry: &ToolchainRegistry,
    printer: &Printer,
) -> Result<()> {
    for tc in toolchains {
        let name = &tc.name;

        // Check if this toolchain is active and/or default
        let is_active = if let Some(ref active_name) = active_toolchain {
            if let Some(ref desc) = tc.desc {
                registry.is_active(desc, active_name)
            } else {
                active_name == name
            }
        } else {
            false
        };

        let is_default = if let Some(ref desc) = tc.desc {
            registry.is_default(desc, config)
        } else {
            config.default_toolchain.as_ref() == Some(name)
        };

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
