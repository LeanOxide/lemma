//! Install command - Download and install toolchains

use anyhow::Result;

use crate::config::Config;
use crate::install::Installer;

pub fn execute(toolchain: &str, force: bool) -> Result<()> {
    let config = Config::load()?;
    let installer = Installer::new(config)?;
    installer.install(toolchain, force)?;
    Ok(())
}
