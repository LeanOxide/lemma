//! Centralized path management for Lean builds
//!
//! This module provides a single source of truth for:
//! - Build directory structure (matching Lake conventions)
//! - LEAN_PATH construction
//! - Artifact locations

use std::env;
use std::path::PathBuf;

/// Standard subdirectory for compiled .olean files
/// Matches Lake convention: .lake/build/lib/lean
pub const LEAN_LIB_SUBDIR: &str = "lean";

/// Build path manager - handles all path construction for builds
pub struct BuildPaths {
    /// Project root directory
    pub project_dir: PathBuf,
    /// Build directory (typically .lake/build)
    pub build_dir: PathBuf,
}

impl BuildPaths {
    /// Create a new build paths manager
    pub fn new(project_dir: PathBuf, build_dir: PathBuf) -> Self {
        Self {
            project_dir,
            build_dir,
        }
    }

    /// Get the library output directory (.lake/build/lib/lean)
    ///
    /// This is where all .olean/.ilean files are placed, matching Lake's structure.
    pub fn lib_dir(&self) -> PathBuf {
        self.build_dir.join("lib").join(LEAN_LIB_SUBDIR)
    }

    /// Get the binary output directory (.lake/build/bin)
    pub fn bin_dir(&self) -> PathBuf {
        self.build_dir.join("bin")
    }

    /// Get the IR output directory (.lake/build/ir)
    pub fn ir_dir(&self) -> PathBuf {
        self.build_dir.join("ir")
    }

    /// Get the .olean path for a module
    ///
    /// Example: "Foo.Bar" -> ".lake/build/lib/lean/Foo/Bar.olean"
    pub fn olean_path(&self, module_name: &str) -> PathBuf {
        let parts: Vec<&str> = module_name.split('.').collect();
        let mut path = self.lib_dir();
        for part in parts {
            path.push(part);
        }
        path.set_extension("olean");
        path
    }

    /// Get the .ilean path for a module
    ///
    /// Example: "Foo.Bar" -> ".lake/build/lib/lean/Foo/Bar.ilean"
    pub fn ilean_path(&self, module_name: &str) -> PathBuf {
        let parts: Vec<&str> = module_name.split('.').collect();
        let mut path = self.lib_dir();
        for part in parts {
            path.push(part);
        }
        path.set_extension("ilean");
        path
    }

    /// Get the .trace path for a module
    ///
    /// Example: "Foo.Bar" -> ".lake/build/lib/lean/Foo/Bar.trace"
    pub fn trace_path(&self, module_name: &str) -> PathBuf {
        let parts: Vec<&str> = module_name.split('.').collect();
        let mut path = self.lib_dir();
        for part in parts {
            path.push(part);
        }
        path.set_extension("trace");
        path
    }

    /// Get the .hash path for a module
    ///
    /// Example: "Foo.Bar" -> ".lake/build/lib/lean/Foo/Bar.hash"
    pub fn hash_path(&self, module_name: &str) -> PathBuf {
        let parts: Vec<&str> = module_name.split('.').collect();
        let mut path = self.lib_dir();
        for part in parts {
            path.push(part);
        }
        path.set_extension("hash");
        path
    }

    /// Get the output directory for a module (without filename)
    ///
    /// Example: "Foo.Bar.Baz" -> ".lake/build/lib/lean/Foo/Bar"
    pub fn module_output_dir(&self, module_name: &str) -> PathBuf {
        let parts: Vec<&str> = module_name.split('.').collect();
        let mut path = self.lib_dir();
        // All parts except the last become directories
        for part in &parts[..parts.len().saturating_sub(1)] {
            path.push(part);
        }
        path
    }

    /// Get the executable path
    ///
    /// Example: "myapp" -> ".lake/build/bin/myapp"
    pub fn executable_path(&self, name: &str) -> PathBuf {
        self.bin_dir().join(name)
    }

    /// Get the C file path for a module
    ///
    /// Example: "Foo.Bar" -> ".lake/build/ir/Foo/Bar.c"
    pub fn c_path(&self, module_name: &str) -> PathBuf {
        let parts: Vec<&str> = module_name.split('.').collect();
        let mut path = self.ir_dir();
        for part in parts {
            path.push(part);
        }
        path.set_extension("c");
        path
    }

