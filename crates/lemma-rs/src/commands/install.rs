//! Install command - Download and install toolchains

use anyhow::Result;

use lemma_config::GlobalSettings;
use lemma_install::Installer;

pub fn execute(toolchain: &str, force: bool, settings: &GlobalSettings) -> Result<()> {
    if settings.is_verbose() {
        tracing::debug!("Installing toolchain: {}", toolchain);
        tracing::debug!("Lemma home: {}", settings.lemma_home.display());
    }

    let installer = Installer::new()?;
    installer.install(toolchain, force)?;
    Ok(())
}
