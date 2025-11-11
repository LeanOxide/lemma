//! Compilation driver - Invokes the Lean compiler

use crate::error::{Error, Result};
use crate::module::Module;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// The compilation driver is responsible for invoking the Lean compiler
pub struct CompilationDriver {
    /// Path to the lean binary
    lean_binary: PathBuf,

    /// Additional compiler flags
    flags: Vec<String>,

    /// Project source directory
    src_dir: PathBuf,

    /// Build output directory
    build_dir: PathBuf,
}

impl CompilationDriver {
    /// Create a new compilation driver
    ///
    /// The lean_binary should be the path to the lean compiler executable.
    pub fn new(
        lean_binary: PathBuf,
        src_dir: PathBuf,
        build_dir: PathBuf,
    ) -> Self {
        Self {
            lean_binary,
            flags: Vec::new(),
            src_dir,
            build_dir,
        }
    }

    /// Add a compiler flag
    pub fn add_flag(&mut self, flag: String) {
        self.flags.push(flag);
    }

    /// Get the output directory for a module's artifacts
    ///
    /// Example: "Foo.Bar" -> "build/lib/Foo/Bar"
    fn get_module_output_dir(&self, module: &Module) -> PathBuf {
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("lib");
        for part in &parts[..parts.len().saturating_sub(1)] {
            path.push(part);
        }
        path
    }

    /// Get the path for a module's .olean artifact
    fn get_olean_path(&self, module: &Module) -> PathBuf {
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("lib");
        for part in parts {
            path.push(part);
        }
        path.set_extension("olean");
        path
    }

    /// Compile a module
    ///
    /// This will invoke the lean compiler to produce:
    /// - .olean file (compiled module)
    /// - .ilean file (interface file)
    /// - .c file (C code for code generation)
    pub async fn compile_module(&self, module: &Module, _output_dir: &Path) -> Result<()> {
        // Create output directory for artifacts
        let output_dir = self.get_module_output_dir(module);
        std::fs::create_dir_all(&output_dir)?;

        // Build the lean compiler command
        let mut cmd = Command::new(&self.lean_binary);

        // Set up LEAN_PATH environment variable
        // This tells the compiler where to find compiled dependencies
        let lib_path = self.build_dir.join("lib");
        cmd.env("LEAN_PATH", lib_path.to_str().unwrap_or(""));

        // Add the source directory to LEAN_SRC_PATH
        cmd.env("LEAN_SRC_PATH", self.src_dir.to_str().unwrap_or(""));

        // Set output directory
        cmd.arg("-o");
        cmd.arg(self.get_olean_path(module));

        // Add custom flags
        for flag in &self.flags {
            cmd.arg(flag);
        }

        // Add the module source file
        cmd.arg(&module.path);

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Execute the compiler
        let output = cmd.output().await.map_err(|e| {
            Error::Compilation(format!(
                "Failed to execute lean compiler for module '{}': {}",
                module.name, e
            ))
        })?;

        // Check if compilation succeeded
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            let error_msg = if !stderr.is_empty() {
                stderr.to_string()
            } else if !stdout.is_empty() {
                stdout.to_string()
            } else {
                format!("Compilation failed with exit code: {:?}", output.status.code())
            };

            return Err(Error::Compilation(format!(
                "Failed to compile module '{}': {}",
                module.name, error_msg
            )));
        }

        // Verify that the .olean file was created
        let olean_path = self.get_olean_path(module);
        if !olean_path.exists() {
            return Err(Error::Compilation(format!(
                "Compilation succeeded but .olean file not found at {}",
                olean_path.display()
            )));
        }

        Ok(())
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
        let driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from("build"),
        );
        assert_eq!(driver.lean_binary, PathBuf::from("/usr/bin/lean"));
        assert_eq!(driver.src_dir, PathBuf::from("src"));
        assert_eq!(driver.build_dir, PathBuf::from("build"));
        assert!(driver.flags.is_empty());
    }

    #[test]
    fn test_add_flag() {
        let mut driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from("build"),
        );
        driver.add_flag("--verbose".to_string());
        assert_eq!(driver.flags.len(), 1);
        assert_eq!(driver.flags[0], "--verbose");
    }

    #[test]
    fn test_get_module_output_dir() {
        let driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from("build"),
        );

        let module = Module::new(
            "Foo.Bar.Baz".to_string(),
            PathBuf::from("src/Foo/Bar/Baz.lean"),
            vec![],
        );

        let output_dir = driver.get_module_output_dir(&module);
        assert_eq!(output_dir, PathBuf::from("build/lib/Foo/Bar"));
    }

    #[test]
    fn test_get_olean_path() {
        let driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from("build"),
        );

        let module = Module::new(
            "Foo.Bar".to_string(),
            PathBuf::from("src/Foo/Bar.lean"),
            vec![],
        );

        let olean_path = driver.get_olean_path(&module);
        assert_eq!(olean_path, PathBuf::from("build/lib/Foo/Bar.olean"));
    }
}
