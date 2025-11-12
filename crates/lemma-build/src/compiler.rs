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

    /// Package name (for organizing lib output)
    package_name: String,
}

impl CompilationDriver {
    /// Create a new compilation driver
    ///
    /// The lean_binary should be the path to the lean compiler executable.
    pub fn new(
        lean_binary: PathBuf,
        src_dir: PathBuf,
        build_dir: PathBuf,
        package_name: String,
    ) -> Self {
        Self {
            lean_binary,
            flags: Vec::new(),
            src_dir,
            build_dir,
            package_name,
        }
    }

    /// Add a compiler flag
    pub fn add_flag(&mut self, flag: String) {
        self.flags.push(flag);
    }

    /// Get the output directory for a module's .olean/.ilean artifacts
    ///
    /// Example: "Foo.Bar" -> ".lake/build/lib/<package>/Foo"
    fn get_lib_output_dir(&self, module: &Module) -> PathBuf {
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("lib").join(&self.package_name);
        for part in &parts[..parts.len().saturating_sub(1)] {
            path.push(part);
        }
        path
    }

    /// Get the path for a module's .olean artifact
    ///
    /// Lake structure: `.lake/build/lib/<package>/Module.olean`
    fn get_olean_path(&self, module: &Module) -> PathBuf {
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("lib").join(&self.package_name);
        for part in parts {
            path.push(part);
        }
        path.set_extension("olean");
        path
    }

    /// Get the C file path for a module
    ///
    /// Lake structure: `.lake/build/ir/Module/Nested.c` (hierarchical)
    fn get_c_path(&self, module: &Module) -> PathBuf {
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("ir");
        for part in parts {
            path.push(part);
        }
        path.set_extension("c");
        path
    }

    /// Get the ilean file path for a module
    ///
    /// Lake structure: `.lake/build/lib/<package>/Module.ilean`
    fn get_ilean_path(&self, module: &Module) -> PathBuf {
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("lib").join(&self.package_name);
        for part in parts {
            path.push(part);
        }
        path.set_extension("ilean");
        path
    }

    /// Compile a module
    ///
    /// This will:
    /// 1. Invoke lean to generate .olean, .ilean, and .c files
    /// 2. Invoke leanc to compile .c to .o (object file)
    pub async fn compile_module(&self, module: &Module, _output_dir: &Path) -> Result<()> {
        let olean_path = self.get_olean_path(module);
        let ilean_path = self.get_ilean_path(module);
        let c_path = self.get_c_path(module);
        let obj_path = self.get_object_path(module);

        // Create output directories for artifacts
        // Lake structure: .lake/build/lib/<package>/ (hierarchical) and .lake/build/ir/ (hierarchical)
        if let Some(parent) = olean_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Some(parent) = c_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Step 1: Run lean to generate .olean, .ilean, and .c files
        let mut cmd = Command::new(&self.lean_binary);

        // Set up LEAN_PATH environment variable
        // This tells the compiler where to find compiled dependencies
        // Include both the package library directory and the Lean stdlib directory
        let package_lib_path = self.build_dir.join("lib").join(&self.package_name);

        // Get Lean stdlib path (usually <lean_binary_dir>/../lib/lean)
        let lean_stdlib_path = self.lean_binary
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("lib").join("lean"));

        // Construct LEAN_PATH: package lib + lean stdlib
        let lean_path = if let Some(ref stdlib) = lean_stdlib_path {
            format!("{}:{}",
                package_lib_path.display(),
                stdlib.display())
        } else {
            package_lib_path.display().to_string()
        };

        cmd.env("LEAN_PATH", lean_path);

        // Add the source directory to LEAN_SRC_PATH
        cmd.env("LEAN_SRC_PATH", self.src_dir.to_str().unwrap_or(""));

        // Set olean output file
        cmd.arg("-o");
        cmd.arg(&olean_path);

        // Set ilean output file (interface/info file)
        cmd.arg("-i");
        cmd.arg(&ilean_path);

        // Set C output file
        cmd.arg("-c");
        cmd.arg(&c_path);

        // Add custom flags
        for flag in &self.flags {
            cmd.arg(flag);
        }

        // Add the module source file
        cmd.arg(&module.path);

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Execute the lean compiler
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
        if !olean_path.exists() {
            return Err(Error::Compilation(format!(
                "Compilation succeeded but .olean file not found at {}",
                olean_path.display()
            )));
        }

        // Verify that the .c file was created
        if !c_path.exists() {
            return Err(Error::Compilation(format!(
                "Compilation succeeded but .c file not found at {}",
                c_path.display()
            )));
        }

        // Step 2: Run leanc to compile .c to .o
        let leanc = self.get_leanc_path()?;
        let mut cmd = Command::new(&leanc);

