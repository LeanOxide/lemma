//! Init command - Initialize a new Lean project

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use lemma_config::GlobalSettings;
use lemma_output::Printer;

/// Project type to create
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectKind {
    /// Standard project with library and executable (Lake's 'std')
    Standard,
    /// Executable-only project (Lake's 'exe')
    Executable,
    /// Library-only project (Lake's 'lib')
    Library,
}

impl Default for ProjectKind {
    fn default() -> Self {
        Self::Standard
    }
}

/// Execute the init command
pub fn execute(
    name: Option<String>,
    path: Option<String>,
    bare: bool,
    std: bool,
    exe: bool,
    lib: bool,
    no_readme: bool,
    _settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    // Determine project kind
    let kind = match (std, exe, lib) {
        (true, false, false) => ProjectKind::Standard,
        (false, true, false) => ProjectKind::Executable,
        (false, false, true) => ProjectKind::Library,
        (false, false, false) => ProjectKind::default(),
        _ => unreachable!("std, exe, and lib are mutually exclusive"),
    };

    // Determine project path and name
    let (project_path, project_name) = match (name, path) {
        // Name provided: create a new directory with that name
        (Some(name), None) => {
            let current_dir = std::env::current_dir().context("Failed to get current directory")?;
            let project_path = current_dir.join(&name);
            (project_path, name)
        }
        // Path provided: use that path, derive name from it or use provided name
        (name, Some(p)) => {
            let project_path = PathBuf::from(p);
            let project_name = determine_project_name(name, &project_path)?;
            (project_path, project_name)
        }
        // Neither provided: use current directory
        (None, None) => {
            let project_path =
                std::env::current_dir().context("Failed to get current directory")?;
            let project_name = determine_project_name(None, &project_path)?;
            (project_path, project_name)
        }
    };

    // Validate project name
    validate_project_name(&project_name)?;

    // Check if directory is empty or create it
    ensure_project_directory(&project_path, printer)?;

    // Initialize the project
    init_project(&project_name, &project_path, kind, bare, no_readme, printer)?;

    // Print success message
    printer.success(format!(
        "Initialized {} project '{}' at {}",
        match kind {
            ProjectKind::Standard => "standard",
            ProjectKind::Executable => "executable",
            ProjectKind::Library => "library",
        },
        project_name,
        project_path.display()
    ))?;

    Ok(())
}

/// Determine the project name from the provided name or directory name
fn determine_project_name(name: Option<String>, path: &Path) -> Result<String> {
    if let Some(name) = name {
        return Ok(name);
    }

    // Use directory name as project name
    let dir_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .context("Failed to determine directory name")?;

    // Convert directory name to a valid package name
    // Replace spaces with hyphens and trim
    let candidate = dir_name.trim().replace(' ', "-");

    if candidate.is_empty() {
        anyhow::bail!("Cannot determine project name from directory. Please provide a name.");
    }

    Ok(candidate)
}

/// Validate the project name
fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Project name cannot be empty");
    }

    if name.chars().all(|c| c == '.') {
        anyhow::bail!("Project name cannot consist only of dots");
    }

    if name.contains('/') || name.contains('\\') {
        anyhow::bail!("Project name cannot contain path separators (/ or \\)");
    }

    // Check for reserved names (matching Lake's validation)
    let lower_name = name.to_lowercase();
    if matches!(lower_name.as_str(), "init" | "lean" | "lake" | "main") {
        anyhow::bail!("'{}' is a reserved project name", name);
    }

    Ok(())
}

/// Ensure the project directory exists and is suitable for initialization
fn ensure_project_directory(path: &Path, printer: &Printer) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).context("Failed to create project directory")?;
        return Ok(());
    }

    if !path.is_dir() {
        anyhow::bail!("Path exists but is not a directory: {}", path.display());
    }

    // Check if directory is empty
    let entries = fs::read_dir(path).context("Failed to read directory")?;
    let mut has_entries = false;
    for entry in entries {
        let entry = entry?;
        // Ignore hidden files like .git
        if let Some(name) = entry.file_name().to_str() {
            if !name.starts_with('.') {
                has_entries = true;
                break;
            }
        }
    }

    if has_entries {
        // Check if lakefile already exists
        if path.join("lakefile.toml").exists() || path.join("lakefile.lean").exists() {
            anyhow::bail!("Directory already contains a lakefile. Cannot initialize project here.");
        }

        printer.warning(
            "Directory is not empty. Existing files will not be modified, but new files will be created.",
        )?;
    }

    Ok(())
}

