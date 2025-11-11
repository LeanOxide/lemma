//! Compilation driver - Invokes the Lean compiler

use crate::error::{Error, Result};
use crate::module::Module;
use std::path::Path;

/// The compilation driver is responsible for invoking the Lean compiler
pub struct CompilationDriver {
    /// Path to the lean binary
    #[allow(dead_code)]
    lean_binary: std::path::PathBuf,

    /// Additional compiler flags
    flags: Vec<String>,
}

impl CompilationDriver {
    /// Create a new compilation driver
    ///
    /// The lean_binary should be the path to the lean compiler executable.
    pub fn new(lean_binary: std::path::PathBuf) -> Self {
        Self {
            lean_binary,
            flags: Vec::new(),
        }
    }

    /// Add a compiler flag
    pub fn add_flag(&mut self, flag: String) {
        self.flags.push(flag);
    }

    /// Compile a module
    ///
    /// This will invoke the lean compiler to produce:
    /// - .olean file (compiled module)
    /// - .c file (C code for code generation)
    /// - .o file (object file from C code)
    ///
    /// TODO: Implement in Phase 5
    pub async fn compile_module(&self, _module: &Module, _output_dir: &Path) -> Result<()> {
        // TODO: Phase 5 - Build lean compiler command
        // TODO: Phase 5 - Set up environment (LEAN_PATH, etc.)
        // TODO: Phase 5 - Execute compiler
        // TODO: Phase 5 - Check for errors
        // TODO: Phase 5 - Compile generated C code
        Err(Error::Other(
            "Compilation not yet implemented. This is Phase 0.".to_string(),
        ))
    }

    /// Link an executable
    ///
    /// This will link together object files to create an executable.
    ///
    /// TODO: Implement in Phase 6
    pub async fn link_executable(
        &self,
        _name: &str,
        _object_files: &[std::path::PathBuf],
        _output: &Path,
    ) -> Result<()> {
        // TODO: Phase 6 - Invoke linker (usually leanc or clang)
        // TODO: Phase 6 - Handle link flags
        // TODO: Phase 6 - Create executable
        Err(Error::Other(
            "Linking not yet implemented. This is Phase 0.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_driver() {
        let driver = CompilationDriver::new(std::path::PathBuf::from("/usr/bin/lean"));
        assert_eq!(driver.lean_binary, std::path::PathBuf::from("/usr/bin/lean"));
        assert!(driver.flags.is_empty());
    }

    #[test]
    fn test_add_flag() {
        let mut driver = CompilationDriver::new(std::path::PathBuf::from("/usr/bin/lean"));
        driver.add_flag("--verbose".to_string());
        assert_eq!(driver.flags.len(), 1);
        assert_eq!(driver.flags[0], "--verbose");
    }
}
