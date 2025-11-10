//! Settings resolution for Lemma
//!
//! This module converts CLI arguments into resolved settings by merging:
//! 1. Command-line arguments (highest priority)
//! 2. Environment variables
//! 3. Configuration files (future)
//! 4. Built-in defaults (lowest priority)

use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::cli::{ColorChoice, GlobalArgs};

/// Resolved global settings used throughout the application.
///
/// Unlike `GlobalArgs`, which represents raw CLI input, `GlobalSettings`
/// represents the final, resolved configuration after merging all sources.
#[derive(Debug, Clone)]
pub struct GlobalSettings {
    /// Verbosity level (0 = normal, 1 = debug, 2+ = trace)
    pub verbose: u8,

    /// Quiet level (0 = normal, 1 = warnings only, 2+ = errors only)
    pub quiet: u8,

    /// Color output configuration
    pub color: ColorChoice,

    /// Lemma home directory (where toolchains are stored)
    pub lemma_home: PathBuf,
}

impl GlobalSettings {
    /// Resolve global settings from CLI arguments.
    ///
    /// Priority order:
    /// 1. CLI flags (highest priority)
    /// 2. Environment variables
    /// 3. Config file (future)
    /// 4. Defaults (lowest priority)
    pub fn resolve(args: &GlobalArgs) -> Result<Self> {
        // Resolve color setting
        let color = resolve_color_choice(args)?;

        // Resolve lemma home directory
        let lemma_home = resolve_lemma_home()?;

        Ok(Self {
            verbose: args.verbose,
            quiet: args.quiet,
            color,
            lemma_home,
        })
    }

    /// Get the appropriate log level for tracing.
    ///
    /// Returns a string like "info", "debug", "trace" based on verbosity.
    pub fn log_level(&self) -> &'static str {
        // Quiet takes precedence over verbose
        match (self.verbose, self.quiet) {
            // Normal verbosity
            (0, 0) => "info",
            // Verbose levels
            (1, 0) => "debug",
            (2.., 0) => "trace",
            // Quiet levels
            (0, 1) => "warn",
            (0, 2..) => "error",
            // If both are set (shouldn't happen due to conflicts_with), default to info
            _ => "info",
        }
    }

    /// Check if we should suppress progress bars and interactive output.
    pub fn is_quiet(&self) -> bool {
        self.quiet > 0
    }

    /// Check if we're in verbose mode.
    pub fn is_verbose(&self) -> bool {
        self.verbose > 0
    }

    /// Get verbosity level for detailed operations.
    pub fn verbosity_level(&self) -> u8 {
        self.verbose
    }

    /// Check if color output should be enabled.
    ///
    /// Takes into account the color choice and whether we're in a TTY.
    pub fn use_colors(&self) -> bool {
        match self.color {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            ColorChoice::Auto => {
                // Auto-detect: use colors if stderr is a TTY
                atty::is(atty::Stream::Stderr)
            }
        }
    }
}

/// Resolve the color choice from CLI args and environment.
fn resolve_color_choice(args: &GlobalArgs) -> Result<ColorChoice> {
    // Priority:
    // 1. --color flag (if provided)
    // 2. --no-color flag (if provided)
    // 3. Default (Auto)

    if let Some(color) = args.color {
        return Ok(color);
    }

    if args.no_color {
        return Ok(ColorChoice::Never);
    }

    // Default: auto-detect terminal support
    Ok(ColorChoice::Auto)
}

/// Resolve the Lemma home directory.
fn resolve_lemma_home() -> Result<PathBuf> {
    // Priority:
    // 1. LEMMA_HOME environment variable
    // 2. Default: ~/.lemma

    if let Ok(home) = std::env::var("LEMMA_HOME") {
        let path = PathBuf::from(home);
        if !path.is_absolute() {
            anyhow::bail!(
                "LEMMA_HOME must be an absolute path, got: {}",
                path.display()
            );
        }
        return Ok(path);
    }

    // Default location
    let home = dirs::home_dir().context("Could not determine home directory")?;

    Ok(home.join(".lemma"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level() {
        let settings = GlobalSettings {
            verbose: 0,
            quiet: 0,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
        };
        assert_eq!(settings.log_level(), "info");

        let settings = GlobalSettings {
            verbose: 1,
            quiet: 0,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
        };
        assert_eq!(settings.log_level(), "debug");

        let settings = GlobalSettings {
            verbose: 0,
            quiet: 1,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
        };
        assert_eq!(settings.log_level(), "warn");
    }

    #[test]
    fn test_is_quiet() {
        let settings = GlobalSettings {
            verbose: 0,
            quiet: 0,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
        };
        assert!(!settings.is_quiet());

        let settings = GlobalSettings {
            verbose: 0,
            quiet: 1,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
        };
        assert!(settings.is_quiet());
    }

    #[test]
    fn test_resolve_color_choice() {
        // Test --color flag
        let args = GlobalArgs {
            verbose: 0,
            quiet: 0,
            color: Some(ColorChoice::Always),
            no_color: false,
        };
        assert!(matches!(
            resolve_color_choice(&args).unwrap(),
            ColorChoice::Always
        ));

        // Test --no-color flag
        let args = GlobalArgs {
            verbose: 0,
            quiet: 0,
            color: None,
            no_color: true,
        };
        assert!(matches!(
            resolve_color_choice(&args).unwrap(),
            ColorChoice::Never
        ));

        // Test default
        let args = GlobalArgs {
            verbose: 0,
            quiet: 0,
            color: None,
            no_color: false,
        };
        assert!(matches!(
            resolve_color_choice(&args).unwrap(),
            ColorChoice::Auto
        ));
    }
}
