//! Build command - Build the current Lean project

use anyhow::{Context, Result};
use lemma_config::GlobalSettings;
use lemma_output::Printer;
use std::path::{Path, PathBuf};

/// Execute the build command
pub fn execute(
    path: Option<&str>,
    clear: bool,
    out_dir: Option<&str>,
    targets: &[String],
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    if settings.is_verbose() {
        printer.hint("Using lemma build system")?;
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

        // Use the new build_targets method if targets are specified
        let result = if targets.is_empty() {
            context.build().await
        } else {
            if settings.is_verbose() {
                printer.hint(format!("Building targets: {}", targets.join(", ")))?;
            }
            context.build_targets(targets).await
        };

        match result {
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
