//! Output abstraction for Lemma
//!
//! This crate provides a unified interface for all output operations in lemma,
//! ensuring consistent formatting, proper quiet/verbose mode handling, and
//! testable output behavior.
//!
//! # Core Types
//!
//! - [`Printer`]: Main abstraction for all output operations
//! - Respects quiet/verbose modes automatically
//! - Handles color output consistently
//! - Provides progress bar support
//!
//! # Examples
//!
//! ```rust
//! use lemma_output::Printer;
//!
//! let printer = Printer::new(false, false, true); // not quiet, not verbose, use colors
//!
//! // Print status messages
//! printer.status("Installing toolchain...").unwrap();
//! printer.success("Installation complete!").unwrap();
//!
//! // Print errors (always shown, even in quiet mode)
//! printer.error("Failed to download").unwrap();
//!
//! // Print hints (only in verbose mode)
//! printer.hint("You can customize this with --config").unwrap();
//! ```
//!
//! # Integration with Settings
//!
//! The [`Printer`] is typically created from [`lemma_config::GlobalSettings`]:
//!
//! ```rust,ignore
//! use lemma_config::GlobalSettings;
//! use lemma_output::Printer;
//!
//! fn my_command(settings: &GlobalSettings) -> Result<(), Box<dyn std::error::Error>> {
//!     let printer = Printer::from_settings(settings);
//!
//!     printer.status("Doing something...")?;
//!     // ... command logic
//!     printer.success("Done!")?;
//!
//!     Ok(())
//! }
//! ```

use colored::*;
use std::io::{self, Write};

pub mod progress;

/// Controls output formatting and verbosity
///
/// The `Printer` abstraction centralizes all output operations, ensuring:
/// - Consistent formatting across all commands
/// - Proper quiet/verbose mode handling
/// - Color support with proper TTY detection
/// - Testable output (via direct stdout/stderr access)
///
/// # Quiet Mode
///
/// When `quiet` is true:
/// - Most output is suppressed
/// - Only errors and critical messages are shown
/// - Progress bars are hidden
///
/// # Verbose Mode
///
/// When `verbose` is true:
/// - Additional debug information is shown
/// - Hints and detailed explanations are displayed
/// - More context is provided for operations
///
/// # Color Support
///
/// The `use_colors` flag controls ANSI color output:
/// - Typically determined by TTY detection
/// - Can be overridden by user preferences
/// - All output methods respect this setting
#[derive(Debug, Clone, Copy)]
pub struct Printer {
    quiet: bool,
    verbose: bool,
    use_colors: bool,
}

impl Printer {
    /// Create a new printer with explicit settings
    ///
    /// # Arguments
    ///
    /// - `quiet`: Suppress most output
    /// - `verbose`: Show additional debug information
    /// - `use_colors`: Enable ANSI color codes
    pub fn new(quiet: bool, verbose: bool, use_colors: bool) -> Self {
        Self {
            quiet,
            verbose,
            use_colors,
        }
    }

    /// Create a printer from global settings
    ///
    /// This is the typical way to create a printer in command implementations.
    ///
    /// ```rust,ignore
    /// use lemma_config::GlobalSettings;
    /// use lemma_output::Printer;
    ///
    /// fn execute(settings: &GlobalSettings) -> Result<(), Box<dyn std::error::Error>> {
    ///     let printer = Printer::from_settings(settings);
    ///     printer.status("Running command...")?;
    ///     Ok(())
    /// }
    /// ```
    pub fn from_settings(settings: &lemma_config::GlobalSettings) -> Self {
        Self::new(
            settings.is_quiet(),
            settings.is_verbose(),
            settings.use_colors(),
        )
    }

    /// Print a status message (=> Installing...)
    ///
    /// Status messages indicate an ongoing operation. They are suppressed
    /// in quiet mode.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(false, false, true);
    /// printer.status("Downloading toolchain...").unwrap();
    /// ```
    pub fn status(&self, msg: impl AsRef<str>) -> io::Result<()> {
        if !self.quiet {
            let prefix = if self.use_colors {
                "=>".green().bold().to_string()
            } else {
                "=>".to_string()
            };
            writeln!(io::stdout(), "{} {}", prefix, msg.as_ref())
        } else {
            Ok(())
        }
    }

    /// Print a success message (✓ Installed)
    ///
    /// Success messages indicate successful completion. They are suppressed
    /// in quiet mode.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(false, false, true);
    /// printer.success("Toolchain installed successfully").unwrap();
    /// ```
    pub fn success(&self, msg: impl AsRef<str>) -> io::Result<()> {
        if !self.quiet {
            let prefix = if self.use_colors {
                "✓".green().to_string()
            } else {
                "✓".to_string()
            };
            writeln!(io::stdout(), "{} {}", prefix, msg.as_ref())
        } else {
            Ok(())
        }
    }