    /// Get the object file path for a module
    ///
    /// Example: "Foo.Bar" -> ".lake/build/ir/Foo/Bar.c.o.export"
    pub fn object_path(&self, module_name: &str) -> PathBuf {
        let parts: Vec<&str> = module_name.split('.').collect();
        let mut path = self.ir_dir();
        for part in parts {
            path.push(part);
        }
        // Add .c.o.export extension
        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
        path.set_file_name(format!("{}.c.o.export", filename));
        path
    }
}

/// LEAN_PATH builder - constructs LEAN_PATH environment variable value
pub struct LeanPathBuilder {
    components: Vec<PathBuf>,
}

impl LeanPathBuilder {
    /// Create a new LEAN_PATH builder
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    /// Add the project's library directory
    pub fn add_project_lib(mut self, build_paths: &BuildPaths) -> Self {
        let lib_dir = build_paths.lib_dir();
        if lib_dir.exists() {
            self.components.push(lib_dir);
        }
        self
    }

    /// Add a custom library directory
    pub fn add_lib_dir(mut self, path: PathBuf) -> Self {
        if path.exists() {
            self.components.push(path);
        }
        self
    }

    /// Add the system LEAN_PATH from environment
    pub fn add_system_path(mut self) -> Self {
        if let Ok(system_path) = env::var("LEAN_PATH") {
            if !system_path.is_empty() {
                // Split by ':' and add each component
                for component in system_path.split(':') {
                    if !component.is_empty() {
                        self.components.push(PathBuf::from(component));
                    }
                }
            }
        }
        self
    }

    /// Build the final LEAN_PATH string
    ///
    /// Returns None if no components were added
    pub fn build(self) -> Option<String> {
        if self.components.is_empty() {
            None
        } else {
            Some(
                self.components
                    .iter()
                    .map(|p| p.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(":"),
            )
        }
    }

    /// Build and set as environment variable
    ///
    /// Returns the constructed path if any components were added
    pub fn build_and_set(self) -> Option<String> {
        if let Some(path) = self.build() {
            env::set_var("LEAN_PATH", &path);
            Some(path)
        } else {
            None
        }
    }
}

impl Default for LeanPathBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_build_paths() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let build_dir = project_dir.join(".lake/build");

        let paths = BuildPaths::new(project_dir.clone(), build_dir.clone());

        assert_eq!(paths.lib_dir(), build_dir.join("lib").join("lean"));

        assert_eq!(
            paths.olean_path("Foo.Bar"),
            build_dir.join("lib/lean/Foo/Bar.olean")
        );

        assert_eq!(paths.executable_path("myapp"), build_dir.join("bin/myapp"));
    }

    #[test]
    fn test_lean_path_builder() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let build_dir = project_dir.join(".lake/build");
        let lib_dir = build_dir.join("lib/lean");

        // Create the directory so it exists
        std::fs::create_dir_all(&lib_dir).unwrap();

        let paths = BuildPaths::new(project_dir, build_dir);

        let lean_path = LeanPathBuilder::new()
            .add_project_lib(&paths)
            .build()
            .unwrap();

        assert!(lean_path.contains("lib/lean"));
    }

    #[test]
    fn test_lean_path_with_system() {
        env::set_var("LEAN_PATH", "/usr/lib/lean");

        let lean_path = LeanPathBuilder::new().add_system_path().build().unwrap();

        assert_eq!(lean_path, "/usr/lib/lean");

        env::remove_var("LEAN_PATH");
    }

    #[test]
    fn test_module_output_dir() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let build_dir = project_dir.join(".lake/build");

        let paths = BuildPaths::new(project_dir, build_dir.clone());

        assert_eq!(
            paths.module_output_dir("Foo.Bar.Baz"),
            build_dir.join("lib/lean/Foo/Bar")
        );

        assert_eq!(
            paths.module_output_dir("Single"),
            build_dir.join("lib/lean")
        );
    }

    #[test]
    fn test_ir_paths() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let build_dir = project_dir.join(".lake/build");

        let paths = BuildPaths::new(project_dir, build_dir.clone());

        assert_eq!(paths.ir_dir(), build_dir.join("ir"));

        assert_eq!(paths.c_path("Foo.Bar"), build_dir.join("ir/Foo/Bar.c"));

        assert_eq!(
            paths.object_path("Foo.Bar"),
            build_dir.join("ir/Foo/Bar.c.o.export")
        );
    }
}
