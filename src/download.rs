//! Download client with retry logic and progress reporting
//!
//! Like elan and rustup, this module:
//! - Uses environment variables for proxy (HTTP_PROXY, HTTPS_PROXY, NO_PROXY)
//! - Automatic retry with exponential backoff
//! - Download resumption for partial downloads
//! - Progress reporting

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;
use url::Url;

use crate::errors::DownloadError;

/// Network timeouts and retry settings (hardcoded like elan/rustup)
const CONNECT_TIMEOUT_SECS: u64 = 30;
const READ_TIMEOUT_SECS: u64 = 30;
const MAX_RETRIES: u32 = 3;

/// Download client with retry logic
/// Proxy configuration is handled automatically via HTTP_PROXY/HTTPS_PROXY env vars
#[derive(Clone)]
pub struct DownloadClient {
    client: Client,
}

impl DownloadClient {
    /// Create a new download client
    /// Proxy configuration is automatically read from HTTP_PROXY/HTTPS_PROXY env vars
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .timeout(Duration::from_secs(READ_TIMEOUT_SECS))
            .user_agent(format!("lemma/{}", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client })
    }

    /// Download a file with retry logic and progress reporting
    pub fn download_file<P: AsRef<Path>>(&self, url: &str, dest: P) -> Result<()> {
        let dest = dest.as_ref();
        let parsed_url = Url::parse(url).context("Invalid download URL")?;

        // Attempt download with retries
        for attempt in 0..=MAX_RETRIES {
            match self.try_download(&parsed_url, dest, attempt) {
                Ok(()) => return Ok(()),
                Err(e) if attempt < MAX_RETRIES => {
                    let delay = self.calculate_backoff(attempt);
                    eprintln!(
                        "Download attempt {} failed: {}. Retrying in {:?}...",
                        attempt + 1,
                        e,
                        delay
                    );
                    thread::sleep(delay);
                }
                Err(e) => {
                    return Err(e).context(format!(
                        "Failed to download after {} attempts",
                        MAX_RETRIES + 1
                    ));
                }
            }
        }

        unreachable!()
    }

    /// Try to download with resume support
    fn try_download(&self, url: &Url, dest: &Path, _attempt: u32) -> Result<()> {
        // Check if we can resume (always enabled, like elan/rustup)
        let (resume_from, mut file) = if dest.exists() {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .open(dest)
                .context("Failed to open file for resume")?;

            let size = file.metadata()?.len();
            if size > 0 {
                println!("Resuming download from byte {}", size);
                (Some(size), file)
            } else {
                (None, file)
            }
        } else {
            (
                None,
                File::create(dest).context("Failed to create destination file")?,
            )
        };

        // Build request with range header for resume
        let mut request = self.client.get(url.as_str());

        if let Some(start) = resume_from {
            request = request.header("Range", format!("bytes={}-", start));
            file.seek(SeekFrom::End(0))?;
        }

        // Execute request
        let response = request.send().context("Failed to send HTTP request")?;

        // Check status
        let status = response.status();
        if !status.is_success() && status != StatusCode::PARTIAL_CONTENT {
            return Err(DownloadError::HttpError {
                status: status.as_u16(),
                url: url.to_string(),
                message: response.text().unwrap_or_default(),
            }
            .into());
        }

        // Get total size for progress bar
        let total_size = response.content_length().or(resume_from).unwrap_or(0);

        // Setup progress bar
        let progress = ProgressBar::new(total_size);
        progress.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                .unwrap()
        );

        // Download with progress
        let mut downloaded = resume_from.unwrap_or(0);
        let mut buffer = vec![0u8; 8192];
        let mut response = response;

        loop {
            match response.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    file.write_all(&buffer[..n])
                        .context("Failed to write to file")?;
                    downloaded += n as u64;
                    progress.set_position(downloaded);
                }
                Err(e) => {
                    return Err(DownloadError::NetworkError {
                        url: url.to_string(),
                        error: e.to_string(),
                    }
                    .into());
                }
            }
        }

        progress.finish_with_message("Download complete");
        file.flush()?;

        Ok(())
    }

    /// Calculate exponential backoff delay
    fn calculate_backoff(&self, attempt: u32) -> Duration {
        const BASE_DELAY_SECS: u64 = 2;
        let exponential = 2_u64.pow(attempt);
        Duration::from_secs(BASE_DELAY_SECS * exponential)
    }

    /// Download to memory (for small files like manifests)
    pub fn download_to_string(&self, url: &str) -> Result<String> {
        let parsed_url = Url::parse(url).context("Invalid URL")?;

        for attempt in 0..=MAX_RETRIES {
            match self.try_download_string(&parsed_url) {
                Ok(content) => return Ok(content),
                Err(e) if attempt < MAX_RETRIES => {
                    let delay = self.calculate_backoff(attempt);
                    eprintln!(
                        "Request attempt {} failed: {}. Retrying in {:?}...",
                        attempt + 1,
                        e,
                        delay
                    );
                    thread::sleep(delay);
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    /// Try to download as string
    fn try_download_string(&self, url: &Url) -> Result<String> {
        let response = self
            .client
            .get(url.as_str())
            .send()
            .context("Failed to send request")?;

        let status = response.status();
        if !status.is_success() {
            return Err(DownloadError::HttpError {
                status: status.as_u16(),
                url: url.to_string(),
                message: response.text().unwrap_or_default(),
            }
            .into());
        }

        response.text().context("Failed to read response body")
    }

    /// Download JSON
    pub fn download_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let content = self.download_to_string(url)?;
        serde_json::from_str(&content).context("Failed to parse JSON response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = DownloadClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_backoff_calculation() {
        let client = DownloadClient::new().unwrap();

        assert_eq!(client.calculate_backoff(0), Duration::from_secs(2));
        assert_eq!(client.calculate_backoff(1), Duration::from_secs(4));
        assert_eq!(client.calculate_backoff(2), Duration::from_secs(8));
    }
}
