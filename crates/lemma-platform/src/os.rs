//! Operating system detection for Lean toolchains

use std::fmt;
use std::ops::Deref;
use std::str::FromStr;
use target_lexicon::OperatingSystem;

/// Operating system
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct Os(OperatingSystem);

impl Os {
    /// Create a new OS
    pub fn new(os: OperatingSystem) -> Self {
        Self(os)
    }

    /// Detect the current OS from the environment
    pub fn from_env() -> Self {
        Self(target_lexicon::HOST.operating_system)
    }

    /// Check if this is Windows
    pub fn is_windows(&self) -> bool {
        matches!(self.0, OperatingSystem::Windows)
    }

    /// Check if this is macOS
    pub fn is_macos(&self) -> bool {
        matches!(self.0, OperatingSystem::Darwin)
    }

    /// Check if this is Linux
    pub fn is_linux(&self) -> bool {
        matches!(self.0, OperatingSystem::Linux)
    }

    /// Get the underlying target_lexicon OS
    pub fn inner(&self) -> OperatingSystem {
        self.0
    }
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            // Normalize Darwin to "macos" for consistency
            OperatingSystem::Darwin => write!(f, "macos"),
            inner => write!(f, "{inner}"),
        }
    }
}

impl FromStr for Os {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let os = match s {
            // Allow "macos" as an alias for Darwin
            "macos" => OperatingSystem::Darwin,
            // Parse other OS names using target_lexicon
            _ => OperatingSystem::from_str(s).map_err(|()| crate::Error::UnknownOs(s.to_string()))?,
        };

        if matches!(os, OperatingSystem::Unknown) {
            return Err(crate::Error::UnknownOs(s.to_string()));
        }

        Ok(Self(os))
    }
}

impl Deref for Os {
    type Target = OperatingSystem;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_from_env() {
        let os = Os::from_env();
        // Should detect a valid OS
        assert!(!matches!(os.0, OperatingSystem::Unknown));
    }

    #[test]
    fn test_os_display() {
        assert_eq!(Os::new(OperatingSystem::Linux).to_string(), "linux");
        assert_eq!(Os::new(OperatingSystem::Darwin).to_string(), "macos");
        assert_eq!(Os::new(OperatingSystem::Windows).to_string(), "windows");
    }

    #[test]
    fn test_os_from_str() {
        assert!(Os::from_str("linux").is_ok());
        assert!(Os::from_str("macos").is_ok());
        assert!(Os::from_str("windows").is_ok());
        assert!(Os::from_str("darwin").is_ok());
        assert!(Os::from_str("invalid_os").is_err());
    }

    #[test]
    fn test_os_checks() {
        let linux = Os::from_str("linux").unwrap();
        assert!(linux.is_linux());
        assert!(!linux.is_macos());
        assert!(!linux.is_windows());

        let macos = Os::from_str("macos").unwrap();
        assert!(macos.is_macos());
        assert!(!macos.is_linux());
        assert!(!macos.is_windows());
    }
}
