//! Show command - Display active toolchain information

use anyhow::{Context, Result};
use colored::Colorize;
use std::io::Write;

use lemma_config::{Config, GlobalSettings, ToolchainRegistry};
use lemma_output::Printer;
use lemma_toolchain as toolchain;

pub fn execute(settings: &GlobalSettings, printer: &Printer) -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    // Show platform and lemma home
    let host_label = if printer.use_colors() {
        "Default host:".bold().to_string()
    } else {
        "Default host:".to_string()
    };
    let home_label = if printer.use_colors() {
        "lemma home:".bold().to_string()
    } else {
        "lemma home:".to_string()
    };

    writeln!(printer.stdout(), "{} {}", host_label, get_host_triple())?;
    writeln!(
        printer.stdout(),
        "{} {}",
        home_label,
        settings.lemma_home.display()
    )?;
    writeln!(printer.stdout())?;

    // Determine the active toolchain
    let active_toolchain = lemma_config::resolve_toolchain_with_source(None)?;

    // List installed toolchains first
    printer.header("installed toolchains")?;
    let separator = if printer.use_colors() {
        "--------------------".bold().to_string()
    } else {
        "--------------------".to_string()
    };
    writeln!(printer.stdout(), "{}", separator)?;

    // Create toolchain registry
    let registry = ToolchainRegistry::new(&Config::lemma_home()?);
    let installed_toolchains = registry.list_installed()?;
    let has_toolchains = !installed_toolchains.is_empty();

    if has_toolchains {
        for tc in &installed_toolchains {
            let name = &tc.name;

            // Build status indicators (active, default)
            let mut status_parts = Vec::new();

            // Check if active
            let is_active = if let Some((ref active_name, _)) = active_toolchain {
                if let Some(ref desc) = tc.desc {
                    registry.is_active(desc, active_name)
                } else {
                    active_name == name
                }
            } else {
                false
            };

            if is_active {
                status_parts.push("active");
            }

            // Check if default
            let is_default = if let Some(ref desc) = tc.desc {
                registry.is_default(desc, &config)
            } else {
                config.default_toolchain.as_ref() == Some(name)
            };

            if is_default {
                status_parts.push("default");
            }

            // Print with status
            if status_parts.is_empty() {
                writeln!(printer.stdout(), "{}", name)?;
            } else {
                writeln!(printer.stdout(), "{} ({})", name, status_parts.join(", "))?;
            }
        }
    }

    if !has_toolchains {
        let msg = if printer.use_colors() {
            "no installed toolchains".dimmed().to_string()
        } else {
            "no installed toolchains".to_string()
        };
        writeln!(printer.stdout(), "{}", msg)?;
    }

    writeln!(printer.stdout())?;

    // Show active toolchain details
    printer.header("active toolchain")?;
    let separator = if printer.use_colors() {
        "----------------".bold().to_string()
    } else {
        "----------------".to_string()
    };
    writeln!(printer.stdout(), "{}", separator)?;

    if let Some((toolchain, src)) = &active_toolchain {
        writeln!(printer.stdout(), "name: {}", toolchain)?;

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
        writeln!(printer.stdout(), "active because: {}", reason)?;

        // Try to find the toolchain and show additional info
        match lemma_config::find_tool_binary(toolchain, "lean") {
            Ok(lean_path) => {
                if let Some(bin_dir) = lean_path.parent() {
                    if let Some(toolchain_path) = bin_dir.parent() {
                        // Show lean version
                        if let Ok(version) = toolchain::get_lean_version(toolchain_path) {
                            if let Some(version_str) = extract_version_line(&version) {
                                writeln!(printer.stdout(), "lean version: {}", version_str)?;
                            }
                        }
                    }
                }
            }
            Err(_) => {
                printer.warning("toolchain is not installed")?;
            }
        }
    } else {
        let no_tc_msg = if printer.use_colors() {
            "no active toolchain".dimmed().to_string()
        } else {
            "no active toolchain".to_string()
        };
        writeln!(printer.stdout(), "{}", no_tc_msg)?;
        writeln!(printer.stdout())?;

        printer.hint("set a default with 'lemma default <toolchain>'")?;
    }

    writeln!(printer.stdout())?;

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
