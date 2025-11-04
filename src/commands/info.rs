//! Info command - Display lemma information

use anyhow::Result;
use colored::Colorize;

use crate::config::Config;

pub fn execute() -> Result<()> {
    println!("{}", "Lemma - A Modern Lean4 Toolchain Manager".bold());
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("{}", "Installation Directory:".bold());
    let lemma_home = Config::lemma_home()?;
    println!("  {}", lemma_home.display());
    println!();
    println!("{}", "Configuration File:".bold());
    let config_path = Config::config_path()?;
    println!("  {}", config_path.display());

    Ok(())
}
