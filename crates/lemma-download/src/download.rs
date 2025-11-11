//! Download client with retry logic and progress reporting
//!
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

/// Network timeouts and retry settings
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_RETRIES: u32 = 3;

const TEMPLATE_PIP: & str =
        "{bar:40.green/black} {bytes:>11.green}/{total_bytes:<11.green} {bytes_per_sec:>13.red} eta {eta:.blue}";

const CHARS_LINE: &str = "━╾╴─";

#[derive(Clone)]
pub struct DownloadClient {
    client: Client,
    max_retries: u32,
}

impl DownloadClient {
    /// Create a new download client with default settings
    pub fn new() -> Result<Self> {
        Self::with_settings(DEFAULT_CONNECT_TIMEOUT_SECS, DEFAULT_MAX_RETRIES, None, None)
    }

    /// Create a download client from global settings
    pub fn from_settings(settings: &lemma_config::GlobalSettings) -> Result<Self> {
        Self::with_settings(
            settings.network_timeout,
            settings.network_retries,
            settings.http_proxy.clone(),
            settings.https_proxy.clone(),
        )
    }

    /// Create a download client with specific network settings
    fn with_settings(
        timeout_secs: u64,
        max_retries: u32,
        http_proxy: Option<String>,
        https_proxy: Option<String>,
    ) -> Result<Self> {
        let mut builder = Client::builder()
            .connect_timeout(Duration::from_secs(timeout_secs))
            .timeout(Duration::from_secs(timeout_secs))
            .user_agent(format!("lemma/{}", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::limited(10));

        // Add proxies if specified
        if let Some(proxy_url) = http_proxy {
            builder = builder.proxy(
                reqwest::Proxy::http(&proxy_url)
                    .context("Failed to configure HTTP proxy")?,
            );
        }

        if let Some(proxy_url) = https_proxy {
            builder = builder.proxy(
                reqwest::Proxy::https(&proxy_url)
                    .context("Failed to configure HTTPS proxy")?,
            );
        }

        let client = builder
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            max_retries,
        })
    }

    /// Download a file with retry logic and progress reporting
    pub fn download_file<P: AsRef<Path>>(&self, url: &str, dest: P) -> Result<()> {
        let dest = dest.as_ref();
        let parsed_url = Url::parse(url).context("Invalid download URL")?;

        // Attempt download with retries
        for attempt in 0..=self.max_retries {
            match self.try_download(&parsed_url, dest, attempt) {
                Ok(()) => return Ok(()),
                Err(e) if attempt < self.max_retries => {
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
                        self.max_retries + 1
                    ));
                }
            }
        }

        unreachable!()
    }

    /// Try to download with resume support
    fn try_download(&self, url: &Url, dest: &Path, _attempt: u32) -> Result<()> {
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
            let message = response.text().unwrap_or_default();
            anyhow::bail!(
                "HTTP error {} while downloading {}: {}",
                status.as_u16(),
                url,
                message
            );
        }

        // Calculate total size for progress bar
        // When resuming, content_length() returns remaining bytes, not total file size
        let total_size = if let Some(resumed) = resume_from {
            // We're resuming: total = already_downloaded + remaining
            response
                .content_length()
                .map(|remaining| resumed + remaining)
                .unwrap_or(resumed)
        } else {
            // Fresh download: use content_length directly
            response.content_length().unwrap_or(0)
        };

        // Setup progress bar
        let progress = ProgressBar::new(total_size);
        progress.set_style(
            ProgressStyle::default_bar()
                .progress_chars(CHARS_LINE)
                .template(TEMPLATE_PIP)
                .unwrap(),
        );

        // Download with progress - start from where we left off
        let mut downloaded = resume_from.unwrap_or(0);
        progress.set_position(downloaded);
        let mut buffer = vec![0u8; 65536]; // 64KB buffer for better I/O performance
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
                    anyhow::bail!("Network error while downloading {}: {}", url, e);
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

    /// Download to memory
    pub fn download_to_string(&self, url: &str) -> Result<String> {
        let parsed_url = Url::parse(url).context("Invalid URL")?;

        for attempt in 0..=self.max_retries {
            match self.try_download_string(&parsed_url) {
                Ok(content) => return Ok(content),
                Err(e) if attempt < self.max_retries => {
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
            let message = response.text().unwrap_or_default();
            anyhow::bail!(
                "HTTP error {} while fetching {}: {}",
                status.as_u16(),
                url,
                message
            );
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
