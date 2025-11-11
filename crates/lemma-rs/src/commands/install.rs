//! Install command - Download and install toolchains

use anyhow::Result;

use lemma_config::GlobalSettings;
use lemma_install::Installer;
use lemma_output::Printer;

pub fn execute(
    toolchain: &str,
    force: bool,
    lean_downloads_json_url: Option<&str>,
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    printer.hint(format!(
        "Installing toolchain '{}' to {}",
        toolchain,
        settings.lemma_home.display()
    ))?;

    let installer = Installer::with_override_url(lean_downloads_json_url)?;

    installer.install(toolchain, force)?;

    printer.success(format!("Installed toolchain '{}'", toolchain))?;
    Ok(())
}
