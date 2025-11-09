//! Mathlib-specific sparse cache fetching
//!
//! This module implements the logic for fetching sparse mathlib caches,
//! including auto-detection of imports, transitive dependency resolution,
//! and parallel downloading.

use anyhow::{Context, Result};
use colored::Colorize;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::dependency_analyzer::{analyze_project_imports, compute_closure};
use super::r2_client::{detect_platform, DependencyIndex, R2Client};

/// Mathlib cache fetcher
pub struct MathlibCacheFetcher {
    r2_client: R2Client,
}

impl MathlibCacheFetcher {
    /// Create a new mathlib cache fetcher
    pub fn new() -> Result<Self> {
        Ok(Self {
            r2_client: R2Client::new()?,
        })
    }

    /// Fetch cache with auto-detected modules from project imports
    pub fn fetch_auto(&self, project_path: &Path, dry_run: bool) -> Result<()> {
        // 1. Find project imports
        println!("   {} Scanning project files...", "•".cyan());
        let imports = analyze_project_imports(project_path)?;

        if imports.is_empty() {
            println!("      {} No mathlib imports found in project", "!".yellow());
            return Ok(());
        }

        println!("      Found {} direct imports", imports.len());

        // 2. Get mathlib commit from lake-manifest.json
        let (commit, lean_version) = self.get_mathlib_info(project_path)?;
        println!("   {} Mathlib commit: {}", "•".cyan(), commit);
        println!("   {} Lean version: {}", "•".cyan(), lean_version);

        // 3. Fetch dependency index
        println!("   {} Fetching dependency index...", "•".cyan());
        let platform = detect_platform();
        let index = self.r2_client.fetch_index(&commit, &platform)?;

        println!("      Index contains {} modules", index.modules.len());

        // 4. Compute transitive closure
        println!("   {} Computing transitive dependencies...", "•".cyan());
        let dependency_graph = Self::build_dependency_graph(&index);
        let closure = compute_closure(&imports, &dependency_graph);

        let total_modules = index.modules.len();
        let needed_modules = closure.len();
        let percentage = (needed_modules as f64 / total_modules as f64) * 100.0;

        println!(
            "      {} modules needed ({:.1}% of total {})",
            needed_modules, percentage, total_modules
        );

        // 5. Calculate total size
        let total_size: u64 = closure
            .iter()
            .filter_map(|m| index.modules.get(m))
            .map(|info| info.size)
            .sum();

        println!("      Total download size: {}", format_bytes(total_size));

        if dry_run {
            println!("\n   {} Modules to download:", "•".cyan());
            let mut modules: Vec<_> = closure.iter().collect();
            modules.sort();
            for module in modules.iter().take(10) {
                println!("      - {}", module);
            }
            if modules.len() > 10 {
                println!("      ... and {} more", modules.len() - 10);
            }
            return Ok(());
        }

        // 6. Download modules in parallel
        println!(
            "   {} Downloading {} modules...",
            "•".cyan(),
            needed_modules
        );
        self.download_modules(project_path, &closure, &index, &commit, &platform)?;

        println!("   {} Download complete!", "✓".green());

        Ok(())
    }

    /// Fetch specific modules
    pub fn fetch_modules(
        &self,
        project_path: &Path,
        modules: &[String],
        dry_run: bool,
    ) -> Result<()> {
        let (commit, _lean_version) = self.get_mathlib_info(project_path)?;
        let platform = detect_platform();

        println!("   {} Fetching dependency index...", "•".cyan());
        let index = self.r2_client.fetch_index(&commit, &platform)?;

        // Compute closure for specified modules
        let roots: HashSet<String> = modules.iter().cloned().collect();
        let dependency_graph = Self::build_dependency_graph(&index);
        let closure = compute_closure(&roots, &dependency_graph);

        println!(
            "      {} modules needed (including dependencies)",
            closure.len()
        );

        if dry_run {
            return Ok(());
        }

        self.download_modules(project_path, &closure, &index, &commit, &platform)?;

        Ok(())
    }

