//! Update command - Update installed toolchains

use anyhow::Result;
use colored::Colorize;

pub fn execute(_toolchain: Option<&str>) -> Result<()> {
    println!("{} Update support coming soon...", "=>".yellow().bold());
    Ok(())
}
