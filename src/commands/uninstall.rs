//! Uninstall command - Remove installed toolchains

use anyhow::Result;
use colored::Colorize;

pub fn execute(_toolchain: &str) -> Result<()> {
    println!("{} Uninstall support coming soon...", "=>".yellow().bold());
    Ok(())
}
