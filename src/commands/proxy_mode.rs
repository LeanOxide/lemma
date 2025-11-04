//! Proxy mode - Execute tools from the active toolchain
//!
//! When invoked as a tool name (lean, lake, etc.), lemma acts as a proxy
//! to the actual tool in the active toolchain.

use anyhow::{Context, Result};
use std::env;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;

/// List of tools that lemma proxies for Lean
pub static PROXY_TOOLS: &[&str] = &[
    "lean",
    "lake",
    "leanpkg",
    "leanchecker",
    "leanc",
    "leanmake",
];

/// Execute a tool from the active toolchain
pub fn execute(tool_name: &str) -> Result<()> {
    // Get command-line args, skipping argv[0]
    let args: Vec<String> = env::args().skip(1).collect();

    // Check for explicit toolchain override (e.g., lean +nightly test.lean)
    let explicit_toolchain = args
        .first()
        .filter(|arg| arg.starts_with('+'))
        .map(|arg| arg[1..].to_string());

    let tool_args: Vec<String> = if explicit_toolchain.is_some() {
        args.into_iter().skip(1).collect()
    } else {
        args
    };

    // Resolve which toolchain to use
    let toolchain_name = resolve_toolchain(explicit_toolchain.as_deref())?;

    // Get the path to the actual tool binary
    let tool_path = find_tool_binary(&toolchain_name, tool_name)?;

    // Execute the tool, replacing the current process (Unix exec)
    // This ensures the tool runs with the correct PID and signal handling
    let mut cmd = Command::new(&tool_path);
    cmd.args(&tool_args);

    // Set environment variables to communicate toolchain info
    cmd.env("LEMMA_TOOLCHAIN", &toolchain_name);
    if let Ok(lemma_home) = Config::lemma_home() {
        cmd.env("LEMMA_HOME", lemma_home);
    }

    // Prepend ~/.lemma/bin to PATH for recursive tool calls
    if let Ok(lemma_home) = Config::lemma_home() {
        let lemma_bin = lemma_home.join("bin");
        if let Ok(current_path) = env::var("PATH") {
            let new_path = format!("{}:{}", lemma_bin.display(), current_path);
            cmd.env("PATH", new_path);
        }
    }

    // Use Unix exec to replace current process with the tool
    // This will not return if successful
    Err(cmd.exec().into())
}

/// Resolve which toolchain to use based on priority:
/// 1. Explicit toolchain from command line (+toolchain)
/// 2. LEMMA_TOOLCHAIN environment variable
/// 3. Directory override (current dir or parents)
/// 4. lean-toolchain file (current dir or parents)
/// 5. leanpkg.toml lean_version field (current dir or parents)
/// 6. Default toolchain from config
fn resolve_toolchain(explicit: Option<&str>) -> Result<String> {
    // 1. Explicit command-line override
    if let Some(toolchain) = explicit {
        return Ok(toolchain.to_string());
    }

    // 2. Environment variable override
    if let Ok(toolchain) = env::var("LEMMA_TOOLCHAIN") {
        return Ok(toolchain);
    }

    // Load config once for both override and default checks
    let config = Config::load().context("Failed to load configuration")?;

    // 3. Directory override (walks up from current directory)
    if let Ok(current_dir) = env::current_dir() {
        if let Some((_, toolchain)) = config.find_override(&current_dir) {
            return Ok(toolchain);
        }
    }

    // 4. Project-specific configuration files
    if let Ok(current_dir) = env::current_dir() {
        if let Some(toolchain) = find_project_toolchain(&current_dir)? {
            return Ok(toolchain);
        }
    }

    // 5. Default toolchain from config
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

    #[test]
    fn test_proxy_tools_list() {
        assert!(PROXY_TOOLS.contains(&"lean"));
        assert!(PROXY_TOOLS.contains(&"lake"));
        assert!(PROXY_TOOLS.contains(&"leanpkg"));
    }
}
