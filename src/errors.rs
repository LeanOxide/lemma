//! Error types with detailed diagnostics
//!
//! Unlike elan's generic errors, these provide:
//! - Specific error types for different failure modes
//! - Network diagnostic information
//! - Actionable suggestions for users

use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("HTTP error {status} while downloading {url}: {message}")]
    HttpError {
        status: u16,
        url: String,
        message: String,
    },

    #[error("Network error while downloading {url}: {error}\n\nPossible causes:\n- Check your internet connection\n- Verify proxy settings (HTTP_PROXY, HTTPS_PROXY)\n- Check if the URL is accessible\n- Try running with LEMMA_LOG=debug for more details")]
    NetworkError { url: String, error: String },

    #[error("Proxy connection failed: {0}\n\nPlease verify:\n- Proxy URL format (http://host:port or socks5://host:port)\n- Proxy authentication credentials\n- Proxy server is reachable")]
    ProxyError(String),

    #[error("SSL/TLS error: {0}\n\nPossible solutions:\n- Check system date/time is correct\n- Update CA certificates\n- If behind corporate proxy, you may need custom CA certificates")]
    TlsError(String),

    #[error("Download timeout after {seconds} seconds\n\nSuggestions:\n- Increase timeout in ~/.lemma/config.toml\n- Check network stability\n- Try using a mirror (if available)")]
    Timeout { seconds: u64 },
}

#[derive(Error, Debug)]
pub enum ToolchainError {
    #[error("Toolchain '{name}' not found\n\nAvailable toolchains:\n- stable\n- nightly\n- v4.x.x (specific version)\n- owner/repo:tag (custom repository)")]
    NotFound { name: String },

    #[error("Invalid toolchain name: {name}\n\nValid formats:\n- stable, nightly\n- v4.0.0, v4.1.0, etc.\n- owner/repo:tag")]
    InvalidName { name: String },

    #[error("Failed to parse version: {0}")]
    InvalidVersion(String),

    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    #[error("Toolchain '{name}' is already installed at {path}")]
    AlreadyInstalled { name: String, path: String },
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Configuration file not found at {path}\n\nRun 'lemma init' to create default configuration")]
    NotFound { path: String },

    #[error(
        "Failed to parse configuration: {0}\n\nPlease check TOML syntax in ~/.lemma/config.toml"
    )]
    ParseError(String),
}

#[derive(Error, Debug)]
pub enum GitHubError {
    #[error("GitHub API rate limit exceeded\n\nSolutions:\n- Set LEMMA_GITHUB_TOKEN environment variable with a personal access token\n- Wait {reset_in} seconds for rate limit to reset\n- Use a mirror instead of GitHub")]
    RateLimitExceeded { reset_in: u64 },

    #[error("GitHub API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Release '{tag}' not found in repository {repo}")]
    ReleaseNotFound { repo: String, tag: String },

    #[error("No suitable asset found for platform {platform} in release {tag}\n\nAvailable assets:\n{available}")]
    NoSuitableAsset {
        platform: String,
        tag: String,
        available: String,
    },
}

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("Unsupported archive format: {}\n\nSupported formats:\n- .tar.gz (gzip compressed tar)\n- .tar.zst (zstd compressed tar)", path.display())]
    UnsupportedFormat { path: PathBuf },

    #[error("SHA-256 checksum mismatch for {}\n\nExpected: {expected}\nActual:   {actual}\n\nThe downloaded file may be corrupted. Please try again.", path.display())]
    ChecksumMismatch {
        expected: String,
        actual: String,
        path: PathBuf,
    },

    #[error("Failed to extract archive: {0}")]
    ExtractionFailed(String),

    #[error("Archive is corrupted or invalid: {0}")]
    CorruptedArchive(String),
}
