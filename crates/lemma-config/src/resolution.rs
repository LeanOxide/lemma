//! Toolchain resolution logic
//!
//! This module handles resolving which toolchain to use based on:
//! - Explicit command-line overrides
//! - Environment variables
//! - Directory overrides
//! - Project files
//! - Default configuration
//!
//! It depends on both the Config and ToolchainDesc types.

use anyhow::{Context, Result};
use lemma_static::EnvVars;
use std::env;
use std::path::{Path, PathBuf};

use crate::config::Config;
use lemma_toolchain::{find_project_toolchain, get_lean_version, ToolchainDesc, ToolchainSource};

/// Resolve the active toolchain with detailed source information
///
/// Returns the toolchain name and the source where it was found
pub fn resolve_toolchain_with_source(
    explicit: Option<&str>,
) -> Result<Option<(String, ToolchainSource)>> {
    // 1. Explicit command-line override
    if let Some(toolchain) = explicit {
        return Ok(Some((toolchain.to_string(), ToolchainSource::Explicit)));
    }

    // 2. Environment variable override
    if let Ok(toolchain) = env::var(EnvVars::LEMMA_TOOLCHAIN) {
        return Ok(Some((toolchain, ToolchainSource::Environment)));
    }

    // Load config once for both override and default checks
    let config = Config::load().context("Failed to load configuration")?;

    // 3. Directory override (walks up from current directory)
    if let Ok(current_dir) = env::current_dir() {
        if let Some((path, toolchain)) = config.find_override(&current_dir) {
            return Ok(Some((toolchain, ToolchainSource::Override(path))));
        }
    }

    // 4. Project-specific configuration files
    if let Ok(current_dir) = env::current_dir() {
        if let Some((toolchain, path)) = find_project_toolchain(&current_dir)? {
            return Ok(Some((toolchain, ToolchainSource::ProjectFile(path))));
        }
    }

    // 5. Default toolchain from config
    if let Some(default) = config.default_toolchain {
        return Ok(Some((default, ToolchainSource::Default)));
    }

    Ok(None)
}

/// Resolve the active toolchain, returning just the name
///
/// This is a convenience wrapper around `resolve_toolchain_with_source`
pub fn resolve_toolchain(explicit: Option<&str>) -> Result<Option<String>> {
    Ok(resolve_toolchain_with_source(explicit)?.map(|(name, _)| name))
}

/// Resolve the active toolchain, returning an error if none is found
///
/// This is useful for commands that require an active toolchain
pub fn resolve_toolchain_or_fail(explicit: Option<&str>) -> Result<String> {
    resolve_toolchain(explicit)?.ok_or_else(|| {
        anyhow::anyhow!(
            "No active toolchain found.\n\n\
             Set a default with: lemma default <toolchain>\n\
             Or install a toolchain with: lemma toolchain install stable"
        )
    })
}

/// Find a tracking channel (stable/beta/nightly) that has the requested version
///
/// This allows using version-specific lean-toolchain files even when only
/// channels are installed. For example, if `stable` is installed and points
/// to v4.24.0, requesting v4.24.0 will find the stable installation.
fn find_channel_with_version(
    toolchains_dir: &Path,
    requested: &ToolchainDesc,
) -> Result<Option<PathBuf>> {
    let channels = ["stable", "beta", "nightly", "latest"];

    for channel in &channels {
        let channel_desc = ToolchainDesc::parse(channel)?;
        let channel_dir = toolchains_dir.join(channel_desc.to_directory_name());

        if channel_dir.exists() {
            // Check the version of the installed channel
            if let Ok(version) = get_lean_version(&channel_dir) {
                // Extract version number from output like "Lean (version 4.24.0, ...)"
                if let Some(installed_version) = extract_version_from_lean_output(&version) {
                    let requested_version = requested.release().trim_start_matches('v');

                    if installed_version == requested_version {
                        return Ok(Some(channel_dir));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Extract version number from `lean --version` output
fn extract_version_from_lean_output(output: &str) -> Option<String> {
    // Output format: "Lean (version 4.24.0, commit ..., Release)"
    // We want to extract "4.24.0"
    if let Some(start) = output.find("version ") {
        let version_part = &output[start + 8..];
        if let Some(end) = version_part.find(',').or_else(|| version_part.find(')')) {
            return Some(version_part[..end].trim().to_string());
        }
    }
    None
}

/// Find the path to a tool binary in the specified toolchain
///
/// This function looks for a tool (lean, lake, etc.) in the toolchain directory
/// and returns its absolute path. It handles platform-specific executable naming
/// (e.g., .exe on Windows).
pub fn find_tool_binary(toolchain: &str, tool_name: &str) -> Result<PathBuf> {
    let toolchains_dir = Config::toolchains_dir()?;

    // Parse the toolchain to get the sanitized directory name
    let toolchain_desc = ToolchainDesc::parse(toolchain)?;
    let dir_name = toolchain_desc.to_directory_name();
    let mut toolchain_path = toolchains_dir.join(&dir_name);

    // Check if toolchain exists
    if !toolchain_path.exists() {
        // Try fallback: if this is a version request, check if any tracking channel
        // (stable/beta/nightly) is installed that might have this version
        if !toolchain_desc.is_tracking_channel() {
            if let Some(fallback_path) =
                find_channel_with_version(&toolchains_dir, &toolchain_desc)?
            {
                toolchain_path = fallback_path;
            } else {
                anyhow::bail!(
                    "Toolchain '{}' is not installed.\n\n\
                     Install it with: lemma toolchain install {}\n\
                     Or install a channel: lemma toolchain install stable",
                    toolchain,
                    toolchain
                );
            }
        } else {
            anyhow::bail!(
                "Toolchain '{}' is not installed.\n\n\
                 Install it with: lemma toolchain install {}",
                toolchain,
                toolchain
            );
        }
    }

    // Common locations for tool binaries
    let bin_name = if cfg!(target_os = "windows") {
        format!("{}.exe", tool_name)
    } else {
        tool_name.to_string()
    };

    let candidates = vec![
        toolchain_path.join("bin").join(&bin_name),
        toolchain_path.join(&bin_name),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    anyhow::bail!(
        "Tool '{}' not found in toolchain '{}'.\n\
         Expected location: {}",
        tool_name,
        toolchain,
        toolchain_path.join("bin").join(&bin_name).display()
    )
}
