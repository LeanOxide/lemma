//! Show command - Display active toolchain information

use anyhow::{Context, Result};
use colored::Colorize;
use std::env;
use std::fs;
use std::path::Path;

use crate::config::Config;

/// Source of the active toolchain
#[derive(Debug)]
enum ToolchainSource {
    Environment,
    Override(String),
    ProjectFile(String),
    Default,
}

impl std::fmt::Display for ToolchainSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Environment => write!(f, "environment variable LEMMA_TOOLCHAIN"),
            Self::Override(path) => write!(f, "directory override at {}", path),
            Self::ProjectFile(path) => write!(f, "project file at {}", path),
            Self::Default => write!(f, "default setting"),
        }
    }
}

pub fn execute() -> Result<()> {
    let config = Config::load().context("Failed to load configuration")?;

    // Determine the active toolchain
    let active_toolchain = resolve_active_toolchain()?;

    // Display active toolchain
    if let Some((toolchain, src)) = &active_toolchain {
        println!("{}", "Active toolchain".green().bold());
        println!("----------------");
        println!();
        println!("  {}", toolchain);
        println!("  ({})", src.to_string().dimmed());
        println!();

        // Show toolchain path if it exists
        let toolchains_dir = Config::toolchains_dir()?;
        let toolchain_path = toolchains_dir.join(&toolchain);
        if toolchain_path.exists() {
            println!("  Path: {}", toolchain_path.display());

            // Try to get lean version
            if let Ok(version) = get_lean_version(&toolchain_path) {
                println!("  Lean: {}", version.trim());
            }
        } else {
            println!("  {} Toolchain not installed", "Warning:".yellow().bold());
        }
        println!();
    } else {
        println!("{} No active toolchain found", "Warning:".yellow().bold());
        println!();
        println!("Set a default with: lemma default <toolchain>");
        println!();
    }

    // Show default toolchain
    if let Some(ref default) = config.default_toolchain {
        println!("{}", "Default toolchain".green().bold());
        println!("-----------------");
        println!("  {}", default);
        println!();
    }

    // List installed toolchains
    println!("{}", "Installed toolchains".green().bold());
    println!("--------------------");

    let toolchains_dir = Config::toolchains_dir()?;
    if toolchains_dir.exists() {
        let mut entries: Vec<_> = fs::read_dir(&toolchains_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();

        entries.sort_by_key(|e| e.file_name());

        if entries.is_empty() {
            println!("  {}", "No toolchains installed".dimmed());
        } else {
            for entry in entries {
                if let Some(name) = entry.file_name().to_str() {
                    // Check if this is the active toolchain
                    let is_active = active_toolchain
                        .as_ref()
                        .map(|(tc, _)| tc == name)
                        .unwrap_or(false);

                    if is_active {
                        println!("  {} {}", "•".cyan(), name);
                    } else {
                        println!("  {} {}", " ", name);
                    }
                }
            }
        }
    } else {
        println!("  {}", "No toolchains installed".dimmed());
    }

    println!();

    Ok(())
}

/// Resolve the active toolchain based on priority
fn resolve_active_toolchain() -> Result<Option<(String, ToolchainSource)>> {
    // 1. Environment variable override
    if let Ok(toolchain) = env::var("LEMMA_TOOLCHAIN") {
        return Ok(Some((toolchain, ToolchainSource::Environment)));
    }

    // Load config once for both override and default checks
    let config = Config::load().context("Failed to load configuration")?;

    // 2. Directory override (walks up from current directory)
    if let Ok(current_dir) = env::current_dir() {
        if let Some((path, toolchain)) = config.find_override(&current_dir) {
            return Ok(Some((toolchain, ToolchainSource::Override(path))));
        }
    }

    // 3. Project-specific configuration files
    if let Ok(current_dir) = env::current_dir() {
        if let Some((toolchain, path)) = find_project_toolchain(&current_dir)? {
            return Ok(Some((toolchain, ToolchainSource::ProjectFile(path))));
        }
    }

    // 4. Default toolchain from config
    if let Some(default) = config.default_toolchain {
        return Ok(Some((default, ToolchainSource::Default)));
    }

    Ok(None)
}

/// Find project-specific toolchain configuration by walking up the directory tree
fn find_project_toolchain(start_dir: &Path) -> Result<Option<(String, String)>> {
    let mut current = start_dir;

    loop {
        // Check for lean-toolchain file
        let toolchain_file = current.join("lean-toolchain");
        if toolchain_file.exists() {
            if let Ok(contents) = fs::read_to_string(&toolchain_file) {
                let toolchain = contents.trim();
                if !toolchain.is_empty() {
                    return Ok(Some((
                        toolchain.to_string(),
                        toolchain_file.display().to_string(),
                    )));
                }
            }
        }

        // Check for leanpkg.toml with lean_version
        let leanpkg_file = current.join("leanpkg.toml");
        if leanpkg_file.exists() {
            if let Ok(contents) = fs::read_to_string(&leanpkg_file) {
                // Simple parsing for lean_version = "..."
                for line in contents.lines() {
                    let line = line.trim();
                    if line.starts_with("lean_version") {
                        if let Some(version) = extract_toml_string_value(line) {
                            return Ok(Some((version, leanpkg_file.display().to_string())));
                        }
                    }
                }
            }
        }

        // Move up to parent directory
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }

    Ok(None)
}

/// Extract a string value from a TOML line like: key = "value"
fn extract_toml_string_value(line: &str) -> Option<String> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }

    let value = parts[1].trim();

    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        Some(value[1..value.len() - 1].to_string())
    } else if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        Some(value[1..value.len() - 1].to_string())
    } else {
        None
    }
}

/// Get the Lean version from a toolchain
fn get_lean_version(toolchain_path: &Path) -> Result<String> {
    let lean_bin = toolchain_path.join("bin").join("lean");

    if !lean_bin.exists() {
        anyhow::bail!("lean binary not found");
    }

    let output = std::process::Command::new(&lean_bin)
        .arg("--version")
        .output()
        .context("Failed to execute lean --version")?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        anyhow::bail!("lean --version failed")
    }
}
