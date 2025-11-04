//! Which command - Display the path to a binary in the active toolchain

use anyhow::{Context, Result};
use colored::Colorize;
use std::env;
use std::path::{Path, PathBuf};

use crate::config::Config;

pub fn execute(binary: &str) -> Result<()> {
    // Resolve which toolchain to use
    let toolchain_name = resolve_toolchain()?;

    // Find the binary path
    let binary_path = find_tool_binary(&toolchain_name, binary)?;

    // Print the path
    println!("{}", binary_path.display());

    Ok(())
}

/// Resolve which toolchain to use based on priority:
/// 1. LEMMA_TOOLCHAIN environment variable
/// 2. Directory override (current dir or parents)
/// 3. lean-toolchain file (current dir or parents)
/// 4. leanpkg.toml lean_version field (current dir or parents)
/// 5. Default toolchain from config
fn resolve_toolchain() -> Result<String> {
    // 1. Environment variable override
    if let Ok(toolchain) = env::var("LEMMA_TOOLCHAIN") {
        return Ok(toolchain);
    }

    // Load config once for both override and default checks
    let config = Config::load().context("Failed to load configuration")?;

    // 2. Directory override (walks up from current directory)
    if let Ok(current_dir) = env::current_dir() {
        if let Some((_, toolchain)) = config.find_override(&current_dir) {
            return Ok(toolchain);
        }
    }

    // 3. Project-specific configuration files
    if let Ok(current_dir) = env::current_dir() {
        if let Some(toolchain) = find_project_toolchain(&current_dir)? {
            return Ok(toolchain);
        }
    }

    // 4. Default toolchain from config
    if let Some(default) = config.default_toolchain {
        return Ok(default);
    }

    anyhow::bail!(
        "No active toolchain found.\n\n\
         Set a default with: lemma default <toolchain>\n\
         Or install a toolchain with: lemma toolchain install stable"
    )
}

/// Find project-specific toolchain configuration by walking up the directory tree
fn find_project_toolchain(start_dir: &Path) -> Result<Option<String>> {
    let mut current = start_dir;

    loop {
        // Check for lean-toolchain file
        let toolchain_file = current.join("lean-toolchain");
        if toolchain_file.exists() {
            if let Ok(contents) = std::fs::read_to_string(&toolchain_file) {
                let toolchain = contents.trim();
                if !toolchain.is_empty() {
                    return Ok(Some(toolchain.to_string()));
                }
            }
        }

        // Check for leanpkg.toml with lean_version
        let leanpkg_file = current.join("leanpkg.toml");
        if leanpkg_file.exists() {
            if let Ok(contents) = std::fs::read_to_string(&leanpkg_file) {
                // Simple parsing for lean_version = "..."
                for line in contents.lines() {
                    let line = line.trim();
                    if line.starts_with("lean_version") {
                        if let Some(version) = extract_toml_string_value(line) {
                            return Ok(Some(version));
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
    // Find the = sign
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }

    // Get the value part and trim whitespace
    let value = parts[1].trim();

    // Remove surrounding quotes
    if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        Some(value[1..value.len() - 1].to_string())
    } else if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        Some(value[1..value.len() - 1].to_string())
    } else {
        None
    }
}

/// Find the path to a tool binary in the specified toolchain
fn find_tool_binary(toolchain: &str, tool_name: &str) -> Result<PathBuf> {
    let toolchains_dir = Config::toolchains_dir()?;
    let toolchain_path = toolchains_dir.join(toolchain);

    // Check if toolchain exists
    if !toolchain_path.exists() {
        anyhow::bail!(
            "Toolchain '{}' is not installed.\n\n\
             Install it with: lemma toolchain install {}",
            toolchain,
            toolchain
        );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_toml_string_value() {
        assert_eq!(
            extract_toml_string_value(r#"lean_version = "v4.5.0""#),
            Some("v4.5.0".to_string())
        );
        assert_eq!(
            extract_toml_string_value(r#"  lean_version  =  "stable"  "#),
            Some("stable".to_string())
        );
        assert_eq!(
            extract_toml_string_value("lean_version = 'nightly'"),
            Some("nightly".to_string())
        );
        assert_eq!(extract_toml_string_value("invalid line"), None);
    }
}
