//! Which command - Display the path to a binary in the active toolchain

use anyhow::Result;
use std::path::PathBuf;

use crate::config::Config;
use crate::toolchain;

pub fn execute(binary: &str) -> Result<()> {
    // Resolve which toolchain to use
    let toolchain_name = toolchain::resolve_toolchain_or_fail(None)?;

    // Find the binary path
    let binary_path = find_tool_binary(&toolchain_name, binary)?;

    // Print the path
    println!("{}", binary_path.display());

    Ok(())
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
