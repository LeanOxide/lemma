//! Module discovery and resolution

use crate::error::{Error, Result};
use lemma_lakefile::Lakefile;
use regex::Regex;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

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

impl Module {
    /// Create a new module
    pub fn new(name: String, path: PathBuf, imports: Vec<String>) -> Self {
        Self {
            name,
            path,
            imports,
        }
    }
}

/// Discovers and resolves module dependencies
pub struct ModuleResolver {
    /// Project root directory
    project_dir: PathBuf,

    /// Source directory
    src_dir: PathBuf,

    /// Regex for parsing import statements
    import_regex: Regex,
}

impl ModuleResolver {
    /// Create a new module resolver
    pub fn new(project_dir: &Path, lakefile: &Lakefile) -> Result<Self> {
        let src_dir = project_dir.join(&lakefile.src_dir);

        // Compile regex for parsing imports
        // Matches: "import Foo.Bar" or "import Foo.Bar.Baz"
        let import_regex = Regex::new(r"^\s*import\s+([\w.]+)").map_err(|e| {
            Error::ModuleResolution(format!("Failed to compile import regex: {}", e))
        })?;

        Ok(Self {
            project_dir: project_dir.to_path_buf(),
            src_dir,
            import_regex,
        })
    }

    /// Discover all modules in the project
    ///
    /// This will walk the source directory and find all .lean files,
    /// parse their imports, and return a list of modules.
    pub fn discover_modules(&self) -> Result<Vec<Module>> {
        let mut modules = Vec::new();

        // Walk the source directory and find all .lean files
        for entry in WalkDir::new(&self.src_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only process .lean files
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("lean") {
                // Convert file path to module name
                let module_name = self.path_to_module_name(path)?;

                // Parse imports from the file
                let imports = self.parse_imports(path)?;

                // Create module
                let module = Module::new(module_name, path.to_path_buf(), imports);
                modules.push(module);
            }
        }

        Ok(modules)
    }

    /// Parse imports from a .lean file
    ///
    /// This extracts all `import Foo.Bar` statements from the file.
    /// Uses buffered reading for better performance on large files,
    /// since imports only appear at the beginning.
    pub fn parse_imports(&self, file: &Path) -> Result<Vec<String>> {
        use std::io::{BufRead, BufReader};

        let file_handle = std::fs::File::open(file).map_err(|e| {
            Error::ModuleResolution(format!("Failed to open {}: {}", file.display(), e))
        })?;

        let reader = BufReader::new(file_handle);
        let mut imports = Vec::new();

        for line_result in reader.lines() {
            let line = line_result.map_err(|e| {
                Error::ModuleResolution(format!("Failed to read line from {}: {}", file.display(), e))
            })?;

            // Stop at the first non-import, non-comment, non-blank line
            // In Lean, imports must come at the beginning of the file
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("--") {
                continue;
            }

            if let Some(captures) = self.import_regex.captures(&line) {
                if let Some(import) = captures.get(1) {
                    imports.push(import.as_str().to_string());
                }
            } else {
                // We've hit a non-import line, stop processing
                break;
            }
        }

        Ok(imports)
    }