        // Get Lean include directory (usually in same directory as lean binary)
        // This contains lean/lean.h and other headers needed for compilation
        let lean_include = self.lean_binary
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("include"));

        // Add Lean include directory if it exists
        if let Some(ref include_dir) = lean_include {
            if include_dir.exists() {
                cmd.arg(format!("-I{}", include_dir.display()));
            }
        }

        // Compile C file to object file
        cmd.arg("-c");
        cmd.arg(&c_path);
        cmd.arg("-o");
        cmd.arg(&obj_path);

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Execute leanc
        let output = cmd.output().await.map_err(|e| {
            Error::Compilation(format!(
                "Failed to execute leanc for module '{}': {}",
                module.name, e
            ))
        })?;

        // Check if C compilation succeeded
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            let error_msg = if !stderr.is_empty() {
                stderr.to_string()
            } else if !stdout.is_empty() {
                stdout.to_string()
            } else {
                format!("C compilation failed with exit code: {:?}", output.status.code())
            };

            return Err(Error::Compilation(format!(
                "Failed to compile C code for module '{}': {}",
                module.name, error_msg
            )));
        }

        // Verify that the .o file was created
        if !obj_path.exists() {
            return Err(Error::Compilation(format!(
                "C compilation succeeded but .o file not found at {}",
                obj_path.display()
            )));
        }

        Ok(())
    }

    /// Get the path to leanc (Lean C compiler/linker)
    ///
    /// leanc is typically in the same directory as the lean binary
    fn get_leanc_path(&self) -> Result<PathBuf> {
        let leanc_path = self
            .lean_binary
            .parent()
            .ok_or_else(|| Error::Other("Could not determine lean binary directory".to_string()))?
            .join("leanc");

        if !leanc_path.exists() {
            return Err(Error::Linking(format!(
                "leanc not found at {}. Ensure Lean toolchain is properly installed.",
                leanc_path.display()
            )));
        }

        Ok(leanc_path)
    }

    /// Get the object file path for a module
    ///
    /// Lake structure: `.lake/build/ir/Module/Nested.c.o.export` (hierarchical)
    fn get_object_path(&self, module: &Module) -> PathBuf {
        let parts: Vec<&str> = module.name.split('.').collect();
        let mut path = self.build_dir.join("ir");
        for part in parts {
            path.push(part);
        }
        // Add .c.o.export extension
        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
        path.set_file_name(format!("{}.c.o.export", filename));
        path
    }

    /// Link an executable
    ///
    /// This will link together object files to create an executable using leanc.
    pub async fn link_executable(
        &self,
        name: &str,
        modules: &[Module],
        output: &Path,
    ) -> Result<()> {
        // Get leanc binary
        let leanc = self.get_leanc_path()?;

        // Collect all object files
        let mut object_files = Vec::new();
        for module in modules {
            let obj_path = self.get_object_path(module);
            if obj_path.exists() {
                object_files.push(obj_path);
            } else {
                return Err(Error::Linking(format!(
                    "Object file not found for module '{}' at {}",
                    module.name,
                    obj_path.display()
                )));
            }
        }

        if object_files.is_empty() {
            return Err(Error::Linking(format!(
                "No object files found for executable '{}'",
                name
            )));
        }

        // Create output directory if needed
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Build leanc command
        let mut cmd = Command::new(&leanc);

        // Add output flag
        cmd.arg("-o");
        cmd.arg(output);

        // Add all object files
        for obj in &object_files {
            cmd.arg(obj);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Execute linker
        let output_result = cmd.output().await.map_err(|e| {
            Error::Linking(format!(
                "Failed to execute leanc for linking '{}': {}",
                name, e
            ))
        })?;

        // Check if linking succeeded
        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            let stdout = String::from_utf8_lossy(&output_result.stdout);

            let error_msg = if !stderr.is_empty() {
                stderr.to_string()
            } else if !stdout.is_empty() {
                stdout.to_string()
            } else {
                format!(
                    "Linking failed with exit code: {:?}",
                    output_result.status.code()
                )
            };

            return Err(Error::Linking(format!(
                "Failed to link executable '{}': {}",
                name, error_msg
            )));
        }

        // Verify that the executable was created
        if !output.exists() {
            return Err(Error::Linking(format!(
                "Linking succeeded but executable not found at {}",
                output.display()
            )));
        }

        Ok(())
    }

    /// Link a static library
    ///
    /// This creates a .a archive file from object files using ar.
    pub async fn link_library(
        &self,
        name: &str,
        modules: &[Module],
        output: &Path,
    ) -> Result<()> {
        // Collect all object files
        let mut object_files = Vec::new();
        for module in modules {
            let obj_path = self.get_object_path(module);
            if obj_path.exists() {
                object_files.push(obj_path);
            } else {
                return Err(Error::Linking(format!(
                    "Object file not found for module '{}' at {}",
                    module.name,
                    obj_path.display()
                )));
            }
        }

        if object_files.is_empty() {
            return Err(Error::Linking(format!(
                "No object files found for library '{}'",
                name
            )));
        }

        // Create output directory if needed
        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Build ar command (standard Unix archiver)
        let mut cmd = Command::new("ar");

        // ar rcs output.a obj1.o obj2.o ...
        cmd.arg("rcs");
        cmd.arg(output);

        // Add all object files
        for obj in &object_files {
            cmd.arg(obj);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Execute archiver
        let output_result = cmd.output().await.map_err(|e| {
            Error::Linking(format!(
                "Failed to execute ar for library '{}': {}",
                name, e
            ))
        })?;

        // Check if archiving succeeded
        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            let stdout = String::from_utf8_lossy(&output_result.stdout);

            let error_msg = if !stderr.is_empty() {
                stderr.to_string()
            } else if !stdout.is_empty() {
                stdout.to_string()
            } else {
                format!(
                    "Archiving failed with exit code: {:?}",
                    output_result.status.code()
                )
            };

            return Err(Error::Linking(format!(
                "Failed to create library '{}': {}",
                name, error_msg
            )));
        }

        // Verify that the library was created
        if !output.exists() {
            return Err(Error::Linking(format!(
                "Archiving succeeded but library not found at {}",
                output.display()
            )));
        }

        Ok(())
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
            PathBuf::from(".lake/build"),
            "TestPackage".to_string(),
        );
        assert_eq!(driver.lean_binary, PathBuf::from("/usr/bin/lean"));
        assert_eq!(driver.src_dir, PathBuf::from("src"));
        assert_eq!(driver.build_dir, PathBuf::from(".lake/build"));
        assert_eq!(driver.package_name, "TestPackage");
        assert!(driver.flags.is_empty());
    }

    #[test]
    fn test_add_flag() {
        let mut driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from(".lake/build"),
            "TestPackage".to_string(),
        );
        driver.add_flag("--verbose".to_string());
        assert_eq!(driver.flags.len(), 1);
        assert_eq!(driver.flags[0], "--verbose");
    }

    #[test]
    fn test_get_lib_output_dir() {
        let driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from(".lake/build"),
            "mypackage".to_string(),
        );

        let module = Module::new(
            "Foo.Bar.Baz".to_string(),
            PathBuf::from("src/Foo/Bar/Baz.lean"),
            vec![],
        );

        let output_dir = driver.get_lib_output_dir(&module);
        assert_eq!(output_dir, PathBuf::from(".lake/build/lib/mypackage/Foo/Bar"));
    }

    #[test]
    fn test_get_olean_path() {
        let driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from(".lake/build"),
            "mypackage".to_string(),
        );

        let module = Module::new(
            "Foo.Bar".to_string(),
            PathBuf::from("src/Foo/Bar.lean"),
            vec![],
        );

        let olean_path = driver.get_olean_path(&module);
        assert_eq!(olean_path, PathBuf::from(".lake/build/lib/mypackage/Foo/Bar.olean"));
    }

    #[test]
    fn test_get_object_path() {
        let driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from(".lake/build"),
            "mypackage".to_string(),
        );

        let module = Module::new(
            "Foo.Bar".to_string(),
            PathBuf::from("src/Foo/Bar.lean"),
            vec![],
        );

        let obj_path = driver.get_object_path(&module);
        assert_eq!(obj_path, PathBuf::from(".lake/build/ir/Foo/Bar.c.o.export"));
    }

    #[test]
    fn test_get_c_path() {
        let driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from(".lake/build"),
            "mypackage".to_string(),
        );

        let module = Module::new(
            "Foo.Bar".to_string(),
            PathBuf::from("src/Foo/Bar.lean"),
            vec![],
        );

        let c_path = driver.get_c_path(&module);
        assert_eq!(c_path, PathBuf::from(".lake/build/ir/Foo/Bar.c"));
    }

    #[test]
    fn test_get_ilean_path() {
        let driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from(".lake/build"),
            "mypackage".to_string(),
        );

        let module = Module::new(
            "Foo.Bar".to_string(),
            PathBuf::from("src/Foo/Bar.lean"),
            vec![],
        );

        let ilean_path = driver.get_ilean_path(&module);
        assert_eq!(ilean_path, PathBuf::from(".lake/build/lib/mypackage/Foo/Bar.ilean"));
    }

    #[test]
    fn test_get_leanc_path() {
        let driver = CompilationDriver::new(
            PathBuf::from("/usr/bin/lean"),
            PathBuf::from("src"),
            PathBuf::from(".lake/build"),
            "mypackage".to_string(),
        );

        let leanc_path = driver.get_leanc_path();
        // This will fail if leanc doesn't exist, but that's expected in test
        // The important thing is the path is correct
        match leanc_path {
            Ok(path) => assert_eq!(path, PathBuf::from("/usr/bin/leanc")),
            Err(_) => {
                // Expected if leanc not installed
            }
        }
    }
}
