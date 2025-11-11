//! Settings resolution for Lemma
//!
//! This module converts CLI arguments into resolved settings by merging:
//! 1. Command-line arguments (highest priority)
//! 2. Environment variables
//! 3. Configuration files
//! 4. Built-in defaults (lowest priority)

use std::io::IsTerminal;
use std::path::PathBuf;

use anyhow::{Context, Result};
use lemma_static::EnvVars;

use crate::config::{self, ColorChoice};
/// CLI arguments needed for settings resolution
///
/// This is a simplified version of the full CLI args structure,
/// containing only what's needed for settings resolution.
#[derive(Debug, Clone, Default)]
pub struct CliArgs {
    pub quiet: u8,
    pub verbose: u8,
    pub no_color: bool,
    pub color: Option<ColorChoice>,
}

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

    /// Network request timeout in seconds
    pub network_timeout: u64,

    /// Number of network retries
    pub network_retries: u32,

    /// HTTP proxy URL
    pub http_proxy: Option<String>,

    /// HTTPS proxy URL
    pub https_proxy: Option<String>,
}

impl GlobalSettings {
    /// Resolve global settings from CLI arguments.
    ///
    /// Priority order:
    /// 1. CLI flags (highest priority)
    /// 2. Environment variables
    /// 3. Config file
    /// 4. Defaults (lowest priority)
    pub fn resolve(args: &CliArgs) -> Result<Self> {
        // Load unified configuration (includes both state and preferences)
        let config = config::Config::load().unwrap_or_default();

        // Resolve each setting with proper precedence
        let verbose = resolve_verbose(args, &config);
        let quiet = resolve_quiet(args, &config);
        let color = resolve_color_choice(args, &config)?;
        let lemma_home = resolve_lemma_home(&config)?;
        let network_timeout = resolve_network_timeout(&config);
        let network_retries = resolve_network_retries(&config);
        let http_proxy = resolve_http_proxy(&config);
        let https_proxy = resolve_https_proxy(&config);

        Ok(Self {
            verbose,
            quiet,
            color,
            lemma_home,
            network_timeout,
            network_retries,
            http_proxy,
            https_proxy,
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
    #[allow(dead_code)]
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
                std::io::stderr().is_terminal()
            }
        }
    }
}

/// Resolve verbose level from CLI args and config.
fn resolve_verbose(args: &CliArgs, config: &config::Config) -> u8 {
    // Priority:
    // 1. CLI flag (--verbose count)
    // 2. Config file
    // 3. Default (0)

    if args.verbose > 0 {
        return args.verbose;
    }

    config.global.as_ref().and_then(|g| g.verbose).unwrap_or(0)
}

/// Resolve quiet level from CLI args and config.
fn resolve_quiet(args: &CliArgs, config: &config::Config) -> u8 {
    // Priority:
    // 1. CLI flag (--quiet count)
    // 2. Config file
    // 3. Default (0)

    if args.quiet > 0 {
        return args.quiet;
    }

    config.global.as_ref().and_then(|g| g.quiet).unwrap_or(0)
}

/// Resolve the color choice from CLI args, environment, and config.
fn resolve_color_choice(args: &CliArgs, config: &config::Config) -> Result<ColorChoice> {
    // Priority:
    // 1. --color flag (if provided)
    // 2. --no-color flag (if provided)
    // 3. Config file
    // 4. Default (Auto)

    if let Some(color) = args.color {
        return Ok(color);
    }

    if args.no_color {
        return Ok(ColorChoice::Never);
    }

    // Check config file
    if let Some(color_config) = config.global.as_ref().and_then(|g| g.color) {
        return Ok(color_config.into());
    }

    // Default: auto-detect terminal support
    Ok(ColorChoice::Auto)
}

/// Resolve the Lemma home directory from environment and config.
fn resolve_lemma_home(config: &config::Config) -> Result<PathBuf> {
    // Priority:
    // 1. LEMMA_HOME environment variable
    // 2. Config file
    // 3. Default: ~/.lemma

    if let Ok(home) = std::env::var(EnvVars::LEMMA_HOME) {
        let path = PathBuf::from(home);
        if !path.is_absolute() {
            anyhow::bail!(
                "{} must be an absolute path, got: {}",
                EnvVars::LEMMA_HOME,
                path.display()
            );
        }
        return Ok(path);
    }

    // Check config file
    if let Some(path) = config.paths.as_ref().and_then(|p| p.home.clone()) {
        if !path.is_absolute() {
            anyhow::bail!(
                "Config file home path must be absolute, got: {}",
                path.display()
            );
        }
        return Ok(path);
    }

    // Default location
    let home = dirs::home_dir().context("Could not determine home directory")?;

    Ok(home.join(".lemma"))
}