    /// Convert a file path to a module name
    ///
    /// Example: "src/Foo/Bar.lean" -> "Foo.Bar"
    /// Example: "MyLib.lean" -> "MyLib"
    pub fn path_to_module_name(&self, path: &Path) -> Result<String> {
        // Get the path relative to the source directory
        let relative_path = path.strip_prefix(&self.src_dir).map_err(|_| {
            Error::ModuleResolution(format!(
                "Path {} is not inside source directory {}",
                path.display(),
                self.src_dir.display()
            ))
        })?;

        // Remove the .lean extension
        let path_without_ext = relative_path.with_extension("");

        // Convert path separators to dots
        let module_name = path_without_ext
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect::<Vec<_>>()
            .join(".");

        if module_name.is_empty() {
            return Err(Error::ModuleResolution(format!(
                "Failed to convert path {} to module name",
                path.display()
            )));
        }

        Ok(module_name)
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

    /// Build a dependency graph from discovered modules
    ///
    /// Returns a DependencyGraph where edges represent "depends on" relationships.
    pub fn build_dependency_graph(
        &self,
        modules: &[Module],
    ) -> Result<lemma_graph::DependencyGraph<String>> {
        let mut graph = lemma_graph::DependencyGraph::new();

        // Add all modules as nodes first
        for module in modules {
            graph.add_node_if_missing(module.name.clone());
        }

        // Add edges for imports (dependencies)
        for module in modules {
            for import in &module.imports {
                // Only add edges for imports that are local to this project
                // (external dependencies will be handled separately)
                if modules.iter().any(|m| &m.name == import) {
                    graph.add_edge_with_nodes(module.name.clone(), import.clone());
                }
            }
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project() -> (TempDir, PathBuf, Lakefile) {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path().to_path_buf();
        let src_dir = project_dir.join("src");
        fs::create_dir_all(&src_dir).unwrap();

        let lakefile = Lakefile {
            name: "TestProject".to_string(),
            version: "0.1.0".to_string(),
            src_dir: PathBuf::from("src"),
            ..Default::default()
        };

        (temp_dir, project_dir, lakefile)
    }

    #[test]
    fn test_resolve_module_path() {
        let (_temp, project_dir, lakefile) = create_test_project();
        let resolver = ModuleResolver::new(&project_dir, &lakefile).unwrap();
        let path = resolver.resolve_module_path("Foo.Bar.Baz");

        assert_eq!(path, project_dir.join("src/Foo/Bar/Baz.lean"));
    }

    #[test]
    fn test_path_to_module_name() {
        let (_temp, project_dir, lakefile) = create_test_project();
        let resolver = ModuleResolver::new(&project_dir, &lakefile).unwrap();

        let path = project_dir.join("src/Foo/Bar.lean");
        let module_name = resolver.path_to_module_name(&path).unwrap();
        assert_eq!(module_name, "Foo.Bar");

        let path = project_dir.join("src/Main.lean");
        let module_name = resolver.path_to_module_name(&path).unwrap();
        assert_eq!(module_name, "Main");
    }

    #[test]
    fn test_parse_imports() {
        let (_temp, project_dir, lakefile) = create_test_project();
        let resolver = ModuleResolver::new(&project_dir, &lakefile).unwrap();

        let test_file = project_dir.join("src/Test.lean");
        fs::write(
            &test_file,
            "import Foo.Bar\nimport Baz.Qux\n\ndef main : IO Unit := IO.println \"Hello\"\n",
        )
        .unwrap();

        let imports = resolver.parse_imports(&test_file).unwrap();
        assert_eq!(imports, vec!["Foo.Bar", "Baz.Qux"]);
    }

    #[test]
    fn test_parse_imports_with_comments() {
        let (_temp, project_dir, lakefile) = create_test_project();
        let resolver = ModuleResolver::new(&project_dir, &lakefile).unwrap();

        let test_file = project_dir.join("src/Test.lean");
        fs::write(
            &test_file,
            "-- This is a comment\nimport Foo\n-- Another comment\nimport Bar\n\ndef main : IO Unit := IO.println \"Hello\"\n",
        )
        .unwrap();

        let imports = resolver.parse_imports(&test_file).unwrap();
        assert_eq!(imports, vec!["Foo", "Bar"]);
    }

    #[test]
    fn test_discover_modules() {
        let (_temp, project_dir, lakefile) = create_test_project();
        let resolver = ModuleResolver::new(&project_dir, &lakefile).unwrap();

        // Create some test modules
        let foo_dir = project_dir.join("src/Foo");
        fs::create_dir_all(&foo_dir).unwrap();

        fs::write(
            project_dir.join("src/Main.lean"),
            "import Foo.Bar\n\ndef main : IO Unit := pure ()",
        )
        .unwrap();
        fs::write(
            project_dir.join("src/Foo/Bar.lean"),
            "import Foo.Baz\n\ndef bar : Nat := 42",
        )
        .unwrap();
        fs::write(
            project_dir.join("src/Foo/Baz.lean"),
            "def baz : String := \"hello\"",
        )
        .unwrap();

        let modules = resolver.discover_modules().unwrap();
        assert_eq!(modules.len(), 3);

        // Check that modules have correct names
        let names: Vec<_> = modules.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"Main"));
        assert!(names.contains(&"Foo.Bar"));
        assert!(names.contains(&"Foo.Baz"));

        // Check imports
        let main_module = modules.iter().find(|m| m.name == "Main").unwrap();
        assert_eq!(main_module.imports, vec!["Foo.Bar"]);

        let bar_module = modules.iter().find(|m| m.name == "Foo.Bar").unwrap();
        assert_eq!(bar_module.imports, vec!["Foo.Baz"]);

        let baz_module = modules.iter().find(|m| m.name == "Foo.Baz").unwrap();
        assert!(baz_module.imports.is_empty());
    }

    #[test]
    fn test_build_dependency_graph() {
        let (_temp, project_dir, lakefile) = create_test_project();
        let resolver = ModuleResolver::new(&project_dir, &lakefile).unwrap();

        // Create test modules with dependencies
        fs::write(project_dir.join("src/A.lean"), "def a : Nat := 1").unwrap();
        fs::write(
            project_dir.join("src/B.lean"),
            "import A\n\ndef b : Nat := 2",
        )
        .unwrap();
        fs::write(
            project_dir.join("src/C.lean"),
            "import A\nimport B\n\ndef c : Nat := 3",
        )
        .unwrap();

        let modules = resolver.discover_modules().unwrap();
        let graph = resolver.build_dependency_graph(&modules).unwrap();

        // Check graph structure
        assert_eq!(graph.len(), 3);
        assert!(graph.contains(&"A".to_string()));
        assert!(graph.contains(&"B".to_string()));
        assert!(graph.contains(&"C".to_string()));

        // Check dependencies
        let b_deps = graph.dependencies(&"B".to_string()).unwrap();
        assert!(b_deps.contains(&"A".to_string()));

        let c_deps = graph.dependencies(&"C".to_string()).unwrap();
        assert!(c_deps.contains(&"A".to_string()));
        assert!(c_deps.contains(&"B".to_string()));

        // Test topological sort
        let sorted = graph.topological_sort().unwrap();
        // A should come before B and C
        let a_pos = sorted.iter().position(|x| x == "A").unwrap();
        let b_pos = sorted.iter().position(|x| x == "B").unwrap();
        let c_pos = sorted.iter().position(|x| x == "C").unwrap();
        assert!(a_pos < b_pos);
        assert!(a_pos < c_pos);
        assert!(b_pos < c_pos); // B should come before C since C depends on B
    }
}
