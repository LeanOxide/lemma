//! Environment variables used or supported by lemma.

/// Declares all environment variables used throughout `lemma` and its crates.
pub struct EnvVars;

impl EnvVars {
    /// Override the default Lemma home directory.
    ///
    /// By default, Lemma stores toolchains and configuration in `~/.lemma`.
    /// Set this variable to use a different directory.
    pub const LEMMA_HOME: &'static str = "LEMMA_HOME";

    /// Specify which toolchain to use.
    ///
    /// This overrides the active toolchain determined by directory overrides
    /// or the default toolchain.
    pub const LEMMA_TOOLCHAIN: &'static str = "LEMMA_TOOLCHAIN";

    /// Disable configuration file discovery.
    ///
    /// When set, lemma will not search for or load configuration files.
    pub const LEMMA_NO_CONFIG: &'static str = "LEMMA_NO_CONFIG";

    /// Set verbosity level.
    ///
    /// Equivalent to the `-v` flag. Can be set to a number (1, 2, 3, etc.)
    /// for different verbosity levels.
    pub const LEMMA_VERBOSE: &'static str = "LEMMA_VERBOSE";

    /// Control colored output.
    ///
    /// Equivalent to the `--color` flag. Valid values: auto, always, never.
    pub const LEMMA_COLOR: &'static str = "LEMMA_COLOR";

    /// Disable colored output.
    ///
    /// Equivalent to setting `LEMMA_COLOR=never`.
    /// Provided for compatibility with other tools.
    pub const NO_COLOR: &'static str = "NO_COLOR";

    /// HTTP proxy to use for downloads.
    ///
    /// Example: `http://proxy.example.com:8080`
    pub const HTTP_PROXY: &'static str = "HTTP_PROXY";

    /// HTTPS proxy to use for downloads.
    ///
    /// Example: `https://proxy.example.com:8080`
    pub const HTTPS_PROXY: &'static str = "HTTPS_PROXY";

    /// Hosts to exclude from proxy settings.
    ///
    /// Comma-separated list of hostnames that should not use the proxy.
    pub const NO_PROXY: &'static str = "NO_PROXY";

    /// Alternative Lean release mirror URL.
    ///
    /// By default, lemma downloads from GitHub releases. Set this to use
    /// a custom mirror or CDN.
    pub const LEMMA_MIRROR: &'static str = "LEMMA_MIRROR";

    /// Disable SSL certificate verification.
    ///
    /// WARNING: This is insecure and should only be used for debugging.
    pub const LEMMA_INSECURE: &'static str = "LEMMA_INSECURE";

    /// Sparse cache URL for R2 client.
    ///
    /// URL endpoint for the sparse cache storage service.
    pub const LEMMA_SPARSE_CACHE_URL: &'static str = "LEMMA_SPARSE_CACHE_URL";
}
