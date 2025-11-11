//! Which command - Display the path to a binary in the active toolchain

use anyhow::Result;
use std::io::Write;

use lemma_config::GlobalSettings;
use lemma_output::Printer;

pub fn execute(
    binary: &str,
    explicit_toolchain: Option<&str>,
    _settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    printer.hint(format!("Looking for binary: {}", binary))?;
    if let Some(tc) = explicit_toolchain {
        printer.hint(format!("Using explicit toolchain: {}", tc))?;
    }

    // Resolve which toolchain to use
    let toolchain_name = lemma_config::resolve_toolchain_or_fail(explicit_toolchain)?;

    // Find the binary path
    let binary_path = lemma_config::find_tool_binary(&toolchain_name, binary)?;

    // Print the path (raw output, not formatted)
    writeln!(printer.stdout(), "{}", binary_path.display())?;

    Ok(())
}
