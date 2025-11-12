//! Build command - Build the current Lean project

use anyhow::{Context, Result};
use lemma_config::{Config, GlobalSettings};
use lemma_output::Printer;
use std::path::{Path, PathBuf};

/// Execute the build command
pub fn execute(
    path: Option<&str>,
    native: bool,
    clear: bool,
    out_dir: Option<&str>,
    targets: &[String],
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    if native {
        execute_native_build(path, clear, out_dir, targets, settings, printer)
    } else {
        execute_lake_wrapper(path, targets, settings, printer)
    }
}

/// Execute build using the native lemma build system
fn execute_native_build(
    path: Option<&str>,
    clear: bool,
    out_dir: Option<&str>,
    _targets: &[String],
    _settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    if _settings.is_verbose() {
        printer.hint("Using native lemma build system (experimental)")?;
    }

    // Determine the project directory
    let project_dir = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        std::env::current_dir().context("Failed to get current directory")?
    };

    // Validate that this is a Lean project
    validate_lean_project(&project_dir)?;

    // Determine build directory
    let build_dir = if let Some(out) = out_dir {
        PathBuf::from(out)
    } else {
        project_dir.join(".lake").join("build")
    };

    // Clear build directory if requested
    if clear {
        if build_dir.exists() {
            std::fs::remove_dir_all(&build_dir).context("Failed to clear build directory")?;
        }
    }

    // Create a Tokio runtime to run the async build
    let runtime = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;

    runtime.block_on(async {
        let mut context = lemma_build::BuildContext::from_directory(&project_dir)
            .context("Failed to create build context")?;

        // Override build directory if custom out_dir was specified
        if let Some(out) = out_dir {
            context.lakefile.build_dir = PathBuf::from(out);
        }

        match context.build().await {
            Ok(()) => Ok(()),
            Err(e) => {
                // Check if this is a "not yet implemented" error for linking
                let err_msg = e.to_string();
                if err_msg.contains("Linking not yet implemented") {
                    printer.warning(
                        "Compilation succeeded, but linking is not yet implemented (Phase 6)",
                    )?;
                    printer.hint("All .olean files have been generated successfully.")?;
                    printer.hint("Use `lemma build` (without --native) to link with Lake.")?;
                    Ok(())
                } else {
                    printer.error(format!("Build failed: {}", e))?;
                    Err(anyhow::anyhow!(e))
                }
            }
        }
    })
}

/// Execute build by wrapping lake (default mode)
fn execute_lake_wrapper(
    path: Option<&str>,
    targets: &[String],
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    use lemma_static::EnvVars;
    use std::env;
    use std::process::Command;

    // Determine the project directory
    let project_dir = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        env::current_dir().context("Failed to get current directory")?
    };

    // Validate that this is a Lean project by checking for lakefile
    validate_lean_project(&project_dir)?;

    // Determine which toolchain to use
    let toolchain_name = match resolve_toolchain(&project_dir, settings, printer)? {
        Some(tc) => {
            printer.hint(format!("Using toolchain: {}", tc))?;
            tc
        }
        None => {
            anyhow::bail!("No toolchain found. Install one with `lemma lean install stable`");
        }
    };

    // Find the lake binary in the toolchain
    let lake_binary =
        lemma_config::find_tool_binary(&toolchain_name, "lake").with_context(|| {
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
    if !targets.is_empty() {
        cmd.args(targets);
    }

    // Set the working directory to the project directory
    cmd.current_dir(&project_dir);

    // Set environment variables
    cmd.env(EnvVars::LEMMA_TOOLCHAIN, &toolchain_name);
    if let Ok(lemma_home) = Config::lemma_home() {
        cmd.env(EnvVars::LEMMA_HOME, lemma_home);
    }

    // Prepend the toolchain's bin directory to PATH
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
    printer.status("Building Lean project with Lake...")?;

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
    use lemma_static::EnvVars;
    use std::env;

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
