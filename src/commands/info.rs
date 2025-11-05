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
    println!("{}", "Settings File:".bold());
    let settings_path = Config::settings_path()?;
    println!("  {}", settings_path.display());

    Ok(())
}