/// Initialize the project structure
fn init_project(
    name: &str,
    path: &Path,
    kind: ProjectKind,
    bare: bool,
    no_readme: bool,
    printer: &Printer,
) -> Result<()> {
    // Initialize git repository first (if not already in one)
    init_git(path, printer)?;

    // Create lakefile.toml
    create_lakefile(name, path, kind)?;

    if bare {
        // Bare mode: only create lakefile.toml
        return Ok(());
    }

    // Create .gitignore
    create_gitignore(path)?;

    // Create README.md unless --no-readme
    if !no_readme {
        create_readme(name, path, kind)?;
    }

    // Create project files based on kind
    match kind {
        ProjectKind::Standard => create_standard_files(name, path)?,
        ProjectKind::Executable => create_executable_files(name, path)?,
        ProjectKind::Library => create_library_files(name, path)?,
    }

    // Try to detect and write lean-toolchain file
    if let Ok(toolchain) = detect_lean_toolchain() {
        create_lean_toolchain_file(path, &toolchain)?;
    }

    Ok(())
}

/// Initialize git repository if not already in one
fn init_git(path: &Path, printer: &Printer) -> Result<()> {
    // Check if already in a git repository
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(path)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            return Ok(());
        }
    }

    // Initialize new git repository
    let status = std::process::Command::new("git")
        .args(["init"])
        .current_dir(path)
        .status();

    match status {
        Ok(status) if status.success() => {}
        _ => {
            printer.warning("Failed to initialize git repository")?;
        }
    }

    Ok(())
}

/// Create lakefile.toml
fn create_lakefile(name: &str, path: &Path, kind: ProjectKind) -> Result<()> {
    let lakefile_path = path.join("lakefile.toml");

    if lakefile_path.exists() {
        anyhow::bail!("lakefile.toml already exists");
    }

    let content = match kind {
        ProjectKind::Standard => generate_std_lakefile(name),
        ProjectKind::Executable => generate_exe_lakefile(name),
        ProjectKind::Library => generate_lib_lakefile(name),
    };

    fs::write(lakefile_path, content).context("Failed to write lakefile.toml")?;

    Ok(())
}

/// Generate lakefile.toml for a standard project (library + executable)
fn generate_std_lakefile(name: &str) -> String {
    let module_name = to_module_name(name);
    format!(
        r#"name = "{name}"
version = "0.1.0"
defaultTargets = ["{name}"]

[[lean_lib]]
name = "{module_name}"

[[lean_exe]]
name = "{name}"
root = "Main"
"#,
        name = name,
        module_name = module_name
    )
}

/// Generate lakefile.toml for an executable-only project
fn generate_exe_lakefile(name: &str) -> String {
    format!(
        r#"name = "{name}"
version = "0.1.0"
defaultTargets = ["{name}"]

[[lean_exe]]
name = "{name}"
root = "Main"
"#,
        name = name
    )
}

/// Generate lakefile.toml for a library-only project
fn generate_lib_lakefile(name: &str) -> String {
    let module_name = to_module_name(name);
    format!(
        r#"name = "{name}"
version = "0.1.0"
defaultTargets = ["{module_name}"]

[[lean_lib]]
name = "{module_name}"
"#,
        name = name,
        module_name = module_name
    )
}

/// Create .gitignore
fn create_gitignore(path: &Path) -> Result<()> {
    let gitignore_path = path.join(".gitignore");

    // If .gitignore already exists, append to it
    let existing_content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path).unwrap_or_default()
    } else {
        String::new()
    };

    // Check if Lake entries are already present
    if existing_content.contains(".lake") {
        return Ok(());
    }

    let mut content = existing_content;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }

    content.push_str(
        r#"
# Lake build artifacts
.lake/
"#,
    );

    fs::write(gitignore_path, content).context("Failed to write .gitignore")?;

    Ok(())
}

/// Create README.md
fn create_readme(name: &str, path: &Path, kind: ProjectKind) -> Result<()> {
    let readme_path = path.join("README.md");

    if readme_path.exists() {
        // Don't overwrite existing README
        return Ok(());
    }

    let content = match kind {
        ProjectKind::Standard => {
            format!(
                r#"# {name}

This is a Lean 4 project with both library and executable components.

## Building

```bash
lake build
```

## Running

```bash
lake exe {name}
```

## Using as a dependency

Add this to your `lakefile.toml`:

```toml
[[require]]
name = "{name}"
```
"#,
                name = name
            )
        }
        ProjectKind::Executable => {
            format!(
                r#"# {name}

This is a Lean 4 executable project.

## Building

```bash
lake build
```

## Running

```bash
lake exe {name}
```
"#,
                name = name
            )
        }
        ProjectKind::Library => {
            format!(
                r#"# {name}

This is a Lean 4 library project.

## Building

```bash
lake build
```

## Using as a dependency

Add this to your `lakefile.toml`:

```toml
[[require]]
name = "{name}"
```
"#,
                name = name
            )
        }
    };

    fs::write(readme_path, content).context("Failed to write README.md")?;

    Ok(())
}

