//! Dependency analysis for Lean projects
//!
//! Analyzes Lean source files to extract import statements and compute
//! transitive dependency closures.

use anyhow::{Context, Result};
use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

/// Extract imports from a Lean file
pub fn parse_imports(file_path: &Path) -> Result<Vec<String>> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let import_re = Regex::new(r"^\s*import\s+([\w\.]+)").unwrap();
    let mut imports = Vec::new();

    for line in content.lines() {
        if let Some(captures) = import_re.captures(line) {
            if let Some(module) = captures.get(1) {
                let mut module_name = module.as_str().to_string();

                if module_name.starts_with("Init.") {
                    continue;
                }

                // Auto-prepend Mathlib. if not present
                if !module_name.starts_with("Mathlib.") {
                    module_name = format!("Mathlib.{}", module_name);
                }

                imports.push(module_name);
            }
        }
    }

    Ok(imports)
}

/// Find all Lean files in a project
pub fn find_lean_files(project_root: &Path) -> Result<Vec<PathBuf>> {
    let mut lean_files = Vec::new();

    // Skip common directories that shouldn't be scanned
    let skip_dirs = ["build", ".lake", ".git", "target"];

    visit_dirs(project_root, &mut lean_files, &skip_dirs)?;

    Ok(lean_files)
}

/// Recursively visit directories to find .lean files
fn visit_dirs(dir: &Path, lean_files: &mut Vec<PathBuf>, skip_dirs: &[&str]) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    // Skip directories we don't want to scan
    if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
        if skip_dirs.contains(&dir_name) {
            return Ok(());
        }
    }

    for entry in fs::read_dir(dir).context("Failed to read directory")? {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();

        if path.is_dir() {
            visit_dirs(&path, lean_files, skip_dirs)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("lean") {
            lean_files.push(path);
        }
    }

    Ok(())
}

/// Analyze a project and extract all imports
pub fn analyze_project_imports(project_root: &Path) -> Result<HashSet<String>> {
    let lean_files = find_lean_files(project_root)?;
    let mut all_imports = HashSet::new();

    for file in &lean_files {
        let imports = parse_imports(file)?;
        all_imports.extend(imports);
    }

    Ok(all_imports)
}

/// Compute transitive closure of dependencies
pub fn compute_closure(
    roots: &HashSet<String>,
    dependency_graph: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    let mut closure = HashSet::new();
    let mut queue = VecDeque::new();

    // Initialize queue with root modules
    for root in roots {
        queue.push_back(root.clone());
    }

    while let Some(module) = queue.pop_front() {
        if closure.contains(&module) {
            continue;
        }

        closure.insert(module.clone());

        // Add dependencies to queue
        if let Some(deps) = dependency_graph.get(&module) {
            for dep in deps {
                if !closure.contains(dep) {
                    queue.push_back(dep.clone());
                }
            }
        }
    }

    closure
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_parsing() {
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("Test.lean");

        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "import Mathlib.Algebra.Group.Basic").unwrap();
        writeln!(file, "import Data.List.Pairwise").unwrap();

        writeln!(file, "def foo := 1").unwrap();

        let imports = parse_imports(&file_path).unwrap();
        assert_eq!(imports.len(), 2);
        assert!(imports.contains(&"Mathlib.Algebra.Group.Basic".to_string()));
        assert!(imports.contains(&"Mathlib.Data.List.Pairwise".to_string()));
    }

    #[test]
    fn test_parse_imports_skips_init_modules() {
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("Example.lean");

        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "import Init.Meta").unwrap();
        writeln!(file, "import Algebra.Group.Basic").unwrap();

        let imports = parse_imports(&file_path).unwrap();
        assert_eq!(imports.len(), 1);
        assert!(imports.contains(&"Mathlib.Algebra.Group.Basic".to_string()));
    }

    #[test]
    fn test_find_lean_files_skips_generated_directories() {
        use std::io::Write;
        let temp_dir = tempfile::tempdir().unwrap();
        let root = temp_dir.path();

        let src_dir = root.join("src");
        let lake_dir = root.join(".lake");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&lake_dir).unwrap();

        let valid_file = src_dir.join("Allowed.lean");
        let ignored_file = lake_dir.join("Ignored.lean");
        fs::File::create(&valid_file)
            .unwrap()
            .write_all(b"def foo := 1")
            .unwrap();
        fs::File::create(&ignored_file)
            .unwrap()
            .write_all(b"def bar := 2")
            .unwrap();

        let files = find_lean_files(root).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0], valid_file);
    }

    #[test]
    fn test_closure_computation() {
        let mut graph = HashMap::new();
        graph.insert("A".to_string(), vec!["B".to_string(), "C".to_string()]);
        graph.insert("B".to_string(), vec!["D".to_string()]);
        graph.insert("C".to_string(), vec!["D".to_string()]);
        graph.insert("D".to_string(), vec![]);

        let mut roots = HashSet::new();
        roots.insert("A".to_string());

        let closure = compute_closure(&roots, &graph);

        assert_eq!(closure.len(), 4);
        assert!(closure.contains("A"));
        assert!(closure.contains("B"));
        assert!(closure.contains("C"));
        assert!(closure.contains("D"));
    }
}
