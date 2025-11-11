//! Architecture detection for Lean toolchains

use std::fmt;
use std::str::FromStr;
use target_lexicon::Architecture;

/// Processor architecture
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct Arch(Architecture);

impl Arch {
    /// Create a new architecture
    pub fn new(arch: Architecture) -> Self {
        Self(arch)
    }

    /// Detect the current architecture from the environment
    pub fn from_env() -> Self {
        Self(target_lexicon::HOST.architecture)
    }

    /// Get the underlying target_lexicon architecture
    pub fn inner(&self) -> Architecture {
        self.0
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            // Map i686 to "x86" for consistency with Lean releases
            Architecture::X86_32(target_lexicon::X86_32Architecture::I686) => write!(f, "x86"),
            // aarch64 should include the suffix
            Architecture::Aarch64(_) => write!(f, "aarch64"),
            inner => write!(f, "{inner}"),
        }
    }
}

impl FromStr for Arch {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let arch = match s {
            // Allow "x86" as shorthand for i686
            "x86" => Architecture::X86_32(target_lexicon::X86_32Architecture::I686),
            // Parse other architectures using target_lexicon
            _ => {
                Architecture::from_str(s).map_err(|()| crate::Error::UnknownArch(s.to_string()))?
            }
        };

        if matches!(arch, Architecture::Unknown) {
            return Err(crate::Error::UnknownArch(s.to_string()));
        }

        Ok(Self(arch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_from_env() {
        let arch = Arch::from_env();
        // Should detect a valid architecture
        assert!(!matches!(arch.0, Architecture::Unknown));
    }

    #[test]
    fn test_arch_display() {
        assert_eq!(Arch::new(Architecture::X86_64).to_string(), "x86_64");
        assert_eq!(
            Arch::new(Architecture::X86_32(
                target_lexicon::X86_32Architecture::I686
            ))
            .to_string(),
            "x86"
        );
        assert_eq!(
            Arch::new(Architecture::Aarch64(
                target_lexicon::Aarch64Architecture::Aarch64
            ))
            .to_string(),
            "aarch64"
        );
    }

    #[test]
    fn test_arch_from_str() {
        assert!(Arch::from_str("x86_64").is_ok());
        assert!(Arch::from_str("x86").is_ok());
        assert!(Arch::from_str("aarch64").is_ok());
        assert!(Arch::from_str("invalid_arch").is_err());
    }
}
