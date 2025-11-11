//! Data structures representing Lake project configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// The main lakefile configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Lakefile {
    /// Package name (required)
    pub name: String,

    /// Package version (optional, default: "0.1.0")
    #[serde(default = "default_version")]
    pub version: String,

    /// Package author (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Package description (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Package license (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Repository URL (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    /// Lean version requirement (optional)
    /// Example: "v4.24.0", ">=4.24.0", etc.
    #[serde(rename = "leanVersion", skip_serializing_if = "Option::is_none")]
    pub lean_version: Option<String>,

    /// Source directory (default: ".")
    #[serde(rename = "srcDir", default = "default_src_dir")]
    pub src_dir: PathBuf,

    /// Build directory (default: ".lake/build")
    #[serde(rename = "buildDir", default = "default_build_dir")]
    pub build_dir: PathBuf,

    /// Package directory for dependencies (default: ".lake/packages")
    #[serde(rename = "packagesDir", default = "default_packages_dir")]
    pub packages_dir: PathBuf,

    /// Library targets
    #[serde(rename = "lib", skip_serializing_if = "Vec::is_empty", default)]
    pub libraries: Vec<LibraryTarget>,

    /// Executable targets
    #[serde(rename = "exe", skip_serializing_if = "Vec::is_empty", default)]
    pub executables: Vec<ExecutableTarget>,

    /// Dependencies
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub dependencies: Vec<Dependency>,

    /// Lean library path additions (optional)
    #[serde(rename = "leanLibs", skip_serializing_if = "Vec::is_empty", default)]
    pub lean_libs: Vec<PathBuf>,

    /// Additional lean compiler flags (optional)
    #[serde(rename = "moreLeanArgs", skip_serializing_if = "Vec::is_empty", default)]
    pub more_lean_args: Vec<String>,

    /// Additional link flags for executables (optional)
    #[serde(rename = "moreLinkArgs", skip_serializing_if = "Vec::is_empty", default)]
    pub more_link_args: Vec<String>,

    /// Default targets to build (optional)
    /// If empty, builds all libraries and executables
    #[serde(rename = "defaultTargets", skip_serializing_if = "Vec::is_empty", default)]
    pub default_targets: Vec<String>,

    /// Custom build options (optional, for extensibility)
    #[serde(flatten)]
    pub custom: HashMap<String, toml::Value>,
}

/// A library target definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryTarget {
    /// Library name (defaults to package name)
    pub name: String,

    /// Root module for this library (optional)
    /// If not specified, uses `{name}.lean` or finds it automatically
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<PathBuf>,

    /// Glob patterns for source files (optional)
    /// Example: ["**/*.lean"]
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub globs: Vec<String>,

    /// Source directory relative to project root (optional)
    #[serde(rename = "srcDir", skip_serializing_if = "Option::is_none")]
    pub src_dir: Option<PathBuf>,

    /// Additional lean compiler flags for this library
    #[serde(rename = "moreLeanArgs", skip_serializing_if = "Vec::is_empty", default)]
    pub more_lean_args: Vec<String>,

    /// Whether to export C bindings (default: true)
    #[serde(rename = "nativeFacets", default = "default_true")]
    pub native_facets: bool,

    /// Whether to build as static library (default: true)
    #[serde(default = "default_true")]
    pub static_lib: bool,
}

/// An executable target definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutableTarget {
    /// Executable name (required)
    pub name: String,

    /// Root module for this executable (required)
    /// Example: "Main.lean" or "src/Main.lean"
    pub root: PathBuf,

    /// Source directory relative to project root (optional)
    #[serde(rename = "srcDir", skip_serializing_if = "Option::is_none")]
    pub src_dir: Option<PathBuf>,

    /// Libraries to link against (optional)
    /// Can reference local library targets or external libraries
    #[serde(rename = "supportInterpreter", default = "default_false")]
    pub support_interpreter: bool,

    /// Additional lean compiler flags for this executable
    #[serde(rename = "moreLeanArgs", skip_serializing_if = "Vec::is_empty", default)]
    pub more_lean_args: Vec<String>,

    /// Additional link flags for this executable
    #[serde(rename = "moreLinkArgs", skip_serializing_if = "Vec::is_empty", default)]
    pub more_link_args: Vec<String>,
}

/// A dependency specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    /// Dependency name (required)
    pub name: String,

    /// Dependency source
    #[serde(flatten)]
    pub source: DependencySource,

    /// Scope of the dependency (optional)
    /// Not used yet, reserved for future use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Source location for a dependency
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", untagged)]
pub enum DependencySource {
    /// Git repository
    Git {
        /// Git URL
        git: String,
        /// Git revision (branch, tag, or commit)
        #[serde(skip_serializing_if = "Option::is_none")]
        rev: Option<String>,
    },
    /// Local path
    Path {
        /// File system path
        path: PathBuf,
    },
    /// Lake registry (future)
    Registry {
        /// Registry name
        registry: String,
        /// Version requirement
        version: String,
    },
}

// Default value functions

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_src_dir() -> PathBuf {
    PathBuf::from(".")
}

fn default_build_dir() -> PathBuf {
    PathBuf::from(".lake/build")
}

fn default_packages_dir() -> PathBuf {
    PathBuf::from(".lake/packages")
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_lakefile() {
        let toml = r#"
name = "test"
        "#;

        let lakefile: Lakefile = toml::from_str(toml).unwrap();
        assert_eq!(lakefile.name, "test");
        assert_eq!(lakefile.version, "0.1.0");
        assert_eq!(lakefile.src_dir, PathBuf::from("."));
    }

    #[test]
    fn test_library_target() {
        let toml = r#"
name = "test"

[[lib]]
name = "MyLib"
        "#;

        let lakefile: Lakefile = toml::from_str(toml).unwrap();
        assert_eq!(lakefile.libraries.len(), 1);
        assert_eq!(lakefile.libraries[0].name, "MyLib");
    }

    #[test]
    fn test_executable_target() {
        let toml = r#"
name = "test"

[[exe]]
name = "myexe"
root = "Main.lean"
        "#;

        let lakefile: Lakefile = toml::from_str(toml).unwrap();
        assert_eq!(lakefile.executables.len(), 1);
        assert_eq!(lakefile.executables[0].name, "myexe");
    }

    #[test]
    fn test_git_dependency() {
        let toml = r#"
name = "test"

[[dependencies]]
name = "std"
git = "https://github.com/leanprover/std4"
rev = "main"
        "#;

        let lakefile: Lakefile = toml::from_str(toml).unwrap();
        assert_eq!(lakefile.dependencies.len(), 1);
        assert_eq!(lakefile.dependencies[0].name, "std");

        match &lakefile.dependencies[0].source {
            DependencySource::Git { git, rev } => {
                assert_eq!(git, "https://github.com/leanprover/std4");
                assert_eq!(rev.as_deref(), Some("main"));
            }
            _ => panic!("Expected git dependency"),
        }
    }
}
