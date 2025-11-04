//! List command - Show installed toolchains

use anyhow::Result;
use colored::Colorize;

pub fn execute(_verbose: bool) -> Result<()> {
    println!("{} Listing toolchains...", "=>".green().bold());
    println!("   No toolchains installed yet.");
    println!("   Run 'lemma install stable' to install the stable toolchain.");
    Ok(())
}
