//! Show command - Display active toolchain information

use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;

use crate::config::Config;
use crate::toolchain;

pub fn execute() -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    // Show platform and lemma home
    println!("{} {}", "Default host:".bold(), get_host_triple());
    if let Ok(lemma_home) = Config::lemma_home() {
        println!("{} {}", "lemma home:".bold(), lemma_home.display());
    }
    println!();

    // Determine the active toolchain
    let active_toolchain = toolchain::resolve_toolchain_with_source(None)?;

    // List installed toolchains first
    println!("{}", "installed toolchains".bold());
    println!("{}", "--------------------".bold());

    let toolchains_dir = Config::toolchains_dir()?;
    let mut has_toolchains = false;

    if toolchains_dir.exists() {
        let mut entries: Vec<_> = fs::read_dir(&toolchains_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        entries.sort_by_key(|e| e.file_name());

        if !entries.is_empty() {
            has_toolchains = true;

            for entry in entries {
                if let Some(dir_name) = entry.file_name().to_str() {
                    // Parse directory name to get canonical toolchain name
                    let name = match crate::toolchain::ToolchainDesc::from_directory_name(dir_name)
                    {
                        Ok(desc) => desc.to_string(),
                        Err(_) => dir_name.to_string(),
                    };

                    // Build status indicators (active, default)
                    let mut status_parts = Vec::new();

                    // Check if active
                    let is_active = if let Some((active_tc, _)) = &active_toolchain {
                        if active_tc == &name {
                            true
                        } else {
                            // Check fallback
                            if let Ok(lean_path) = toolchain::find_tool_binary(active_tc, "lean") {
                                if let Some(bin_dir) = lean_path.parent() {
                                    if let Some(tc_path) = bin_dir.parent() {
                                        if let (Ok(entry_canonical), Ok(tc_canonical)) =
                                            (entry.path().canonicalize(), tc_path.canonicalize())
                                        {
                                            entry_canonical == tc_canonical
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        }
                    } else {
                        false
                    };

                    if is_active {
                        status_parts.push("active");
                    }

                    // Check if default
                    let is_default = config
                        .default_toolchain
                        .as_ref()
                        .map(|d| d == &name)
                        .unwrap_or(false);

                    if is_default {
                        status_parts.push("default");
                    }

                    // Print with status
                    if status_parts.is_empty() {
                        println!("{}", name);
                    } else {
                        println!("{} ({})", name, status_parts.join(", "));
                    }
                }
            }
        }
    }

    if !has_toolchains {
        println!("{}", "no installed toolchains".dimmed());
    }

    println!();

    // Show active toolchain details
    println!("{}", "active toolchain".bold());
    println!("{}", "----------------".bold());

    if let Some((toolchain, src)) = &active_toolchain {
        println!("name: {}", toolchain);

        // Show reason for being active
        let reason = match src {
            toolchain::ToolchainSource::Explicit => "overridden by +toolchain on the command line",
            toolchain::ToolchainSource::Environment => {
                "overridden by environment variable LEMMA_TOOLCHAIN"
            }
            toolchain::ToolchainSource::Override(_) => "directory override for current directory",
            toolchain::ToolchainSource::ProjectFile(path) => {
                // Extract just the filename for cleaner display
                if let Some(file_name) = std::path::Path::new(path).file_name() {
                    if let Some(file_str) = file_name.to_str() {
                        if file_str == "lean-toolchain" {
                            "overridden by lean-toolchain file"
                        } else {
                            "overridden by project file"
                        }
                    } else {
                        "overridden by project file"
                    }
                } else {
                    "overridden by project file"
                }
            }
            toolchain::ToolchainSource::Default => "it's the default toolchain",
        };
        println!("active because: {}", reason);

        // Try to find the toolchain and show additional info
        match toolchain::find_tool_binary(toolchain, "lean") {
            Ok(lean_path) => {
                if let Some(bin_dir) = lean_path.parent() {
                    if let Some(toolchain_path) = bin_dir.parent() {
                        // Show lean version
                        if let Ok(version) = toolchain::get_lean_version(toolchain_path) {
                            if let Some(version_str) = extract_version_line(&version) {
                                println!("lean version: {}", version_str);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                println!("{}", "warning: toolchain is not installed".yellow().bold());
            }
        }
    } else {
        println!("{}", "no active toolchain".dimmed());
        println!();
        println!(
            "{}",
            "tip: set a default with 'lemma default <toolchain>'".dimmed()
        );
    }

    println!();

    Ok(())
}

/// Get the host platform triple
fn get_host_triple() -> &'static str {
    // Common platform triples
    #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "gnu"))]
    return "x86_64-unknown-linux-gnu";

    #[cfg(all(target_arch = "x86_64", target_os = "linux", target_env = "musl"))]
    return "x86_64-unknown-linux-musl";

    #[cfg(all(target_arch = "aarch64", target_os = "linux", target_env = "gnu"))]
    return "aarch64-unknown-linux-gnu";

    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    return "x86_64-apple-darwin";

    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    return "aarch64-apple-darwin";

    #[cfg(all(target_arch = "x86_64", target_os = "windows", target_env = "msvc"))]
    return "x86_64-pc-windows-msvc";

    #[cfg(all(target_arch = "x86_64", target_os = "windows", target_env = "gnu"))]
    return "x86_64-pc-windows-gnu";

    // Fallback
    #[cfg(not(any(
        all(target_arch = "x86_64", target_os = "linux"),
        all(target_arch = "aarch64", target_os = "linux"),
        all(target_arch = "x86_64", target_os = "macos"),
        all(target_arch = "aarch64", target_os = "macos"),
        all(target_arch = "x86_64", target_os = "windows"),
    )))]
    return "unknown";
}

/// Extract the first line of lean --version output
fn extract_version_line(output: &str) -> Option<String> {
    output.lines().next().map(|s| s.trim().to_string())
}
