//! Run command - Execute a command with a specific toolchain

use anyhow::{Context, Result};
use lemma_static::EnvVars;
use std::env;
use std::process::Command;

use lemma_config::Config;
use lemma_config::GlobalSettings;
use lemma_output::Printer;

/// Execute a command with a specific toolchain
pub fn execute(
    toolchain: &str,
    command_args: &[String],
    _settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    printer.hint(format!("Running command with toolchain: {}", toolchain))?;
    printer.hint(format!("Command: {:?}", command_args))?;
    // Check if command is empty
    if command_args.is_empty() {
        anyhow::bail!("No command specified.\n\nUsage: lemma run <toolchain> <command> [args...]");
    }

    // Get the command name
    let command_name = &command_args[0];
    let args = &command_args[1..];

    // Check if this is a known Lean tool (lean, lake, etc.)
    let is_lean_tool = crate::commands::proxy_mode::PROXY_TOOLS
        .iter()
        .any(|&tool| tool == command_name);

    let actual_command = if is_lean_tool {
        // If it's a Lean tool, find it in the specified toolchain
        lemma_config::find_tool_binary(toolchain, command_name)?
    } else {
        // If it's not a Lean tool, just use the command as-is from PATH
        // But we'll still set up the environment with the toolchain
        std::path::PathBuf::from(command_name)
    };

    // Set up the command
    let mut cmd = Command::new(&actual_command);
    cmd.args(args);

    // Set environment variables
    cmd.env(EnvVars::LEMMA_TOOLCHAIN, toolchain);
    if let Ok(lemma_home) = Config::lemma_home() {
        cmd.env(EnvVars::LEMMA_HOME, lemma_home);
    }

    // Prepend the toolchain's bin directory to PATH
    // This ensures that when the command calls other tools (e.g., lake calling lean),
    // they use the same toolchain
    if is_lean_tool {
        if let Some(bin_dir) = actual_command.parent() {
            if let Some(current_path) = env::var_os("PATH") {
                let mut paths = vec![bin_dir.to_path_buf()];
                paths.extend(env::split_paths(&current_path));

                if let Ok(new_path) = env::join_paths(paths) {
                    cmd.env("PATH", new_path);
                }
            }
        }
    }

    // Run the command and wait for it to complete
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute command: {}", command_name))?;

    // Exit with the same code as the command
    if status.success() {
        Ok(())
    } else {
        std::process::exit(status.code().unwrap_or(1));
    }
}
