//! Target specification parsing and resolution
//!
//! This module implements Lake-compatible target syntax:
//! `[@[<package>]/][<target>|[+]<module>][:<facet>]`
//!
//! Examples:
//! - `MyModule` - Build default facets of module MyModule
//! - `+MyModule` - Explicitly specify a module (not a file path)
//! - `MyModule:olean` - Build only the .olean file for MyModule
//! - `MyModule:c` - Build only the .c file for MyModule
//! - `@/mylib` - Build target 'mylib' in root package
//! - `myexe` - Build executable 'myexe'
//! - `Foo/Bar.lean:o` - Build object file from source path

use crate::error::{Error, Result};
use crate::module::Module;
use lemma_lakefile::{ExecutableTarget, Lakefile, LibraryTarget};
use std::path::Path;

/// A facet specifies what artifact to build for a target
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Facet {
    // Module facets
    /// Dependencies of the module
    Deps,
    /// All Lean artifacts (default): .olean, .ilean, .c files
    LeanArts,
    /// Binary blob of Lean data for importers
    Olean,
    /// Binary blob of metadata for LSP
    Ilean,
    /// Compiled C file
    C,
    /// Compiled LLVM bitcode file
    Bc,
    /// Compiled object file from C
    CO,
    /// Compiled object file from LLVM bitcode
    BcO,
    /// Compiled object file (configured backend)
    O,
    /// Shared library for --load-dynlib
    Dynlib,

    // Library facets
    /// Static library (.a file)
    Static,
    /// Shared library (.so, .dll, .dylib)
    Shared,

    // Executable facets
    /// Executable binary
    Exe,

    // Package facet
    /// Default target(s) of package
    Default,
}

impl Facet {
    /// Parse a facet from a string
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "deps" => Ok(Facet::Deps),
            "leanArts" => Ok(Facet::LeanArts),
            "olean" => Ok(Facet::Olean),
            "ilean" => Ok(Facet::Ilean),
            "c" => Ok(Facet::C),
            "bc" => Ok(Facet::Bc),
            "c.o" => Ok(Facet::CO),
            "bc.o" => Ok(Facet::BcO),
            "o" => Ok(Facet::O),
            "dynlib" => Ok(Facet::Dynlib),
            "static" => Ok(Facet::Static),
            "shared" => Ok(Facet::Shared),
            "exe" => Ok(Facet::Exe),
            "default" => Ok(Facet::Default),
            _ => Err(Error::InvalidTarget(format!("Unknown facet: {}", s))),
        }
    }

    /// Get the file extension for this facet
    pub fn extension(&self) -> Option<&'static str> {
        match self {
            Facet::Olean => Some("olean"),
            Facet::Ilean => Some("ilean"),
            Facet::C => Some("c"),
            Facet::Bc => Some("bc"),
            Facet::CO | Facet::BcO | Facet::O => Some("o"),
            _ => None,
        }
    }
}

/// A resolved build target
#[derive(Debug, Clone)]
pub enum BuildTarget {
    /// Build a specific module with a facet
    Module { module: Module, facet: Facet },
    /// Build a library with a facet
    Library {
        library: LibraryTarget,
        facet: Facet,
    },
    /// Build an executable
    Executable { executable: ExecutableTarget },
    /// Build all default targets of the package
    Package { facet: Option<Facet> },
}

impl BuildTarget {
    /// Get a human-readable description of this target
    pub fn description(&self) -> String {
        match self {
            BuildTarget::Module { module, facet } => {
                format!("{}:{:?}", module.name, facet)
            }
            BuildTarget::Library { library, facet } => {
                format!("{}:{:?}", library.name, facet)
            }
            BuildTarget::Executable { executable } => executable.name.clone(),
            BuildTarget::Package { facet } => {
                if let Some(f) = facet {
                    format!("package:{:?}", f)
                } else {
                    "package".to_string()
                }
            }
        }
    }
}

/// Target specification parser
pub struct TargetSpec<'a> {
    lakefile: &'a Lakefile,
    project_dir: &'a Path,
    modules: &'a [Module],
}

impl<'a> TargetSpec<'a> {
    /// Create a new target specification parser
    pub fn new(lakefile: &'a Lakefile, project_dir: &'a Path, modules: &'a [Module]) -> Self {
        Self {
            lakefile,
            project_dir,
            modules,
        }
    }

