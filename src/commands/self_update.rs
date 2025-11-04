//! Self-update command - Update lemma itself

use anyhow::Result;
use colored::Colorize;

pub fn execute() -> Result<()> {
    println!(
        "{} Self-update support coming soon...",
        "=>".yellow().bold()
    );
    Ok(())
}