/// Resolve network timeout from environment and config.
fn resolve_network_timeout(config: &config::Config) -> u64 {
    // Priority:
    // 1. LEMMA_NETWORK_TIMEOUT environment variable
    // 2. Config file
    // 3. Default (30 seconds)

    if let Ok(timeout) = std::env::var("LEMMA_NETWORK_TIMEOUT") {
        if let Ok(value) = timeout.parse::<u64>() {
            return value;
        }
    }

    config
        .network
        .as_ref()
        .and_then(|n| n.timeout)
        .unwrap_or(30)
}

/// Resolve network retries from environment and config.
fn resolve_network_retries(config: &config::Config) -> u32 {
    // Priority:
    // 1. LEMMA_NETWORK_RETRIES environment variable
    // 2. Config file
    // 3. Default (3 retries)

    if let Ok(retries) = std::env::var("LEMMA_NETWORK_RETRIES") {
        if let Ok(value) = retries.parse::<u32>() {
            return value;
        }
    }

    config
        .network
        .as_ref()
        .and_then(|n| n.retries)
        .unwrap_or(3)
}

/// Resolve HTTP proxy from environment and config.
fn resolve_http_proxy(config: &config::Config) -> Option<String> {
    // Priority:
    // 1. HTTP_PROXY environment variable
    // 2. Config file
    // 3. None

    if let Ok(proxy) = std::env::var("HTTP_PROXY") {
        return Some(proxy);
    }

    config
        .network
        .as_ref()
        .and_then(|n| n.http_proxy.clone())
}

/// Resolve HTTPS proxy from environment and config.
fn resolve_https_proxy(config: &config::Config) -> Option<String> {
    // Priority:
    // 1. HTTPS_PROXY environment variable
    // 2. Config file
    // 3. None

    if let Ok(proxy) = std::env::var("HTTPS_PROXY") {
        return Some(proxy);
    }

    config
        .network
        .as_ref()
        .and_then(|n| n.https_proxy.clone())
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
            network_timeout: 30,
            network_retries: 3,
            http_proxy: None,
            https_proxy: None,
        };
        assert_eq!(settings.log_level(), "info");

        let settings = GlobalSettings {
            verbose: 1,
            quiet: 0,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
            network_timeout: 30,
            network_retries: 3,
            http_proxy: None,
            https_proxy: None,
        };
        assert_eq!(settings.log_level(), "debug");

        let settings = GlobalSettings {
            verbose: 0,
            quiet: 1,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
            network_timeout: 30,
            network_retries: 3,
            http_proxy: None,
            https_proxy: None,
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
            network_timeout: 30,
            network_retries: 3,
            http_proxy: None,
            https_proxy: None,
        };
        assert!(!settings.is_quiet());

        let settings = GlobalSettings {
            verbose: 0,
            quiet: 1,
            color: ColorChoice::Auto,
            lemma_home: PathBuf::from("/tmp/lemma"),
            network_timeout: 30,
            network_retries: 3,
            http_proxy: None,
            https_proxy: None,
        };
        assert!(settings.is_quiet());
    }

    #[test]
    fn test_resolve_color_choice() {
        use crate::config::Config;

        let config = Config::default();

        // Test --color flag
        let args = CliArgs {
            verbose: 0,
            quiet: 0,
            color: Some(ColorChoice::Always),
            no_color: false,
        };
        assert!(matches!(
            resolve_color_choice(&args, &config).unwrap(),
            ColorChoice::Always
        ));

        // Test --no-color flag
        let args = CliArgs {
            verbose: 0,
            quiet: 0,
            color: None,
            no_color: true,
        };
        assert!(matches!(
            resolve_color_choice(&args, &config).unwrap(),
            ColorChoice::Never
        ));

        // Test default
        let args = CliArgs {
            verbose: 0,
            quiet: 0,
            color: None,
            no_color: false,
        };
        assert!(matches!(
            resolve_color_choice(&args, &config).unwrap(),
            ColorChoice::Auto
        ));
    }
}
