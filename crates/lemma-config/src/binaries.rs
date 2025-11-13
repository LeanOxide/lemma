//! Toolchain binary resolution
//!
//! This module provides a stable interface for obtaining paths to Lean toolchain
//! binaries (lean, lake, leanc, etc.). It properly respects toolchain resolution
//! order (environment variables, overrides, project files, defaults).

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::resolution::{find_tool_binary, resolve_toolchain_or_fail};

/// Represents the resolved binary paths for a Lean toolchain
#[derive(Debug, Clone)]
pub struct ToolchainBinaries {
    /// The name of the toolchain (e.g., "stable", "v4.24.0")
    pub toolchain_name: String,
    /// Path to the lean binary
    pub lean: PathBuf,
    /// Path to the lake binary
    pub lake: PathBuf,
    /// Path to the leanc compiler (if available in toolchain)
    pub leanc: Option<PathBuf>,
}

impl ToolchainBinaries {
    /// Resolve the active toolchain binaries
    ///
    /// This respects the full toolchain resolution order:
    /// 1. Explicit toolchain override
    /// 2. LEMMA_TOOLCHAIN environment variable
    /// 3. Directory overrides
    /// 4. Project files (lean-toolchain or leanpkg.toml)
    /// 5. Default toolchain
    ///
    /// # Arguments
    ///
    /// * `explicit_toolchain` - Optional explicit toolchain override (e.g., from +toolchain syntax)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lemma_config::ToolchainBinaries;
    ///
    /// // Resolve using the default resolution order
    /// let binaries = ToolchainBinaries::resolve(None)?;
    /// println!("Using Lean at: {}", binaries.lean.display());
    ///
    /// // Explicitly use a specific toolchain
    /// let binaries = ToolchainBinaries::resolve(Some("v4.24.0"))?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn resolve(explicit_toolchain: Option<&str>) -> Result<Self> {
        // Resolve which toolchain to use
        let toolchain_name = resolve_toolchain_or_fail(explicit_toolchain)
            .context("Failed to resolve active toolchain")?;

        Self::from_toolchain(&toolchain_name)
    }

    /// Get binaries for a specific toolchain by name
    ///
    /// Unlike `resolve()`, this doesn't check the resolution order - it directly
    /// uses the specified toolchain name.
    ///
    /// # Arguments
    ///
    /// * `toolchain_name` - The name of the toolchain (e.g., "stable", "v4.24.0")
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lemma_config::ToolchainBinaries;
    ///
    /// let binaries = ToolchainBinaries::from_toolchain("stable")?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn from_toolchain(toolchain_name: &str) -> Result<Self> {
        let lean = find_tool_binary(toolchain_name, "lean").with_context(|| {
            format!(
                "Failed to find 'lean' binary in toolchain '{}'",
                toolchain_name
            )
        })?;

        let lake = find_tool_binary(toolchain_name, "lake").with_context(|| {
            format!(
                "Failed to find 'lake' binary in toolchain '{}'",
                toolchain_name
            )
        })?;

        // leanc is optional - it might not be in the toolchain
        let leanc = find_tool_binary(toolchain_name, "leanc").ok();

        Ok(Self {
            toolchain_name: toolchain_name.to_string(),
            lean,
            lake,
            leanc,
        })
    }

    /// Find a C compiler for compiling Lean native code
    ///
    /// This tries to find a suitable C compiler in the following order:
    /// 1. `leanc` from the toolchain (if available)
    /// 2. System `gcc`
    /// 3. System `clang`
    ///
    /// Returns an error if no suitable compiler is found.
    pub fn find_c_compiler(&self) -> Result<PathBuf> {
        // First, try leanc from the toolchain
        if let Some(leanc) = &self.leanc {
            return Ok(leanc.clone());
        }

        // Fall back to system compilers
        which::which("gcc")
            .or_else(|_| which::which("clang"))
            .map_err(|_| {
                anyhow::anyhow!(
                    "No C compiler found. Toolchain '{}' doesn't include leanc, \
                     and no system compiler (gcc or clang) was found in PATH.",
                    self.toolchain_name
                )
            })
    }

    /// Get just the lean binary path from the active toolchain
    ///
    /// This is a convenience method for cases where you only need the lean binary.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use lemma_config::ToolchainBinaries;
    ///
    /// let lean = ToolchainBinaries::lean_binary(None)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn lean_binary(explicit_toolchain: Option<&str>) -> Result<PathBuf> {
        let binaries = Self::resolve(explicit_toolchain)?;
        Ok(binaries.lean)
    }

    /// Get just the lake binary path from the active toolchain
    ///
    /// This is a convenience method for cases where you only need the lake binary.
    pub fn lake_binary(explicit_toolchain: Option<&str>) -> Result<PathBuf> {
        let binaries = Self::resolve(explicit_toolchain)?;
        Ok(binaries.lake)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toolchain_binaries_display() {
        let binaries = ToolchainBinaries {
            toolchain_name: "v4.24.0".to_string(),
            lean: PathBuf::from("/home/user/.lemma/toolchains/v4.24.0-linux/bin/lean"),
            lake: PathBuf::from("/home/user/.lemma/toolchains/v4.24.0-linux/bin/lake"),
            leanc: Some(PathBuf::from(
                "/home/user/.lemma/toolchains/v4.24.0-linux/bin/leanc",
            )),
        };

        assert_eq!(binaries.toolchain_name, "v4.24.0");
        assert!(binaries.lean.ends_with("lean"));
        assert!(binaries.lake.ends_with("lake"));
        assert!(binaries.leanc.is_some());
    }
}
