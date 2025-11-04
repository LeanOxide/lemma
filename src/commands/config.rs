//! Config command - Manage configuration

use anyhow::Result;
use colored::Colorize;

use crate::config::Config;

pub fn execute(show_path: bool, edit: bool) -> Result<()> {
    let settings_path = Config::settings_path()?;

    if show_path {
        println!("{}", settings_path.display());
        return Ok(());
    }

    if edit {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
        std::process::Command::new(editor)
            .arg(&settings_path)
            .status()?;
        return Ok(());
    }

    let config = Config::load()?;
    println!("{} Current configuration:", "=>".green().bold());
    println!("{}", toml::to_string_pretty(&config)?);

    Ok(())
}
