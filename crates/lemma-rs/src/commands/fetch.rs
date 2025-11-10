//! Fetch command - download sparse dependency caches
//!
//! This command implements sparse cache downloading for large dependencies like mathlib4.
//! Instead of downloading the entire cache, it analyzes project imports and downloads
//! only the required modules and their transitive dependencies.

use anyhow::{Context, Result};
use colored::Colorize;
use std::path::PathBuf;

/// Execute the fetch command
pub fn execute(
    package: &str,
    modules: Vec<String>,
    auto: bool,
    dry_run: bool,
    path: Option<String>,
) -> Result<()> {
    let project_path = match path {
        Some(p) => PathBuf::from(p),
        None => std::env::current_dir().context("Failed to get current directory")?,
    };

    println!("{} Fetching cache for {}", "=>".green().bold(), package);

    match package {
        "mathlib4" | "mathlib" => fetch_mathlib(project_path, modules, auto, dry_run),
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
) -> Result<()> {
    use crate::sparse_cache::mathlib::MathlibCacheFetcher;

    let fetcher = MathlibCacheFetcher::new()?;

    if auto {
        println!(
            "   {} Auto-detecting modules from project imports...",
            "•".cyan()
        );
        fetcher.fetch_auto(&project_path, dry_run)?;
    } else if !modules.is_empty() {
        println!(
            "   {} Fetching specified modules: {:?}",
            "•".cyan(),
            modules
        );
        fetcher.fetch_modules(&project_path, &modules, dry_run)?;
    } else {
        println!(
            "   {} Fetching full cache (no --module or --auto specified)",
            "•".cyan()
        );
        fetcher.fetch_full(&project_path, dry_run)?;
    }

    if dry_run {
        println!(
            "\n{} Dry run complete. No files were downloaded.",
            "✓".green().bold()
        );
    } else {
        println!("\n{} Cache fetch complete!", "✓".green().bold());
    }

    Ok(())
}
