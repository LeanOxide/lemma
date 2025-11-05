//! Archive extraction with streaming decompression
//!
//! This module provides efficient archive extraction:
//! - Streaming decompression (no intermediate files)
//! - Support for .tar.gz and .tar.zst formats
//! - Strips first directory component
//! - Progress tracking
//! - Permission preservation on Unix

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;
use zstd::stream::read::Decoder as ZstdDecoder;

/// Supported compression formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    /// Gzip (.tar.gz)
    Gzip,
    /// Zstandard (.tar.zst)
    Zstd,
}

impl CompressionFormat {
    /// Detect format from file extension
    pub fn from_path(path: &Path) -> Result<Self> {
        let path_str = path.to_string_lossy();

        if path_str.ends_with(".tar.gz") || path_str.ends_with(".tgz") {
            Ok(Self::Gzip)
        } else if path_str.ends_with(".tar.zst") || path_str.ends_with(".tar.zstd") {
            Ok(Self::Zstd)
        } else {
            anyhow::bail!(
                "Unsupported archive format: {}\n\nSupported formats:\n- .tar.gz (gzip compressed tar)\n- .tar.zst (zstd compressed tar)",
                path.display()
            )
        }
    }
}

/// Extract archive to destination, stripping the first directory component
///
/// This follows the elan pattern where archives contain a top-level directory
/// (e.g., "lean-4.0.0/") that we want to remove during extraction.
pub fn extract_archive(archive_path: &Path, dest: &Path) -> Result<()> {
    let format = CompressionFormat::from_path(archive_path)?;

    let file = File::open(archive_path)
        .with_context(|| format!("Failed to open archive: {}", archive_path.display()))?;

    match format {
        CompressionFormat::Gzip => extract_tar_gz(file, dest),
        CompressionFormat::Zstd => extract_tar_zst(file, dest),
    }
}

/// Extract .tar.gz archive with streaming decompression
fn extract_tar_gz<R: Read>(reader: R, dest: &Path) -> Result<()> {
    let decoder = GzDecoder::new(reader);
    extract_tar_with_strip(decoder, dest)
}

/// Extract .tar.zst archive with streaming decompression
fn extract_tar_zst<R: Read>(reader: R, dest: &Path) -> Result<()> {
    let decoder = ZstdDecoder::new(reader).context("Failed to create zstd decoder")?;
    extract_tar_with_strip(decoder, dest)
}

/// Extract tar archive while stripping the first path component
///
/// This is inspired by elan's `unpack_without_first_dir` functionality.
/// For example:
///   lean-4.0.0/bin/lean  -> bin/lean
///   lean-4.0.0/lib/      -> lib/
fn extract_tar_with_strip<R: Read>(reader: R, dest: &Path) -> Result<()> {
    let mut archive = Archive::new(reader);

    // Ensure destination exists
    fs::create_dir_all(dest)
        .with_context(|| format!("Failed to create destination: {}", dest.display()))?;

    let entries = archive
        .entries()
        .context("Failed to read archive entries")?;

    for entry_result in entries {
        let mut entry = entry_result.context("Failed to read archive entry")?;

        let path = entry.path().context("Failed to get entry path")?;

        // Strip the first component
        let stripped_path = strip_first_component(&path)?;

        // Skip if nothing remains after stripping
        if stripped_path.as_os_str().is_empty() {
            continue;
        }

        let dest_path = dest.join(&stripped_path);

        // Extract the entry
        entry
            .unpack(&dest_path)
            .with_context(|| format!("Failed to extract: {}", stripped_path.display()))?;
    }

    Ok(())
}

/// Strip the first component from a path
///
/// Examples:
///   "foo/bar/baz" -> "bar/baz"
///   "foo" -> ""
///   "foo/" -> ""
fn strip_first_component(path: &Path) -> Result<PathBuf> {
    let mut components = path.components();

    // Skip the first component
    components.next();

    // Collect remaining components
    Ok(components.as_path().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format() {
        assert_eq!(
            CompressionFormat::from_path(Path::new("foo.tar.gz")).unwrap(),
            CompressionFormat::Gzip
        );

        assert_eq!(
            CompressionFormat::from_path(Path::new("foo.tar.zst")).unwrap(),
            CompressionFormat::Zstd
        );

        assert!(CompressionFormat::from_path(Path::new("foo.zip")).is_err());
    }

    #[test]
    fn test_strip_first_component() {
        let path = Path::new("foo/bar/baz");
        let stripped = strip_first_component(path).unwrap();
        assert_eq!(stripped, Path::new("bar/baz"));

        let path = Path::new("foo");
        let stripped = strip_first_component(path).unwrap();
        assert_eq!(stripped, Path::new(""));
    }
}
