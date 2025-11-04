//! Download client with full proxy support and retry logic
//!
//! Unlike elan's curl backend which has NO proxy support, this module provides:
//! - HTTP, HTTPS, and SOCKS5 proxy support
//! - Automatic retry with exponential backoff
//! - Download resumption for partial downloads
//! - Bandwidth limiting
//! - Progress reporting
//! - Detailed error messages with network diagnostics

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use reqwest::{Proxy, StatusCode};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::thread;
use std::time::Duration;
use url::Url;

use crate::config::Config;
use crate::errors::DownloadError;

/// Download client with retry logic and proxy support
#[derive(Clone)]
pub struct DownloadClient {
    client: Client,
    config: Config,
}

impl DownloadClient {
    /// Create a new download client from configuration
    pub fn new(config: Config) -> Result<Self> {
        let mut client_builder = Client::builder()
            .connect_timeout(config.connect_timeout())
            .timeout(config.read_timeout())
            .user_agent(format!("lemma/{}", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::limited(10));

        // Configure HTTP proxy
        if let Some(ref proxy_url) = config.network.http_proxy {
            let proxy = Proxy::http(proxy_url).context("Invalid HTTP proxy URL")?;

            // Add proxy authentication if configured
            let proxy = if let Some(ref auth) = config.network.proxy_auth {
                Self::add_proxy_auth(proxy, auth)?
            } else {
                proxy
            };

            client_builder = client_builder.proxy(proxy);
        }

        // Configure HTTPS proxy
        if let Some(ref proxy_url) = config.network.https_proxy {
            let proxy = Proxy::https(proxy_url).context("Invalid HTTPS proxy URL")?;

            let proxy = if let Some(ref auth) = config.network.proxy_auth {
                Self::add_proxy_auth(proxy, auth)?
            } else {
                proxy
            };

            client_builder = client_builder.proxy(proxy);
        }

        // Configure SOCKS proxy
        if let Some(ref proxy_url) = config.network.socks_proxy {
            let proxy = Proxy::all(proxy_url).context("Invalid SOCKS proxy URL")?;

            client_builder = client_builder.proxy(proxy);
        }

        // Configure NO_PROXY
        if let Some(ref _no_proxy) = config.network.no_proxy {
            client_builder = client_builder.no_proxy();
            // Note: reqwest handles NO_PROXY env var automatically
        }

        // Danger zone: skip SSL verification (only for testing)
        if config.network.insecure {
            eprintln!("WARNING: SSL certificate verification is disabled!");
            client_builder = client_builder.danger_accept_invalid_certs(true);
        }

        let client = client_builder
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self { client, config })
    }

    /// Add proxy authentication to a Proxy
    fn add_proxy_auth(proxy: Proxy, auth: &str) -> Result<Proxy> {
        let parts: Vec<&str> = auth.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Proxy auth must be in format 'username:password'");
        }
        Ok(proxy.basic_auth(parts[0], parts[1]))
    }

    /// Download a file with retry logic and progress reporting
    pub fn download_file<P: AsRef<Path>>(&self, url: &str, dest: P) -> Result<()> {
        let dest = dest.as_ref();
        let parsed_url = Url::parse(url).context("Invalid download URL")?;

        // Attempt download with retries
        for attempt in 0..=self.config.network.max_retries {
            match self.try_download(&parsed_url, dest, attempt) {
                Ok(()) => return Ok(()),
                Err(e) if attempt < self.config.network.max_retries => {
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
                        self.config.network.max_retries + 1
                    ));
                }
            }
        }

        unreachable!()
    }

    /// Try to download with resume support
    fn try_download(&self, url: &Url, dest: &Path, _attempt: u32) -> Result<()> {
        // Check if we can resume
        let (resume_from, mut file) = if self.config.network.resume_downloads && dest.exists() {
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

                    // Apply bandwidth limit if configured
                    if self.config.network.max_download_speed > 0 {
                        let delay =
                            (n as f64 / self.config.network.max_download_speed as f64) * 1000.0;
                        if delay > 0.0 {
                            thread::sleep(Duration::from_millis(delay as u64));
                        }
                    }
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
        let base_delay_secs = self.config.network.retry_delay;
        let exponential = 2_u64.pow(attempt);
        Duration::from_secs(base_delay_secs * exponential)
    }

    /// Download to memory (for small files like manifests)
    pub fn download_to_string(&self, url: &str) -> Result<String> {
        let parsed_url = Url::parse(url).context("Invalid URL")?;

        for attempt in 0..=self.config.network.max_retries {
            match self.try_download_string(&parsed_url) {
                Ok(content) => return Ok(content),
                Err(e) if attempt < self.config.network.max_retries => {
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
        let config = Config::default();
        let client = DownloadClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_backoff_calculation() {
        let config = Config::default();
        let client = DownloadClient::new(config).unwrap();

        assert_eq!(client.calculate_backoff(0), Duration::from_secs(2));
        assert_eq!(client.calculate_backoff(1), Duration::from_secs(4));
        assert_eq!(client.calculate_backoff(2), Duration::from_secs(8));
    }
}