/// Create standard project files (library + executable)
fn create_standard_files(name: &str, path: &Path) -> Result<()> {
    // Create Main.lean
    let main_path = path.join("Main.lean");

    if !main_path.exists() {
        let module_name = to_module_name(name);
        let content = format!(
            r#"import {module_name}

def main : IO Unit :=
  IO.println s!"Hello from {{hello}}!"
"#,
            module_name = module_name
        );
        fs::write(main_path, content).context("Failed to write Main.lean")?;
    }

    // Create library directory and files
    create_library_structure(name, path)?;

    Ok(())
}

/// Create executable-only project files
fn create_executable_files(name: &str, path: &Path) -> Result<()> {
    // Create Main.lean
    let main_path = path.join("Main.lean");

    if !main_path.exists() {
        let content = format!(
            r#"def main : IO Unit :=
  IO.println s!"Hello from {name}!"
"#,
            name = name
        );
        fs::write(main_path, content).context("Failed to write Main.lean")?;
    }

    Ok(())
}

/// Create library-only project files
fn create_library_files(name: &str, path: &Path) -> Result<()> {
    create_library_structure(name, path)
}

/// Create the library directory structure
fn create_library_structure(name: &str, path: &Path) -> Result<()> {
    let module_name = to_module_name(name);
    let lib_dir = path.join(&module_name);

    // Create library directory
    fs::create_dir_all(&lib_dir).context("Failed to create library directory")?;

    // Create <ModuleName>/<ModuleName>.lean (root module)
    let root_module_path = path.join(format!("{}.lean", module_name));
    if !root_module_path.exists() {
        let content = format!(
            r#"-- Main module for {name}
import {module_name}.Basic
"#,
            name = name,
            module_name = module_name
        );
        fs::write(root_module_path, content).context("Failed to write root module")?;
    }

    // Create Basic.lean
    let basic_path = lib_dir.join("Basic.lean");
    if !basic_path.exists() {
        let content = format!(
            r#"-- Basic definitions for {name}

def hello : String := "{name}"
"#,
            name = name
        );
        fs::write(basic_path, content).context("Failed to write Basic.lean")?;
    }

    Ok(())
}

/// Create lean-toolchain file
fn create_lean_toolchain_file(path: &Path, toolchain: &str) -> Result<()> {
    let toolchain_path = path.join("lean-toolchain");

    if toolchain_path.exists() {
        // Don't overwrite existing lean-toolchain
        return Ok(());
    }

    fs::write(toolchain_path, format!("{}\n", toolchain))
        .context("Failed to write lean-toolchain")?;

    Ok(())
}

/// Detect the current Lean toolchain
fn detect_lean_toolchain() -> Result<String> {
    // Try to resolve the active toolchain using lemma's resolution system
    if let Some((toolchain, _source)) = lemma_config::resolve_toolchain_with_source(None)? {
        return Ok(toolchain);
    }

    // If no toolchain is active, use a sensible default
    // This matches Lake's default behavior
    Ok("leanprover/lean4:stable".to_string())
}

/// Convert a package name to a module name (PascalCase)
fn to_module_name(name: &str) -> String {
    // Split on hyphens, underscores, or spaces
    let parts: Vec<&str> = name.split(|c| c == '-' || c == '_' || c == ' ').collect();

    // Convert each part to PascalCase
    parts
        .iter()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_module_name() {
        assert_eq!(to_module_name("myapp"), "Myapp");
        assert_eq!(to_module_name("my-app"), "MyApp");
        assert_eq!(to_module_name("my_app"), "MyApp");
        assert_eq!(to_module_name("my-cool-app"), "MyCoolApp");
    }

    #[test]
    fn test_validate_project_name() {
        assert!(validate_project_name("myapp").is_ok());
        assert!(validate_project_name("my-app").is_ok());
        assert!(validate_project_name("my_app").is_ok());

        assert!(validate_project_name("").is_err());
        assert!(validate_project_name(".").is_err());
        assert!(validate_project_name("..").is_err());
        assert!(validate_project_name("my/app").is_err());
        assert!(validate_project_name("my\\app").is_err());
        assert!(validate_project_name("init").is_err());
        assert!(validate_project_name("lean").is_err());
        assert!(validate_project_name("lake").is_err());
        assert!(validate_project_name("main").is_err());
    }
}
