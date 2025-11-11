//! Progress bar support for long-running operations
//!
//! This module provides progress bar integration that respects quiet mode
//! and provides consistent styling across lemma.

use indicatif::{ProgressBar, ProgressStyle};

use crate::Printer;

impl Printer {
    /// Create a progress bar for operations with known length
    ///
    /// The progress bar automatically:
    /// - Hides in quiet mode
    /// - Respects color settings
    /// - Uses consistent styling
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(false, false, true);
    /// let pb = printer.progress_bar(1024 * 1024); // 1MB
    ///
    /// // Update progress as work completes
    /// for chunk in 0..100 {
    ///     pb.inc(1024 * 10); // 10KB
    ///     // ... do work
    /// }
    ///
    /// pb.finish_with_message("Download complete");
    /// ```
    pub fn progress_bar(&self, len: u64) -> ProgressBar {
        if self.quiet {
            ProgressBar::hidden()
        } else {
            let pb = ProgressBar::new(len);

            let template = if self.use_colors {
                "[{bar:40.cyan/blue}] {bytes}/{total_bytes} {msg}"
            } else {
                "[{bar:40}] {bytes}/{total_bytes} {msg}"
            };

            pb.set_style(
                ProgressStyle::default_bar()
                    .template(template)
                    .unwrap()
                    .progress_chars("=>-"),
            );
            pb
        }
    }

    /// Create a spinner for operations with unknown length
    ///
    /// Spinners are useful for operations where progress can't be measured,
    /// but you want to show that work is happening.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use lemma_output::Printer;
    /// let printer = Printer::new(false, false, true);
    /// let spinner = printer.spinner("Resolving dependencies...");
    ///
    /// // Spinner animates automatically
    /// // ... do work
    ///
    /// spinner.finish_with_message("Dependencies resolved");
    /// ```
    pub fn spinner(&self, msg: &str) -> ProgressBar {
        if self.quiet {
            ProgressBar::hidden()
        } else {
            let pb = ProgressBar::new_spinner();
            pb.set_message(msg.to_string());

            if self.use_colors {
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner:.cyan} {msg}")
                        .unwrap(),
                );
            } else {
                pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{spinner} {msg}")
                        .unwrap(),
                );
            }

            pb
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_hidden_in_quiet_mode() {
        let printer = Printer::new(true, false, false);
        let pb = printer.progress_bar(100);
        assert!(pb.is_hidden());
    }

    #[test]
    fn test_progress_bar_visible_in_normal_mode() {
        let printer = Printer::new(false, false, false);
        let pb = printer.progress_bar(100);
        // Can't easily test visibility, but shouldn't panic
        pb.finish();
    }

    #[test]
    fn test_spinner_hidden_in_quiet_mode() {
        let printer = Printer::new(true, false, false);
        let spinner = printer.spinner("test");
        assert!(spinner.is_hidden());
    }
}
