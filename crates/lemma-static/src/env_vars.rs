//! Environment variables used or supported by lemma.

/// Declares all environment variables used throughout `lemma` and its crates.
pub struct EnvVars;

impl EnvVars {
    /// Override the default Lemma home directory.
    ///
    /// By default, Lemma stores toolchains and configuration in `~/.lemma`.
    /// Set this variable to use a different directory.
    pub const LEMMA_HOME: &str = "LEMMA_HOME";

    /// Specify which toolchain to use.
    ///
    /// This overrides the active toolchain determined by directory overrides
    /// or the default toolchain.
    pub const LEMMA_TOOLCHAIN: &str = "LEMMA_TOOLCHAIN";

    /// Set verbosity level.
    ///
    /// Equivalent to the `-v` flag. Can be set to a number (1, 2, 3, etc.)
    /// for different verbosity levels.
    pub const LEMMA_VERBOSE: &str = "LEMMA_VERBOSE";

    /// Control colored output.
    ///
    /// Equivalent to the `--color` flag. Valid values: auto, always, never.
    pub const LEMMA_COLOR: &str = "LEMMA_COLOR";

    /// Disable colored output.
    ///
    /// Equivalent to setting `LEMMA_COLOR=never`.
    /// Provided for compatibility with other tools.
    pub const NO_COLOR: &str = "NO_COLOR";

    /// Lean release server URL.
    ///
    /// URL endpoint for Lean toolchain releases.
    pub const LEMMA_RELEASE_URL: &str = "LEMMA_RELEASE_URL";
}
