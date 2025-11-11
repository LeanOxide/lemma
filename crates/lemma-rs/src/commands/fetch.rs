//! Fetch command - download sparse dependency caches
//!
//! This command implements sparse cache downloading for large dependencies like mathlib4.
//! Instead of downloading the entire cache, it analyzes project imports and downloads
//! only the required modules and their transitive dependencies.

use anyhow::{Context, Result};
use std::path::PathBuf;

use lemma_config::GlobalSettings;
use lemma_output::Printer;

/// Execute the fetch command
pub fn execute(
    package: &str,
    modules: Vec<String>,
    auto: bool,
    dry_run: bool,
    path: Option<String>,
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    let project_path = match path {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().context("Failed to get current directory")?,
    };

    printer.status(format!("Fetching cache for {}", package))?;

    match package {
        "mathlib4" | "mathlib" => {
            fetch_mathlib(project_path, modules, auto, dry_run, settings, printer)
        }
        _ => {
            anyhow::bail!(
                "Unknown package: {}. Currently only 'mathlib4' is supported.",
                package
            )
        }
    }
}

/// Fetch mathlib4 cache
fn fetch_mathlib(
    project_path: PathBuf,
    modules: Vec<String>,
    auto: bool,
    dry_run: bool,
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    let _ = settings;
    use crate::sparse_cache::mathlib::MathlibCacheFetcher;

    let fetcher = MathlibCacheFetcher::new()?;

    if auto {
        printer.list_item("Auto-detecting modules from project imports")?;
        fetcher.fetch_auto(&project_path, dry_run)?;
    } else if !modules.is_empty() {
        printer.list_item(format!(
            "Fetching specified modules: {}",
            modules.join(", ")
        ))?;
        fetcher.fetch_modules(&project_path, &modules, dry_run)?;
    } else {
        printer.list_item("Fetching full cache")?;
        fetcher.fetch_full(&project_path, dry_run)?;
    }

    if dry_run {
        printer.success("Dry run complete. No files were downloaded.")?;
    } else {
        printer.success("Cache fetch complete!")?;
    }

    Ok(())
}