    /// Parse a target specification string
    ///
    /// Syntax: `[@[<package>]/][<target>|[+]<module>][:<facet>]`
    pub fn parse(&self, spec: &str) -> Result<Vec<BuildTarget>> {
        // Split by ':' to separate target from facet
        let parts: Vec<&str> = spec.split(':').collect();

        let (target_spec, facet_str) = match parts.as_slice() {
            [target] => (*target, None),
            [target, facet] => (*target, Some(*facet)),
            _ => {
                return Err(Error::InvalidTarget(format!(
                    "Invalid target specification: {}. Too many ':' separators",
                    spec
                )))
            }
        };

        // Parse facet if specified
        let facet = if let Some(f) = facet_str {
            Some(Facet::from_str(f)?)
        } else {
            None
        };

        // Handle empty target (build default)
        if target_spec.is_empty() {
            return self.resolve_default_targets();
        }

        // Check if target starts with '@' (package specifier)
        if target_spec.starts_with('@') {
            return self.parse_package_target(&target_spec[1..], facet);
        }

        // Check if target starts with '+' (explicit module)
        if target_spec.starts_with('+') {
            let module_name = &target_spec[1..];
            return self.resolve_module_target(module_name, facet);
        }

        // Check if it's a file path (contains / or ends with .lean)
        if target_spec.contains('/') || target_spec.ends_with(".lean") {
            return self.resolve_file_path_target(target_spec, facet);
        }

        // Try to resolve as module, library, or executable
        self.resolve_ambiguous_target(target_spec, facet)
    }

    /// Parse a package-scoped target: `@[<package>/][<target>]`
    fn parse_package_target(&self, spec: &str, facet: Option<Facet>) -> Result<Vec<BuildTarget>> {
        // Split by '/'
        let parts: Vec<&str> = spec.split('/').collect();

        match parts.as_slice() {
            // @/ or @ - root package default targets
            [""] | [] => self.resolve_default_targets(),
            // @/target - root package specific target
            ["", target] => self.resolve_target_in_package(target, facet),
            // @package - build package default targets
            [package] if parts.len() == 1 => {
                // For now, we only support root package
                if package.is_empty() || *package == self.lakefile.name {
                    self.resolve_default_targets()
                } else {
                    Err(Error::InvalidTarget(format!(
                        "Package '{}' not found. Multi-package workspaces not yet supported",
                        package
                    )))
                }
            }
            // @package/target
            [package, target] => {
                if *package == self.lakefile.name || package.is_empty() {
                    self.resolve_target_in_package(target, facet)
                } else {
                    Err(Error::InvalidTarget(format!(
                        "Package '{}' not found. Multi-package workspaces not yet supported",
                        package
                    )))
                }
            }
            _ => Err(Error::InvalidTarget(format!(
                "Invalid package target specification: @{}",
                spec
            ))),
        }
    }

    /// Resolve a target within the current package
    fn resolve_target_in_package(
        &self,
        target: &str,
        facet: Option<Facet>,
    ) -> Result<Vec<BuildTarget>> {
        // Check if target starts with '+'
        if target.starts_with('+') {
            return self.resolve_module_target(&target[1..], facet);
        }

        self.resolve_ambiguous_target(target, facet)
    }

    /// Resolve module target by name
    fn resolve_module_target(
        &self,
        module_name: &str,
        facet: Option<Facet>,
    ) -> Result<Vec<BuildTarget>> {
        // Find the module
        let module = self
            .modules
            .iter()
            .find(|m| m.name == module_name)
            .ok_or_else(|| Error::InvalidTarget(format!("Module '{}' not found", module_name)))?;

        let facet = facet.unwrap_or(Facet::LeanArts);
        Ok(vec![BuildTarget::Module {
            module: module.clone(),
            facet,
        }])
    }

    /// Resolve a file path target (e.g., "Foo/Bar.lean:o")
    fn resolve_file_path_target(
        &self,
        path_spec: &str,
        facet: Option<Facet>,
    ) -> Result<Vec<BuildTarget>> {
        // Remove .lean extension if present
        let path_without_ext = path_spec.strip_suffix(".lean").unwrap_or(path_spec);

        // Convert path to module name
        let module_name = path_without_ext.replace('/', ".");

        self.resolve_module_target(&module_name, facet)
    }

    /// Resolve an ambiguous target (could be module, library, or executable)
    fn resolve_ambiguous_target(
        &self,
        target: &str,
        facet: Option<Facet>,
    ) -> Result<Vec<BuildTarget>> {
        // Try module first
        if let Some(module) = self.modules.iter().find(|m| m.name == target) {
            let facet = facet.unwrap_or(Facet::LeanArts);
            return Ok(vec![BuildTarget::Module {
                module: module.clone(),
                facet,
            }]);
        }

        // Try library
        if let Some(library) = self.lakefile.libraries.iter().find(|l| l.name == target) {
            let facet = facet.unwrap_or(Facet::LeanArts);
            return Ok(vec![BuildTarget::Library {
                library: library.clone(),
                facet,
            }]);
        }

        // Try executable
        if let Some(executable) = self.lakefile.executables.iter().find(|e| e.name == target) {
            if facet.is_some() && facet != Some(Facet::Exe) {
                return Err(Error::InvalidTarget(format!(
                    "Executable '{}' only supports :exe facet",
                    target
                )));
            }
            return Ok(vec![BuildTarget::Executable {
                executable: executable.clone(),
            }]);
        }

        Err(Error::InvalidTarget(format!(
            "Target '{}' not found (not a module, library, or executable)",
            target
        )))
    }

