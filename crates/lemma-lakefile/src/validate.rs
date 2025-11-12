//! Validation logic for lakefiles

use crate::error::{Error, Result};
use crate::types::{ExecutableTarget, Lakefile, LibraryTarget};
use std::collections::HashSet;

/// Validate a lakefile for correctness
///
/// This checks:
/// - Required fields are present
/// - No duplicate target names
/// - Valid version strings
/// - Sensible field values
pub fn validate_lakefile(lakefile: &Lakefile) -> Result<()> {
    // Check required fields
    if lakefile.name.is_empty() {
        return Err(Error::MissingField("name".to_string()));
    }

    // Validate package name (alphanumeric, hyphens, underscores)
    if !is_valid_package_name(&lakefile.name) {
        return Err(Error::InvalidField(
            "name".to_string(),
            format!(
                "'{}' is not a valid package name. Use alphanumeric characters, hyphens, or underscores",
                lakefile.name
            ),
        ));
    }

    // Validate version string if present
    if !lakefile.version.is_empty() {
        semver::Version::parse(&lakefile.version).map_err(|_| {
            Error::InvalidField(
                "version".to_string(),
                format!("'{}' is not a valid semantic version", lakefile.version),
            )
        })?;
    }

    // Check for duplicate target names
    check_duplicate_targets(lakefile)?;

    // Validate library targets
    for lib in &lakefile.libraries {
        validate_library_target(lib)?;
    }

    // Validate executable targets
    for exe in &lakefile.executables {
        validate_executable_target(exe)?;
    }

    Ok(())
}

/// Check if package name is valid
fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

/// Check for duplicate target names across all target types
fn check_duplicate_targets(lakefile: &Lakefile) -> Result<()> {
    let mut seen = HashSet::new();

    // Check library targets
    for lib in &lakefile.libraries {
        if !seen.insert(&lib.name) {
            return Err(Error::DuplicateTarget(lib.name.clone()));
        }
    }

    // Check executable targets
    for exe in &lakefile.executables {
        if !seen.insert(&exe.name) {
            return Err(Error::DuplicateTarget(exe.name.clone()));
        }
    }

    Ok(())
}

/// Validate a library target
fn validate_library_target(lib: &LibraryTarget) -> Result<()> {
    if lib.name.is_empty() {
        return Err(Error::InvalidField(
            "lib.name".to_string(),
            "library name cannot be empty".to_string(),
        ));
    }

    if !is_valid_target_name(&lib.name) {
        return Err(Error::InvalidField(
            "lib.name".to_string(),
            format!(
                "'{}' is not a valid target name. Use alphanumeric characters, hyphens, or underscores",
                lib.name
            ),
        ));
    }

    Ok(())
}

/// Validate an executable target
fn validate_executable_target(exe: &ExecutableTarget) -> Result<()> {
    if exe.name.is_empty() {
        return Err(Error::InvalidField(
            "exe.name".to_string(),
            "executable name cannot be empty".to_string(),
        ));
    }

    if !is_valid_target_name(&exe.name) {
        return Err(Error::InvalidField(
            "exe.name".to_string(),
            format!(
                "'{}' is not a valid target name. Use alphanumeric characters, hyphens, or underscores",
                exe.name
            ),
        ));
    }

    // Validate root module if specified (it's optional and defaults to name)
    if let Some(ref root) = exe.root {
        if root.is_empty() {
            return Err(Error::InvalidField(
                "exe.root".to_string(),
                "root module name cannot be empty if specified".to_string(),
            ));
        }
    }

    Ok(())
}

/// Check if target name is valid
fn is_valid_target_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_toml;

    #[test]
    fn test_valid_lakefile() {
        let content = r#"
name = "test"
version = "0.1.0"
        "#;

        let lakefile = parse_toml(content).unwrap();
        assert!(validate_lakefile(&lakefile).is_ok());
    }

    #[test]
    fn test_invalid_package_name() {
        let content = r#"
name = "test@invalid"
        "#;

        let result = parse_toml(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_version() {
        let content = r#"
name = "test"
version = "not-a-version"
        "#;

        let result = parse_toml(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_targets() {
        let content = r#"
name = "test"

[[lean_lib]]
name = "MyLib"

[[lean_exe]]
name = "MyLib"
root = "Main"
        "#;

        let result = parse_toml(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_exe_with_empty_root() {
        // Empty root string should fail validation
        let content = r#"
name = "test"

[[lean_exe]]
name = "myexe"
root = ""
        "#;

        let result = parse_toml(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_exe_without_root() {
        // Missing root is OK - it defaults to name
        let content = r#"
name = "test"

[[lean_exe]]
name = "myexe"
        "#;

        let result = parse_toml(content);
        assert!(result.is_ok());
        let lakefile = result.unwrap();
        assert_eq!(lakefile.executables[0].root, None);
    }
}