    /// Fetch full cache (fallback to traditional Lake behavior)
    pub fn fetch_full(&self, project_path: &Path, dry_run: bool) -> Result<()> {
        let (commit, _lean_version) = self.get_mathlib_info(project_path)?;
        let platform = detect_platform();

        println!("   {} Downloading full cache archive...", "•".cyan());

        if dry_run {
            println!("      Would download full cache for commit {}", commit);
            return Ok(());
        }

        let cache_dir = self.get_cache_dir(project_path)?;
        let archive_path = cache_dir.join("cache.tar.zst");

        self.r2_client
            .download_full_archive(&commit, &platform, &archive_path)?;

        // Extract archive
        println!("   {} Extracting archive...", "•".cyan());
        let file = std::fs::File::open(&archive_path)?;
        crate::archive::extract_tar_zst(file, &cache_dir)?;

        // Clean up archive
        fs::remove_file(&archive_path)?;

        Ok(())
    }

    /// Get mathlib commit and lean version from lake-manifest.json
    fn get_mathlib_info(&self, project_path: &Path) -> Result<(String, String)> {
        let manifest_path = project_path.join("lake-manifest.json");

        if !manifest_path.exists() {
            anyhow::bail!(
                "lake-manifest.json not found at {}. Run 'lake update' first.",
                manifest_path.display()
            );
        }

        let content =
            fs::read_to_string(&manifest_path).context("Failed to read lake-manifest.json")?;

        let manifest: serde_json::Value =
            serde_json::from_str(&content).context("Failed to parse lake-manifest.json")?;

        // Extract mathlib package info
        let packages = manifest
            .get("packages")
            .and_then(|p| p.as_array())
            .context("Invalid lake-manifest.json: missing 'packages'")?;

        let mathlib = packages
            .iter()
            .find(|pkg| {
                pkg.get("name")
                    .and_then(|n| n.as_str())
                    .map(|name| name == "mathlib" || name == "mathlib4")
                    .unwrap_or(false)
            })
            .context("Mathlib not found in lake-manifest.json")?;

        let commit = mathlib
            .get("rev")
            .and_then(|r| r.as_str())
            .context("Failed to get mathlib commit from lake-manifest.json")?
            .to_string();

        let lean_version = manifest
            .get("leanVersion")
            .or_else(|| manifest.get("version"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        Ok((commit, lean_version))
    }

    /// Get the cache directory for mathlib
    fn get_cache_dir(&self, project_path: &Path) -> Result<PathBuf> {
        let lake_dir = project_path.join(".lake");
        let cache_dir = lake_dir.join("build").join("lib");

        fs::create_dir_all(&cache_dir).with_context(|| {
            format!("Failed to create cache directory: {}", cache_dir.display())
        })?;

        Ok(cache_dir)
    }

    /// Build dependency graph from index
    fn build_dependency_graph(
        index: &DependencyIndex,
    ) -> std::collections::HashMap<String, Vec<String>> {
        index
            .modules
            .iter()
            .map(|(name, info)| (name.clone(), info.dependencies.clone()))
            .collect()
    }

    /// Download modules in parallel
    fn download_modules(
        &self,
        project_path: &Path,
        modules: &HashSet<String>,
        index: &DependencyIndex,
        commit: &str,
        platform: &str,
    ) -> Result<()> {
        let cache_dir = self.get_cache_dir(project_path)?;

        let modules_vec: Vec<_> = modules.iter().collect();

        // Download in parallel using rayon
        modules_vec
            .par_iter()
            .try_for_each(|module_name| -> Result<()> {
                let module_info = index.modules.get(*module_name).with_context(|| {
                    format!("Module {} not found in dependency index", module_name)
                })?;

                // Construct destination path
                let dest_path = cache_dir.join(&module_info.path);

                // Create parent directories
                if let Some(parent) = dest_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Download the file
                self.r2_client
                    .download_olean(commit, platform, &module_info.path, &dest_path)?;

                Ok(())
            })?;

        Ok(())
    }
}

/// Format bytes into human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
