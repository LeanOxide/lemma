//! Build command - Build the current Lean project using Lake

use anyhow::{Context, Result};
use lemma_config::{Config, GlobalSettings};
use lemma_output::Printer;
use lemma_static::EnvVars;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Execute the build command
pub fn execute(
    toolchain: Option<&str>,
    path: Option<&str>,
    args: &[String],
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    // Determine the project directory
    let project_dir = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        env::current_dir().context("Failed to get current directory")?
    };

    // Validate that this is a Lean project by checking for lakefile
    validate_lean_project(&project_dir)?;

    // Determine which toolchain to use
    let toolchain_name = if let Some(tc) = toolchain {
        // Explicit toolchain specified
        printer.hint(format!("Using specified toolchain: {}", tc))?;
        tc.to_string()
    } else {
        // Resolve toolchain from environment/overrides/project/default
        match resolve_toolchain(&project_dir, settings, printer)? {
            Some(tc) => {
                printer.hint(format!("Using active toolchain: {}", tc))?;
                tc
            }
            None => {
                anyhow::bail!(
                    "No toolchain found. Install one with `lemma lean install stable` \
                     or specify with --toolchain"
                );
            }
        }
    };

    // Find the lake binary in the toolchain
    let lake_binary = lemma_config::find_tool_binary(&toolchain_name, "lake")
        .with_context(|| {
            format!(
                "Failed to find 'lake' in toolchain '{}'. \
                 Ensure the toolchain is properly installed.",
                toolchain_name
            )
        })?;

    // Build the lake command
    let mut cmd = Command::new(&lake_binary);
    cmd.arg("build");

    // Add any additional arguments (targets, flags, etc.)
    if !args.is_empty() {
        cmd.args(args);
    }

    // Set the working directory to the project directory
    cmd.current_dir(&project_dir);

    // Set environment variables
    cmd.env(EnvVars::LEMMA_TOOLCHAIN, &toolchain_name);
    if let Ok(lemma_home) = Config::lemma_home() {
        cmd.env(EnvVars::LEMMA_HOME, lemma_home);
    }

    // Prepend the toolchain's bin directory to PATH
    // This ensures lake can find lean and other tools from the same toolchain
    if let Some(bin_dir) = lake_binary.parent() {
        if let Some(current_path) = env::var_os("PATH") {
            let mut paths = vec![bin_dir.to_path_buf()];
            paths.extend(env::split_paths(&current_path));

            if let Ok(new_path) = env::join_paths(paths) {
                cmd.env("PATH", new_path);
            }
        }
    }

    // Show what we're doing
    printer.status("Building Lean project...")?;

    // Run the command and wait for it to complete
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute lake from toolchain '{}'", toolchain_name))?;

    // Exit with the same code as lake
    if status.success() {
        printer.success("Build completed successfully")?;
        Ok(())
    } else {
        std::process::exit(status.code().unwrap_or(1));
    }
}

/// Validate that the given directory is a Lean project
fn validate_lean_project(dir: &Path) -> Result<()> {
    // Check for lakefile.lean or lakefile.toml
    let has_lakefile_lean = dir.join("lakefile.lean").exists();
    let has_lakefile_toml = dir.join("lakefile.toml").exists();

    if !has_lakefile_lean && !has_lakefile_toml {
        anyhow::bail!(
            "No lakefile found in '{}'. \
             Make sure you're in a Lean project directory or use --path to specify the project location.",
            dir.display()
        );
    }

    Ok(())
}

/// Resolve the active toolchain for the given directory
fn resolve_toolchain(
    dir: &Path,
    _settings: &GlobalSettings,
    _printer: &Printer,
) -> Result<Option<String>> {
    // Priority order:
    // 1. LEMMA_TOOLCHAIN environment variable
    // 2. Directory override
    // 3. lean-toolchain file in project (or parent directories)
    // 4. Default toolchain

    // 1. Check environment variable
    if let Ok(tc) = env::var(EnvVars::LEMMA_TOOLCHAIN) {
        if !tc.is_empty() {
            return Ok(Some(tc));
        }
    }

    // 2. Check for directory override
    if let Ok(config) = Config::load() {
        if let Some((_path, tc)) = config.find_override(dir) {
            return Ok(Some(tc));
        }
    }

    // 3. Check for lean-toolchain file
    if let Some(tc) = find_toolchain_file(dir)? {
        return Ok(Some(tc));
    }

    // 4. Fall back to default toolchain
    if let Ok(config) = Config::load() {
        if let Some(default) = config.default_toolchain {
            return Ok(Some(default));
        }
    }

    Ok(None)
}

/// Find and read lean-toolchain file in the directory or its parents
fn find_toolchain_file(mut dir: &Path) -> Result<Option<String>> {
    loop {
        let toolchain_file = dir.join("lean-toolchain");
        if toolchain_file.exists() {
            let content = std::fs::read_to_string(&toolchain_file)
                .with_context(|| format!("Failed to read {}", toolchain_file.display()))?;

            // Parse the toolchain name (trim whitespace and remove "leanprover/lean4:" prefix if present)
            let toolchain = content.trim();
            let toolchain = toolchain
                .strip_prefix("leanprover/lean4:")
                .unwrap_or(toolchain);

            if !toolchain.is_empty() {
                return Ok(Some(toolchain.to_string()));
            }
        }

        // Move up to parent directory
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }

    Ok(None)
}
