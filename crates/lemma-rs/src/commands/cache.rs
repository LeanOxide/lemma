//! Cache command - Manage lemma's download cache and installed toolchains

use anyhow::Result;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use lemma_cli::cli::CacheCommands;
use lemma_config::{Config, GlobalSettings};
use lemma_output::Printer;

pub fn execute(command: CacheCommands, settings: &GlobalSettings, printer: &Printer) -> Result<()> {
    match command {
        CacheCommands::Dir => show_dir(settings, printer),
        CacheCommands::Stats => show_stats(settings, printer),
        CacheCommands::Clean { yes } => clean_cache(settings, printer, yes),
        CacheCommands::Prune { yes } => prune_all(settings, printer, yes),
    }
}

/// Show the cache directory path
fn show_dir(_settings: &GlobalSettings, printer: &Printer) -> Result<()> {
    let cache_dir = Config::tmp_dir()?;
    writeln!(printer.stdout(), "{}", cache_dir.display())?;
    Ok(())
}

/// Show cache statistics
fn show_stats(_settings: &GlobalSettings, printer: &Printer) -> Result<()> {
    let cache_dir = Config::tmp_dir()?;
    let toolchains_dir = Config::toolchains_dir()?;

    let cache_size = dir_size(&cache_dir)?;
    let toolchains_size = dir_size(&toolchains_dir)?;
    let total_size = cache_size + toolchains_size;

    printer.header("Cache Statistics")?;
    writeln!(
        printer.stdout(),
        "  Download cache:  {}",
        format_size(cache_size)
    )?;
    writeln!(
        printer.stdout(),
        "  Toolchains:      {}",
        format_size(toolchains_size)
    )?;
    writeln!(
        printer.stdout(),
        "  Total:           {}",
        format_size(total_size)
    )?;

    Ok(())
}

/// Clean the download cache
fn clean_cache(_settings: &GlobalSettings, printer: &Printer, yes: bool) -> Result<()> {
    let cache_dir = Config::tmp_dir()?;

    if !cache_dir.exists() {
        printer.status("Cache directory doesn't exist or is already clean")?;
        return Ok(());
    }

    let size = dir_size(&cache_dir)?;

    if size == 0 {
        printer.status("Cache is already empty")?;
        return Ok(());
    }

    if !yes {
        print!("Remove {} of cached downloads? [y/N]: ", format_size(size));
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            printer.status("Cancelled")?;
            return Ok(());
        }
    }

    fs::remove_dir_all(&cache_dir)?;
    fs::create_dir_all(&cache_dir)?;
    printer.success(format!("Removed {}", format_size(size)))?;
    Ok(())
}

/// Prune all cached data (downloads and toolchains)
fn prune_all(_settings: &GlobalSettings, printer: &Printer, yes: bool) -> Result<()> {
    let cache_dir = Config::tmp_dir()?;
    let toolchains_dir = Config::toolchains_dir()?;

    let cache_size = if cache_dir.exists() {
        dir_size(&cache_dir)?
    } else {
        0
    };

    let toolchains_size = if toolchains_dir.exists() {
        dir_size(&toolchains_dir)?
    } else {
        0
    };

    let total_size = cache_size + toolchains_size;

    if total_size == 0 {
        printer.status("Nothing to prune")?;
        return Ok(());
    }

    if !yes {
        writeln!(printer.stdout())?;
        printer.warning("This will remove ALL installed toolchains!")?;
        writeln!(printer.stdout())?;
        print!(
            "Remove {} total (cache + toolchains)? [y/N]: ",
            format_size(total_size)
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            printer.status("Cancelled")?;
            return Ok(());
        }
    }

    // Remove cache
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)?;
        fs::create_dir_all(&cache_dir)?;
    }

    // Remove toolchains
    if toolchains_dir.exists() {
        fs::remove_dir_all(&toolchains_dir)?;
        fs::create_dir_all(&toolchains_dir)?;
    }

    printer.success(format!("Removed {} total", format_size(total_size)))?;
    writeln!(printer.stdout())?;
    printer.hint("Reinstall toolchains with 'lemma lean install <toolchain>'")?;
    Ok(())
}

/// Calculate the size of a directory recursively
fn dir_size(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let mut size = 0;

    if path.is_file() {
        return Ok(path.metadata()?.len());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            size += metadata.len();
        } else if metadata.is_dir() {
            size += dir_size(&entry.path())?;
        }
    }

    Ok(size)
}

/// Format bytes as human-readable size
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes_f = bytes as f64;
    let i = (bytes_f.log10() / 1024_f64.log10()).floor() as usize;
    let i = i.min(UNITS.len() - 1);

    let size = bytes_f / 1024_f64.powi(i as i32);

    if i == 0 {
        format!("{} {}", bytes, UNITS[i])
    } else {
        format!("{:.2} {}", size, UNITS[i])
    }
}
