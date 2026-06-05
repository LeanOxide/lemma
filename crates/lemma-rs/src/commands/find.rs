//! Find command - Find an installed toolchain matching a request

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::io::Write;

use lemma_config::{Config, GlobalSettings};
use lemma_output::Printer;
use lemma_toolchain::ToolchainDesc;

pub fn execute(request: Option<&str>, _settings: &GlobalSettings, printer: &Printer) -> Result<()> {
    let toolchains_dir = Config::toolchains_dir()?;

    // If no request given, show the active toolchain
    if request.is_none() {
        let active = lemma_config::resolve_toolchain(None)?;
        if let Some(toolchain_name) = active {
            // Get the directory name for this toolchain
            let desc = ToolchainDesc::parse(&toolchain_name)
                .with_context(|| format!("Invalid toolchain name: {}", toolchain_name))?;
            let dir_name = desc.to_directory_name();

            let display = if printer.use_colors() {
                dir_name.cyan().bold().to_string()
            } else {
                dir_name.to_string()
            };
            writeln!(printer.stdout(), "{}", display)?;
            return Ok(());
        } else {
            anyhow::bail!(
                "No active toolchain found. Run 'lemma default <toolchain>' to set a default."
            );
        }
    }

    let request_str = request.unwrap();

    // Check if toolchains directory exists
    if !toolchains_dir.exists() {
        anyhow::bail!(
            "No toolchains installed yet.\nRun 'lemma toolchain install {}' to install it.",
            request_str
        );
    }

    // Read all installed toolchains
    let entries = fs::read_dir(&toolchains_dir).with_context(|| {
        format!(
            "Failed to read toolchains directory: {}",
            toolchains_dir.display()
        )
    })?;

    let mut matching_toolchains = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip non-directories and temp directories
        if !path.is_dir()
            || path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .ends_with(".tmp")
        {
            continue;
        }

        let dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Parse the directory name to get the toolchain
        let desc = match ToolchainDesc::from_directory_name(&dir_name) {
            Ok(desc) => desc,
            Err(_) => continue, // Skip directories we can't parse
        };

        // Check if this toolchain matches the request
        if matches_request(&desc, request_str) {
            matching_toolchains.push((desc, dir_name, path));
        }
    }

    if matching_toolchains.is_empty() {
        anyhow::bail!(
            "No installed toolchain matches '{}'.\nRun 'lemma toolchain list' to see installed toolchains.",
            request_str
        );
    }

    // Sort by version (newest first) and take the first match
    matching_toolchains.sort_by(|a, b| {
        // Sort Remote toolchains by version, keep Local ones at the end
        use lemma_toolchain::ToolchainDesc;
        match (&a.0, &b.0) {
            (
                ToolchainDesc::Remote { release: r1, .. },
                ToolchainDesc::Remote { release: r2, .. },
            ) => {
                r2.cmp(r1) // Reverse order for newest first
            }
            (ToolchainDesc::Local { .. }, ToolchainDesc::Remote { .. }) => {
                std::cmp::Ordering::Greater
            }
            (ToolchainDesc::Remote { .. }, ToolchainDesc::Local { .. }) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Equal,
        }
    });

    let (_desc, dir_name, _path) = &matching_toolchains[0];

    // Print the unique identifier (directory name)
    let display = if printer.use_colors() {
        dir_name.cyan().bold().to_string()
    } else {
        dir_name.to_string()
    };
    writeln!(printer.stdout(), "{}", display)?;

    Ok(())
}

/// Check if a toolchain matches a request
fn matches_request(desc: &ToolchainDesc, request: &str) -> bool {
    match desc {
        ToolchainDesc::Remote { release, .. } => {
            // Match exact release name
            if release == request {
                return true;
            }

            // Match channel names
            if request == "stable" || request == "beta" || request == "nightly" {
                return release == request;
            }

            // Match version patterns (v4, v4.24, v4.24.0)
            if let Some(version) = release.strip_prefix('v') {
                if let Some(req_version) = request.strip_prefix('v') {
                    // Check if the version starts with the request
                    return version.starts_with(req_version);
                }
            }

            false
        }
        ToolchainDesc::Local { name } => {
            // For local toolchains, match by name
            name == request
        }
    }
}
