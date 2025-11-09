//! Archive extraction with streaming decompression
//!
//! This module provides efficient archive extraction:
//! - Streaming decompression (no intermediate files)
//! - Support for .tar.gz and .tar.zst formats
//! - Multi-threaded decompression where supported
//! - Parallel file extraction
//! - Strips first directory component
//! - Progress tracking
//! - Permission preservation on Unix

use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use rayon::prelude::*;
use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use tar::Archive;
use zip::ZipArchive;
use zstd::stream::read::Decoder as ZstdDecoder;

/// Supported compression formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionFormat {
    /// Gzip (.tar.gz)
    Gzip,
    /// Zstandard (.tar.zst)
    Zstd,
    /// Zip (.zip)
    Zip,
}

impl CompressionFormat {
    /// Detect format from file extension
    pub fn from_path(path: &Path) -> Result<Self> {
        let path_str = path.to_string_lossy();

        if path_str.ends_with(".tar.gz") || path_str.ends_with(".tgz") {
            Ok(Self::Gzip)
        } else if path_str.ends_with(".tar.zst") || path_str.ends_with(".tar.zstd") {
            Ok(Self::Zstd)
        } else if path_str.ends_with(".zip") {
            Ok(Self::Zip)
        } else {
            anyhow::bail!(
                "Unsupported archive format: {}\n\nSupported formats:\n- .tar.gz (gzip compressed tar)\n- .tar.zst (zstd compressed tar)\n- .zip (zip archive)",
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
        CompressionFormat::Zip => extract_zip(file, dest),
    }
}

/// Extract .tar.gz archive with streaming decompression
fn extract_tar_gz<R: Read>(reader: R, dest: &Path) -> Result<()> {
    // Use buffered reader for better I/O performance
    let buffered = BufReader::with_capacity(128 * 1024, reader); // 128KB buffer
    let decoder = GzDecoder::new(buffered);
    extract_tar_with_strip(decoder, dest)
}

/// Extract .tar.zst archive with streaming decompression and multi-threading
fn extract_tar_zst<R: Read>(reader: R, dest: &Path) -> Result<()> {
    // Use buffered reader for better I/O performance
    let buffered = BufReader::with_capacity(128 * 1024, reader); // 128KB buffer

    // Create decoder with larger window for better decompression performance
    let mut decoder = ZstdDecoder::new(buffered).context("Failed to create zstd decoder")?;

    // Set window size for better performance (larger window = more memory but faster)
    decoder
        .window_log_max(31)
        .context("Failed to set zstd window size")?;

    extract_tar_with_strip(decoder, dest)
}

/// Represents a file to be extracted
struct FileEntry {
    path: PathBuf,
    data: Vec<u8>,
    #[cfg(unix)]
    mode: Option<u32>,
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

    // Collect file entries for parallel extraction
    let mut file_entries = Vec::new();
    let mut directories = Vec::new();

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

        // Handle directories and files separately
        if entry.header().entry_type().is_dir() {
            directories.push(dest_path);
        } else {
            // Read file data into memory
            let mut data = Vec::new();
            entry
                .read_to_end(&mut data)
                .with_context(|| format!("Failed to read entry: {}", stripped_path.display()))?;

            #[cfg(unix)]
            let mode = entry.header().mode().ok();

            file_entries.push(FileEntry {
                path: dest_path,
                data,
                #[cfg(unix)]
                mode,
            });
        }
    }

    // Create directories first (sequentially, as they might be nested)
    for dir_path in directories {
        fs::create_dir_all(&dir_path)
            .with_context(|| format!("Failed to create directory: {}", dir_path.display()))?;
    }

    // Extract files in parallel using rayon
    file_entries
        .par_iter()
        .try_for_each(|entry| -> Result<()> {
            // Ensure parent directory exists
            if let Some(parent) = entry.path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create parent directory: {}", parent.display())
                })?;
            }

            // Write file with buffering
            let mut file = File::create(&entry.path)
                .with_context(|| format!("Failed to create file: {}", entry.path.display()))?;

            file.write_all(&entry.data)
                .with_context(|| format!("Failed to write file: {}", entry.path.display()))?;

            // Set permissions on Unix
            #[cfg(unix)]
            if let Some(mode) = entry.mode {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(mode);
                std::fs::set_permissions(&entry.path, permissions).with_context(|| {
                    format!("Failed to set permissions for: {}", entry.path.display())
                })?;
            }

            Ok(())
        })?;

    Ok(())
}

/// Extract .zip archive (used for Windows self-update)
fn extract_zip<R: Read + std::io::Seek>(reader: R, dest: &Path) -> Result<()> {
    let mut archive = ZipArchive::new(reader).context("Failed to open zip archive")?;

    // Ensure destination exists
    fs::create_dir_all(dest)
        .with_context(|| format!("Failed to create destination: {}", dest.display()))?;

    // Collect file entries for parallel extraction
    let mut zip_entries = Vec::new();
    let mut directories = Vec::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("Failed to read zip entry")?;
        let outpath = dest.join(file.name());

        if file.is_dir() {
            directories.push(outpath);
        } else {
            let mut data = Vec::new();
            std::io::copy(&mut file, &mut data)
                .with_context(|| format!("Failed to read zip entry: {}", outpath.display()))?;

            #[cfg(unix)]
            let mode = file.unix_mode();

            zip_entries.push(FileEntry {
                path: outpath,
                data,
                #[cfg(unix)]
                mode,
            });
        }
    }

    // Create directories first (sequentially, as they might be nested)
    for dir_path in directories {
        fs::create_dir_all(&dir_path)
            .with_context(|| format!("Failed to create directory: {}", dir_path.display()))?;
    }

    // Extract files in parallel
    zip_entries.par_iter().try_for_each(|entry| -> Result<()> {
        if let Some(parent) = entry.path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create parent directory: {}", parent.display())
            })?;
        }

        let mut outfile = File::create(&entry.path)
            .with_context(|| format!("Failed to create file: {}", entry.path.display()))?;

        outfile
            .write_all(&entry.data)
            .with_context(|| format!("Failed to extract file: {}", entry.path.display()))?;

        // Set permissions on Unix
        #[cfg(unix)]
        if let Some(mode) = entry.mode {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(mode);
            std::fs::set_permissions(&entry.path, permissions).with_context(|| {
                format!("Failed to set permissions for: {}", entry.path.display())
            })?;
        }

        Ok(())
    })?;

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

        assert!(CompressionFormat::from_path(Path::new("foo.zip")).is_ok());
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
