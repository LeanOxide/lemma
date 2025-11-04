//! Default command - Set the default toolchain

use anyhow::Result;
use colored::Colorize;

pub fn execute(_toolchain: &str) -> Result<()> {
    println!("{} Setting default toolchain...", "=>".yellow().bold());
    Ok(())
}
