//! Which command - Display the path to a binary in the active toolchain

use anyhow::Result;

use crate::settings::GlobalSettings;
use crate::toolchain;

pub fn execute(binary: &str, explicit_toolchain: Option<&str>, settings: &GlobalSettings) -> Result<()> {
    if settings.is_verbose() {
        tracing::debug!("Looking for binary: {}", binary);
        if let Some(tc) = explicit_toolchain {
            tracing::debug!("Using explicit toolchain: {}", tc);
        }
    }
    // Resolve which toolchain to use
    let toolchain_name = toolchain::resolve_toolchain_or_fail(explicit_toolchain)?;

    // Find the binary path
    let binary_path = toolchain::find_tool_binary(&toolchain_name, binary)?;

    // Print the path
    println!("{}", binary_path.display());

    Ok(())
}
