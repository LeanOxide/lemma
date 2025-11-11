//! Module discovery and resolution

use crate::error::Result;
use lemma_lakefile::Lakefile;
use std::path::{Path, PathBuf};

/// A Lean module (a .lean source file)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Module {
    /// Module name (e.g., "Foo.Bar")
    pub name: String,

    /// File system path to the .lean file
    pub path: PathBuf,

    /// Direct imports (module names this module imports)
    pub imports: Vec<String>,
}

/// Discovers and resolves module dependencies
pub struct ModuleResolver {
    /// Source directory
    src_dir: PathBuf,
}

impl ModuleResolver {
    /// Create a new module resolver
    pub fn new(project_dir: &Path, lakefile: &Lakefile) -> Result<Self> {
        let src_dir = project_dir.join(&lakefile.src_dir);

        Ok(Self { src_dir })
    }

    /// Discover all modules in the project
    ///
    /// This will walk the source directory and find all .lean files.
    /// TODO: Implement in Phase 1
    pub fn discover_modules(&self) -> Result<Vec<Module>> {
        // TODO: Phase 1 - Walk directory tree and find all .lean files
        // TODO: Phase 1 - Convert file paths to module names
        Ok(Vec::new())
    }

    /// Parse imports from a .lean file
    ///
    /// This extracts all `import Foo.Bar` statements from the file.
    /// TODO: Implement in Phase 1
    pub fn parse_imports(&self, _file: &Path) -> Result<Vec<String>> {
        // TODO: Phase 1 - Read file and extract import statements
        // Using regex: r"^\s*import\s+([\w.]+)"
        Ok(Vec::new())
    }

    /// Resolve a module name to a file path
    ///
    /// Example: "Foo.Bar" -> "src/Foo/Bar.lean"
    pub fn resolve_module_path(&self, module_name: &str) -> PathBuf {
        let parts: Vec<&str> = module_name.split('.').collect();
        let mut path = self.src_dir.clone();
        for part in parts {
            path.push(part);
        }
        path.set_extension("lean");
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_module_path() {
        let lakefile = lemma_lakefile::Lakefile {
            name: "test".to_string(),
            version: "0.1.0".to_string(),
            src_dir: PathBuf::from("src"),
            ..Default::default()
        };

        let resolver = ModuleResolver::new(Path::new("/project"), &lakefile).unwrap();
        let path = resolver.resolve_module_path("Foo.Bar.Baz");

        assert_eq!(
            path,
            PathBuf::from("/project/src/Foo/Bar/Baz.lean")
        );
    }
}
