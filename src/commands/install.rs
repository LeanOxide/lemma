//! Install command - Download and install toolchains

use anyhow::Result;

use crate::install::Installer;

pub fn execute(toolchain: &str, force: bool) -> Result<()> {
    let installer = Installer::new()?;
    installer.install(toolchain, force)?;
    Ok(())
}
