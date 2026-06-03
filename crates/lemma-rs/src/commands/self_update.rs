//! Self-management commands for lemma.

use anyhow::{Context, Result};
use lemma_config::{Config, GlobalSettings};
use lemma_output::Printer;
use std::env;
use std::fs;

const PACKAGE_NAME: &str = "lemma";

pub fn update(settings: &GlobalSettings, printer: &Printer) -> Result<()> {
    printer.header("Update lemma")?;
    printer.status("Lemma is distributed as a Python package")?;

    let current_version = env!("CARGO_PKG_VERSION");
    printer.list_item(format!("Current version: {}", current_version))?;

    if settings.is_verbose() {
        if let Ok(current_exe) = env::current_exe() {
            printer.hint(format!("Executable: {}", current_exe.display()))?;
        }
    }

    printer.list_item("Recommended: pipx upgrade lemma")?;
    printer.list_item("Fallback: python -m pip install --user --upgrade lemma")?;
    printer.list_item("Windows fallback: py -m pip install --user --upgrade lemma")?;
    printer.list_item("Source install: cargo install lemma --force")?;

    printer.success("Run the command that matches how you installed lemma.")?;
    printer.warning(
        "Direct binary self-update is disabled so package managers keep ownership of installed files.",
    )?;

    Ok(())
}

/// Clean up old backup files from pre-PyPI self-update installations.
pub fn cleanup_old_backups() -> Result<()> {
    if let Ok(current_exe) = env::current_exe() {
        #[cfg(unix)]
        let backup = current_exe.with_extension("old");
        #[cfg(windows)]
        let backup = current_exe.with_extension("old.exe");

        if backup.exists() {
            fs::remove_file(&backup).ok();
        }
    }
    Ok(())
}

/// Uninstall lemma-managed data and toolchains.
pub fn uninstall(skip_confirm: bool, settings: &GlobalSettings, printer: &Printer) -> Result<()> {
    use std::io::{self, Write};

    printer.header("Uninstall lemma-managed data")?;
    printer.list_item("All installed Lean toolchains")?;
    printer.list_item("All lemma proxy binaries")?;
    printer.list_item("The entire ~/.lemma directory")?;
    printer.warning(format!(
        "This does not uninstall the {} Python package or remove package-manager owned executables.",
        PACKAGE_NAME
    ))?;

    if !skip_confirm {
        print!("Continue? (y/N): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            printer.hint("Uninstall cancelled")?;
            return Ok(());
        }
    }

    let lemma_home = Config::lemma_home()?;

    printer.status("Removing lemma home")?;
    if settings.is_verbose() {
        printer.hint(format!("Directory: {}", lemma_home.display()))?;
    }

    if lemma_home.exists() {
        fs::remove_dir_all(&lemma_home).context("Failed to remove lemma home directory")?;
        if settings.is_verbose() {
            printer.hint("Removed ~/.lemma")?;
        }
    }

    printer.success("Removed lemma-managed data")?;
    printer.list_item("To remove the Python package: pipx uninstall lemma")?;
    printer.list_item("Or, if installed with pip: python -m pip uninstall lemma")?;

    Ok(())
}
