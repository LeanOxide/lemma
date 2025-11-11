//! Proxy mode - Execute tools from the active toolchain
//!
//! When invoked as a tool name (lean, lake, etc.), lemma acts as a proxy
//! to the actual tool in the active toolchain.

use anyhow::Result;
use lemma_static::EnvVars;
use std::env;
use std::process::Command;

#[cfg(not(unix))]
use anyhow::Context;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use lemma_config::Config;

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
    let toolchain_name = lemma_config::resolve_toolchain_or_fail(explicit_toolchain.as_deref())?;

    // Get the path to the actual tool binary
    let tool_path = lemma_config::find_tool_binary(&toolchain_name, tool_name)?;

    // Execute the tool, replacing the current process (Unix exec)
    // This ensures the tool runs with the correct PID and signal handling
    let mut cmd = Command::new(&tool_path);
    cmd.args(&tool_args);

    // Set environment variables to communicate toolchain info
    cmd.env(EnvVars::LEMMA_TOOLCHAIN, &toolchain_name);
    if let Ok(lemma_home) = Config::lemma_home() {
        cmd.env(EnvVars::LEMMA_HOME, lemma_home);
    }

    // Prepend ~/.lemma/bin to PATH for recursive tool calls
    // This ensures that when tools call each other (e.g., lake calling lean),
    // they go through lemma's proxy and use the same toolchain.
    if let Ok(lemma_home) = Config::lemma_home() {
        let lemma_bin = lemma_home.join("bin");
        if let Some(current_path) = env::var_os("PATH") {
            let mut paths = vec![lemma_bin];
            paths.extend(env::split_paths(&current_path));

            if let Ok(new_path) = env::join_paths(paths) {
                cmd.env("PATH", new_path);
            }
        }
    }

    // On Unix: Use exec to replace current process with the tool
    // On Windows: Spawn and wait, then exit with the same code
    #[cfg(unix)]
    {
        // This will not return if successful
        Err(cmd.exec().into())
    }

    #[cfg(not(unix))]
    {
        let mut child = cmd.spawn().context("Failed to execute tool")?;

        let status = child
            .wait()
            .context("Failed to wait for tool to complete")?;

        std::process::exit(status.code().unwrap_or(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_tools_list() {
        assert!(PROXY_TOOLS.contains(&"lean"));
        assert!(PROXY_TOOLS.contains(&"lake"));
        assert!(PROXY_TOOLS.contains(&"leanpkg"));
    }
}
