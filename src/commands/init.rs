//! Init command - Initialize lemma configuration

use anyhow::Result;
use colored::Colorize;

use crate::config::Config;

pub fn execute(no_path: bool, default_toolchain: &str) -> Result<()> {
    println!("{} Initializing lemma...", "=>".green().bold());

    let config = Config::default();
    config.save()?;

    let config_path = Config::config_path()?;
    println!("   Created config at: {}", config_path.display());

    if !no_path {
        println!(
            "\n{} Add the following to your shell profile:",
            "Note:".yellow().bold()
        );
        let lemma_home = Config::lemma_home()?;
        println!(
            "   export PATH=\"{}:$PATH\"",
            lemma_home.join("bin").display()
        );
    }

    println!(
        "\n{} Run 'lemma install {}' to install the default toolchain",
        "Next:".green().bold(),
        default_toolchain
    );

    Ok(())
}