    /// Print an error message (✗ Failed)
    ///
    /// Error messages are **always shown**, even in quiet mode.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(true, false, true); // Even in quiet mode
    /// printer.error("Failed to download toolchain").unwrap();
    /// ```
    pub fn error(&self, msg: impl AsRef<str>) -> io::Result<()> {
        let prefix = if self.use_colors {
            "✗".red().bold().to_string()
        } else {
            "✗".to_string()
        };
        writeln!(io::stderr(), "{} {}", prefix, msg.as_ref())
    }

    /// Print a warning message (⚠ Warning)
    ///
    /// Warnings are shown unless in quiet mode.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(false, false, true);
    /// printer.warning("Toolchain is outdated").unwrap();
    /// ```
    pub fn warning(&self, msg: impl AsRef<str>) -> io::Result<()> {
        if !self.quiet {
            let prefix = if self.use_colors {
                "⚠".yellow().to_string()
            } else {
                "!".to_string()
            };
            writeln!(io::stderr(), "{} {}", prefix, msg.as_ref())
        } else {
            Ok(())
        }
    }

    /// Print a list item (• Item)
    ///
    /// Used for displaying lists of items. Suppressed in quiet mode.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(false, false, true);
    /// printer.list_item("stable (default)").unwrap();
    /// printer.list_item("v4.24.0").unwrap();
    /// ```
    pub fn list_item(&self, msg: impl AsRef<str>) -> io::Result<()> {
        if !self.quiet {
            let prefix = if self.use_colors {
                "•".cyan().to_string()
            } else {
                "•".to_string()
            };
            writeln!(io::stdout(), "  {} {}", prefix, msg.as_ref())
        } else {
            Ok(())
        }
    }

    /// Print a dimmed hint (shown only in verbose mode)
    ///
    /// Hints provide additional context and are only shown when verbose
    /// mode is enabled.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(false, true, true); // verbose = true
    /// printer.hint("Tip: You can set a default with 'lemma default'").unwrap();
    /// ```
    pub fn hint(&self, msg: impl AsRef<str>) -> io::Result<()> {
        if !self.quiet && self.verbose {
            let text = if self.use_colors {
                msg.as_ref().dimmed().to_string()
            } else {
                msg.as_ref().to_string()
            };
            writeln!(io::stdout(), "{}", text)
        } else {
            Ok(())
        }
    }

    /// Print a section header
    ///
    /// Headers separate major sections of output. Suppressed in quiet mode.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(false, false, true);
    /// printer.header("Installed Toolchains").unwrap();
    /// ```
    pub fn header(&self, msg: impl AsRef<str>) -> io::Result<()> {
        if !self.quiet {
            let text = if self.use_colors {
                msg.as_ref().bold().to_string()
            } else {
                msg.as_ref().to_string()
            };
            writeln!(io::stdout(), "\n{}", text)
        } else {
            Ok(())
        }
    }

    /// Get raw stdout access
    ///
    /// Use this for non-stylized output that shouldn't be filtered by
    /// quiet mode (e.g., command output, directory paths).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lemma_output::Printer;
    /// use std::io::Write;
    ///
    /// let printer = Printer::new(false, false, true);
    /// writeln!(printer.stdout(), "/path/to/toolchain").unwrap();
    /// ```
    pub fn stdout(&self) -> io::Stdout {
        io::stdout()
    }

    /// Get raw stderr access
    ///
    /// Use this for error details that need to bypass the error() method.
    pub fn stderr(&self) -> io::Stderr {
        io::stderr()
    }

    /// Check if quiet mode is enabled
    ///
    /// Useful for conditional logic when printer methods don't fit.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(true, false, true);
    /// assert!(printer.is_quiet());
    /// ```
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }

    /// Check if verbose mode is enabled
    ///
    /// Useful for conditional logic when printer methods don't fit.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(false, true, true);
    /// assert!(printer.is_verbose());
    /// ```
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Check if colors are enabled
    ///
    /// Useful when you need to conditionally apply colors outside of
    /// printer methods.
    pub fn use_colors(&self) -> bool {
        self.use_colors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_printer_creation() {
        let printer = Printer::new(false, false, true);
        assert!(!printer.is_quiet());
        assert!(!printer.is_verbose());
        assert!(printer.use_colors());
    }

    #[test]
    fn test_quiet_mode_suppresses_output() {
        let printer = Printer::new(true, false, false);
        assert!(printer.is_quiet());

        // These should succeed but not print (we can't easily test actual output)
        assert!(printer.status("test").is_ok());
        assert!(printer.success("test").is_ok());
        assert!(printer.list_item("test").is_ok());
    }

    #[test]
    fn test_errors_always_shown() {
        let printer = Printer::new(true, false, false); // quiet mode
                                                        // Errors should still work in quiet mode
        assert!(printer.error("test error").is_ok());
    }

    #[test]
    fn test_hints_only_in_verbose() {
        let non_verbose = Printer::new(false, false, false);
        let verbose = Printer::new(false, true, false);

        assert!(!non_verbose.is_verbose());
        assert!(verbose.is_verbose());
    }
}
