//! Platform detection for Lean toolchains
//!
//! This crate provides platform identification for operating system and architecture,
//! used for determining which Lean toolchain to download and install.

use std::fmt;
use std::str::FromStr;
use thiserror::Error;

pub use crate::arch::Arch;
pub use crate::os::Os;

mod arch;
mod os;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unknown operating system: {0}")]
    UnknownOs(String),
    #[error("Unknown architecture: {0}")]
    UnknownArch(String),
    #[error("Invalid platform format: {0}")]
    InvalidPlatformFormat(String),
}

/// A platform identifier combining operating system and architecture
///
/// Format: `{os}-{arch}` (e.g., "linux-x86_64", "macos-aarch64")
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    /// Create a new platform with the given OS and architecture
    pub fn new(os: Os, arch: Arch) -> Self {
        Self { os, arch }
    }

    /// Create a platform from string parts (os, arch)
    pub fn from_parts(os: &str, arch: &str) -> Result<Self, Error> {
        Ok(Self {
            os: Os::from_str(os)?,
            arch: Arch::from_str(arch)?,
        })
    }

    /// Detect the platform from the current environment
    pub fn from_env() -> Self {
        Self {
            os: Os::from_env(),
            arch: Arch::from_env(),
        }
    }

    /// Convert to a platform string suitable for directory names
    ///
    /// Returns strings like: "linux", "linux_aarch64", "macos", "macos_aarch64", "windows"
    ///
    /// Note: x86_64 (the default) has no suffix, while other architectures include an underscore separator.
    pub fn as_directory_suffix(&self) -> String {
        match self.arch.inner() {
            // x86_64 is the default, no suffix needed
            target_lexicon::Architecture::X86_64 => self.os.to_string(),
            // All other architectures include an underscore separator
            _ => format!("{}_{}", self.os, self.arch),
        }
    }

    /// Parse from a directory suffix string
    ///
    /// Handles formats like: "linux", "linux_aarch64", "macos", "darwin_aarch64", "windows"
    pub fn from_directory_suffix(s: &str) -> Result<Self, Error> {
        // List of known architectures that might appear as suffixes
        const ARCH_SUFFIXES: &[&str] = &["aarch64", "x86", "arm", "armv7"];

        // Try to find an architecture suffix
        for arch_str in ARCH_SUFFIXES {
            let suffix = format!("_{}", arch_str);
            if let Some(os_str) = s.strip_suffix(&suffix) {
                return Self::from_parts(os_str, arch_str);
            }
        }

        // No architecture suffix found, assume x86_64
        Self::from_parts(s, "x86_64")
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.os, self.arch)
    }
}

impl FromStr for Platform {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('-').collect();

        if parts.len() != 2 {
            return Err(Error::InvalidPlatformFormat(format!(
                "expected exactly 2 parts separated by '-', got {}",
                parts.len()
            )));
        }

        Self::from_parts(parts[0], parts[1])
    }
}

/// Get the current platform identifier as a string
///
/// Returns platform strings like: "linux", "linux_aarch64", "macos", "macos_aarch64", "windows"
///
/// This is a convenience function equivalent to `Platform::from_env().as_directory_suffix()`
pub fn current_platform() -> String {
    Platform::from_env().as_directory_suffix()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_from_env() {
        let platform = Platform::from_env();
        // Should successfully detect the current platform
        assert!(!platform.to_string().is_empty());
    }

    #[test]
    fn test_platform_display() {
        let platform = Platform::from_parts("linux", "x86_64").unwrap();
        assert_eq!(platform.to_string(), "linux-x86_64");

        let platform = Platform::from_parts("macos", "aarch64").unwrap();
        assert_eq!(platform.to_string(), "macos-aarch64");
    }

    #[test]
    fn test_platform_from_str() {
        let platform = Platform::from_str("linux-x86_64").unwrap();
        assert_eq!(platform.os.to_string(), "linux");
        assert_eq!(platform.arch.to_string(), "x86_64");

        let platform = Platform::from_str("macos-aarch64").unwrap();
        assert_eq!(platform.os.to_string(), "macos");
        assert_eq!(platform.arch.to_string(), "aarch64");

        // Test error cases
        assert!(Platform::from_str("invalid").is_err());
        assert!(Platform::from_str("linux-x86-64-extra").is_err());
    }

    #[test]
    fn test_platform_from_parts() {
        let platform = Platform::from_parts("linux", "x86_64").unwrap();
        assert_eq!(platform.os.to_string(), "linux");
        assert_eq!(platform.arch.to_string(), "x86_64");

        // Test error cases
        assert!(Platform::from_parts("invalid_os", "x86_64").is_err());
        assert!(Platform::from_parts("linux", "invalid_arch").is_err());
    }

    #[test]
    fn test_as_directory_suffix() {
        let platform = Platform::from_parts("linux", "x86_64").unwrap();
        assert_eq!(platform.as_directory_suffix(), "linux");

        let platform = Platform::from_parts("linux", "aarch64").unwrap();
        assert_eq!(platform.as_directory_suffix(), "linux_aarch64");

        let platform = Platform::from_parts("macos", "x86_64").unwrap();
        assert_eq!(platform.as_directory_suffix(), "macos");

        let platform = Platform::from_parts("macos", "aarch64").unwrap();
        assert_eq!(platform.as_directory_suffix(), "macos_aarch64");

        let platform = Platform::from_parts("windows", "x86_64").unwrap();
        assert_eq!(platform.as_directory_suffix(), "windows");
    }

    #[test]
    fn test_from_directory_suffix() {
        // x86_64 (no suffix)
        let platform = Platform::from_directory_suffix("linux").unwrap();
        assert_eq!(platform.os.to_string(), "linux");
        assert_eq!(platform.arch.to_string(), "x86_64");

        // aarch64
        let platform = Platform::from_directory_suffix("linux_aarch64").unwrap();
        assert_eq!(platform.os.to_string(), "linux");
        assert_eq!(platform.arch.to_string(), "aarch64");

        // macOS variations
        let platform = Platform::from_directory_suffix("macos_aarch64").unwrap();
        assert_eq!(platform.os.to_string(), "macos");
        assert_eq!(platform.arch.to_string(), "aarch64");

        let platform = Platform::from_directory_suffix("darwin_aarch64").unwrap();
        assert_eq!(platform.os.to_string(), "macos");
        assert_eq!(platform.arch.to_string(), "aarch64");

        // windows
        let platform = Platform::from_directory_suffix("windows").unwrap();
        assert_eq!(platform.os.to_string(), "windows");
        assert_eq!(platform.arch.to_string(), "x86_64");
    }

    #[test]
    fn test_roundtrip_directory_suffix() {
        // Test that converting to/from directory suffix is consistent
        let original = Platform::from_parts("linux", "x86_64").unwrap();
        let suffix = original.as_directory_suffix();
        let parsed = Platform::from_directory_suffix(&suffix).unwrap();
        assert_eq!(original, parsed);

        let original = Platform::from_parts("macos", "aarch64").unwrap();
        let suffix = original.as_directory_suffix();
        let parsed = Platform::from_directory_suffix(&suffix).unwrap();
        assert_eq!(original, parsed);
    }
}
