//! Parser for lakefile.toml

use crate::error::{Error, Result};
use crate::types::Lakefile;
use crate::validate::validate_lakefile;

/// Parse a lakefile.toml string into a Lakefile struct
///
/// This function parses the TOML content and validates the result
/// to ensure it meets Lake's requirements.
///
/// # Example
///
/// ```rust
/// use lemma_lakefile::parse_toml;
///
/// let content = r#"
/// name = "MyProject"
/// version = "0.1.0"
/// "#;
///
/// let lakefile = parse_toml(content).unwrap();
/// assert_eq!(lakefile.name, "MyProject");
/// ```
pub fn parse_toml(content: &str) -> Result<Lakefile> {
    // Parse TOML
    let lakefile: Lakefile = toml::from_str(content)?;

    // Validate the parsed lakefile
    validate_lakefile(&lakefile)?;

    Ok(lakefile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal() {
        let content = r#"
name = "test"
        "#;

        let lakefile = parse_toml(content).unwrap();
        assert_eq!(lakefile.name, "test");
        assert_eq!(lakefile.version, "0.1.0");
    }

    #[test]
    fn test_parse_full() {
        let content = r#"
name = "MyProject"
version = "0.2.0"
author = "John Doe"
description = "A test project"
license = "MIT"
leanVersion = "v4.24.0"

[[lib]]
name = "MyLib"
root = "MyLib.lean"

[[exe]]
name = "myexe"
root = "Main.lean"

[[dependencies]]
name = "std"
git = "https://github.com/leanprover/std4"
rev = "main"
        "#;

        let lakefile = parse_toml(content).unwrap();
        assert_eq!(lakefile.name, "MyProject");
        assert_eq!(lakefile.version, "0.2.0");
        assert_eq!(lakefile.author, Some("John Doe".to_string()));
        assert_eq!(lakefile.libraries.len(), 1);
        assert_eq!(lakefile.executables.len(), 1);
        assert_eq!(lakefile.dependencies.len(), 1);
    }

    #[test]
    fn test_parse_missing_name() {
        let content = r#"
version = "0.1.0"
        "#;

        let result = parse_toml(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_toml() {
        let content = r#"
name = "test
version = 0.1.0
        "#;

        let result = parse_toml(content);
        assert!(result.is_err());
    }
}