    /// Resolve default targets for the package
    fn resolve_default_targets(&self) -> Result<Vec<BuildTarget>> {
        let mut targets = Vec::new();

        // If default_targets is specified in lakefile, use those
        if !self.lakefile.default_targets.is_empty() {
            for target_name in &self.lakefile.default_targets {
                targets.extend(self.resolve_ambiguous_target(target_name, None)?);
            }
            return Ok(targets);
        }

        // Otherwise, build all libraries and executables
        for library in &self.lakefile.libraries {
            targets.push(BuildTarget::Library {
                library: library.clone(),
                facet: Facet::LeanArts,
            });
        }

        for executable in &self.lakefile.executables {
            targets.push(BuildTarget::Executable {
                executable: executable.clone(),
            });
        }

        // If no targets defined, build all modules
        if targets.is_empty() {
            targets.push(BuildTarget::Package { facet: None });
        }

        Ok(targets)
    }

    /// Parse multiple target specifications
    pub fn parse_multiple(&self, specs: &[String]) -> Result<Vec<BuildTarget>> {
        if specs.is_empty() {
            return self.resolve_default_targets();
        }

        let mut all_targets = Vec::new();
        for spec in specs {
            all_targets.extend(self.parse(spec)?);
        }
        Ok(all_targets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_lakefile() -> Lakefile {
        Lakefile {
            name: "TestProject".to_string(),
            version: "0.1.0".to_string(),
            src_dir: PathBuf::from("."),
            libraries: vec![LibraryTarget {
                name: "MyLib".to_string(),
                root: None,
                globs: vec![],
                src_dir: None,
                more_lean_args: vec![],
                native_facets: true,
                static_lib: true,
                deps: vec![],
            }],
            executables: vec![ExecutableTarget {
                name: "myexe".to_string(),
                root: Some("Main".to_string()),
                src_dir: None,
                exe_name: None,
                support_interpreter: false,
                more_lean_args: vec![],
                more_link_args: vec![],
                deps: vec![],
            }],
            ..Default::default()
        }
    }

    fn create_test_modules() -> Vec<Module> {
        vec![
            Module::new("Main".to_string(), PathBuf::from("Main.lean"), vec![]),
            Module::new("Foo.Bar".to_string(), PathBuf::from("Foo/Bar.lean"), vec![]),
        ]
    }

    #[test]
    fn test_parse_module_with_facet() {
        let lakefile = create_test_lakefile();
        let modules = create_test_modules();
        let parser = TargetSpec::new(&lakefile, Path::new("."), &modules);

        let targets = parser.parse("Main:olean").unwrap();
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            BuildTarget::Module { module, facet } => {
                assert_eq!(module.name, "Main");
                assert_eq!(*facet, Facet::Olean);
            }
            _ => panic!("Expected module target"),
        }
    }

    #[test]
    fn test_parse_module_explicit() {
        let lakefile = create_test_lakefile();
        let modules = create_test_modules();
        let parser = TargetSpec::new(&lakefile, Path::new("."), &modules);

        let targets = parser.parse("+Main").unwrap();
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            BuildTarget::Module { module, facet } => {
                assert_eq!(module.name, "Main");
                assert_eq!(*facet, Facet::LeanArts);
            }
            _ => panic!("Expected module target"),
        }
    }

    #[test]
    fn test_parse_file_path() {
        let lakefile = create_test_lakefile();
        let modules = create_test_modules();
        let parser = TargetSpec::new(&lakefile, Path::new("."), &modules);

        let targets = parser.parse("Foo/Bar.lean:o").unwrap();
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            BuildTarget::Module { module, facet } => {
                assert_eq!(module.name, "Foo.Bar");
                assert_eq!(*facet, Facet::O);
            }
            _ => panic!("Expected module target"),
        }
    }

    #[test]
    fn test_parse_library() {
        let lakefile = create_test_lakefile();
        let modules = create_test_modules();
        let parser = TargetSpec::new(&lakefile, Path::new("."), &modules);

        let targets = parser.parse("MyLib:static").unwrap();
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            BuildTarget::Library { library, facet } => {
                assert_eq!(library.name, "MyLib");
                assert_eq!(*facet, Facet::Static);
            }
            _ => panic!("Expected library target"),
        }
    }

    #[test]
    fn test_parse_executable() {
        let lakefile = create_test_lakefile();
        let modules = create_test_modules();
        let parser = TargetSpec::new(&lakefile, Path::new("."), &modules);

        let targets = parser.parse("myexe").unwrap();
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            BuildTarget::Executable { executable } => {
                assert_eq!(executable.name, "myexe");
            }
            _ => panic!("Expected executable target"),
        }
    }

    #[test]
    fn test_parse_package_target() {
        let lakefile = create_test_lakefile();
        let modules = create_test_modules();
        let parser = TargetSpec::new(&lakefile, Path::new("."), &modules);

        let targets = parser.parse("@/myexe").unwrap();
        assert_eq!(targets.len(), 1);
        match &targets[0] {
            BuildTarget::Executable { executable } => {
                assert_eq!(executable.name, "myexe");
            }
            _ => panic!("Expected executable target"),
        }
    }
}
