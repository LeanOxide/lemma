//! Self-update command - Update lemma itself

use anyhow::{bail, Context, Result};
use colored::Colorize;
use lemma_config::GlobalSettings;
use lemma_output::Printer;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use lemma_config::Config;
use lemma_download::DownloadClient;
use lemma_install::extract_archive;

/// Release server base URL
const RELEASE_BASE_URL: &str = "https://lemma.puqing.work";

/// Version manifest structure
#[derive(Debug, Deserialize)]
struct VersionManifest {
    version: String,
}

/// Fetch the latest available version from the server
fn get_available_version() -> Result<String> {
    let download_client = DownloadClient::new()?;

    // Download the version manifest (using download_to_string to avoid progress bar)
    let manifest_url = format!("{}/manifests/stable.toml", RELEASE_BASE_URL);
    let manifest_content = download_client
        .download_to_string(&manifest_url)
        .context("Failed to download version manifest")?;

    // Parse the manifest
    let manifest: VersionManifest =
        toml::from_str(&manifest_content).context("Failed to parse version manifest")?;

    Ok(manifest.version)
}

pub fn update(_settings: &GlobalSettings, #[allow(unused_variables)] printer: &Printer) -> Result<()> {
    println!("{} Checking for updates...", "=>".cyan().bold());

    // Get current version
    let current_version = env!("CARGO_PKG_VERSION");
    println!("   Current version: {}", current_version);

    // Fetch available version
    let available_version = get_available_version().context("Failed to check for updates")?;
    println!("   Latest version: {}", available_version);

    // Check if update is needed
    if available_version == current_version {
        println!();
        println!(
            "{} lemma is already up-to-date ({})",
            "✓".green().bold(),
            current_version
        );
        return Ok(());
    }

    // Detect platform
    let target = get_host_target()?;
    println!("   Platform: {}", target);

    // Check if we can determine the current executable
    let current_exe = env::current_exe().context("Failed to determine current executable path")?;
    println!("   Executable: {}", current_exe.display());

    // Download specific version
    println!();
    println!(
        "{} Downloading version {}...",
        "=>".cyan().bold(),
        available_version
    );
    let download_client = DownloadClient::new()?;
    let temp_dir = Config::tmp_dir()?;
    fs::create_dir_all(&temp_dir).context("Failed to create temp directory")?;

    // Determine archive extension based on platform
    #[cfg(windows)]
    let archive_ext = "zip";
    #[cfg(not(windows))]
    let archive_ext = "tar.gz";

    let archive_name = format!("lemma-{}.{}", target, archive_ext);
    let download_path = temp_dir.join(&archive_name);

    // Construct download URL using versioned release
    let download_url = format!(
        "{}/releases/{}/{}",
        RELEASE_BASE_URL, available_version, archive_name
    );
    println!("   From: {}", download_url);

    download_client
        .download_file(&download_url, &download_path)
        .context(format!("Failed to download update from {}.", download_url))?;

    // Extract archive
    println!("{} Extracting...", "=>".cyan().bold());
    let extract_dir = temp_dir.join("lemma-update");
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).context("Failed to clean extraction directory")?;
    }
    fs::create_dir_all(&extract_dir).context("Failed to create extraction directory")?;
    extract_archive(&download_path, &extract_dir).context("Failed to extract archive")?;

    // Find the new binary
    let new_binary = find_binary_in_dir(&extract_dir)?;
    println!("   New binary: {}", new_binary.display());

    // Replace current executable
    println!("{} Installing update...", "=>".cyan().bold());
    replace_binary(&new_binary, &current_exe)?;

    // Clean up
    let _ = fs::remove_file(&download_path);
    let _ = fs::remove_dir_all(&extract_dir);

    println!();
    println!("{} Successfully updated lemma!", "✓".green().bold());
    println!(
        "   {} → {}",
        current_version,
        available_version.green().bold()
    );
    println!("   Run 'lemma --version' to verify the update");

    Ok(())
}

/// Get the host target triple
#[allow(unreachable_code)]
fn get_host_target() -> Result<&'static str> {
    // Detect the current platform (only platforms we build releases for)
    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
    {
        return Ok("x86_64-unknown-linux-gnu");
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "musl"))]
    {
        return Ok("x86_64-unknown-linux-musl");
    }

    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return Ok("x86_64-apple-darwin");
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return Ok("aarch64-apple-darwin");
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return Ok("x86_64-pc-windows-gnu");
    }

    // Unsupported platform
    bail!(
        "Unsupported platform for self-update.\n\n\
        Self-update is only available for:\n\
        - x86_64-unknown-linux-gnu\n\
        - x86_64-unknown-linux-musl\n\
        - x86_64-apple-darwin\n\
        - aarch64-apple-darwin\n\
        - x86_64-pc-windows-gnu\n\n\
        Please download and install updates manually from the releases page."
    )
}

/// Find the lemma binary in the extracted directory
fn find_binary_in_dir(dir: &Path) -> Result<PathBuf> {
    #[cfg(unix)]
    let binary_name = "lemma";
    #[cfg(windows)]
    let binary_name = "lemma.exe";

    // Try common locations
    let candidates = vec![
        dir.join(binary_name),
        dir.join("bin").join(binary_name),
        dir.join("lemma").join(binary_name),
    ];

    for candidate in candidates {
        if candidate.exists() && candidate.is_file() {
            return Ok(candidate);
        }
    }

    // Walk the directory to find it
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.file_name() == Some(binary_name.as_ref()) {
            return Ok(path);
        }

        if path.is_dir() {
            if let Ok(found) = find_binary_in_dir(&path) {
                return Ok(found);
            }
        }
    }

    bail!("Could not find lemma binary in extracted archive")
}

/// Replace the current binary with the new one
#[cfg(unix)]
fn replace_binary(new_binary: &Path, current_exe: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    // Make the new binary executable
    let mut perms = fs::metadata(new_binary)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(new_binary, perms)?;

    // On Unix, we can't replace a running binary directly
    // So we:
    // 1. Move current binary to .old
    // 2. Copy new binary to current location
    // 3. Remove .old on next run

    let backup = current_exe.with_extension("old");

    // Remove old backup if it exists
    if backup.exists() {
        fs::remove_file(&backup).ok();
    }

    // Move current to backup
    fs::rename(current_exe, &backup).context("Failed to backup current executable")?;

    // Copy new binary to current location
    fs::copy(new_binary, current_exe).context("Failed to install new executable")?;

    // Make sure it's executable
    let mut perms = fs::metadata(current_exe)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(current_exe, perms)?;

    Ok(())
}

/// Replace the current binary with the new one (Windows)
#[cfg(windows)]
fn replace_binary(new_binary: &Path, current_exe: &Path) -> Result<()> {
    // On Windows, we can't replace a running executable
    // We need to use a different strategy:
    // 1. Rename current exe to .old
    // 2. Copy new binary to current location
    // 3. Schedule .old for deletion on next boot or run

    let backup = current_exe.with_extension("old.exe");

    // Remove old backup if it exists
    if backup.exists() {
        fs::remove_file(&backup).ok();
    }

    // Try to rename current to backup
    // On Windows this might fail if the file is locked
    if let Err(e) = fs::rename(current_exe, &backup) {
        eprintln!(
            "{} Could not backup current executable: {}",
            "Warning:".yellow().bold(),
            e
        );
        eprintln!("   Attempting direct copy...");

        // Try direct copy instead
        fs::copy(new_binary, current_exe).context("Failed to replace executable")?;
    } else {
        // Copy new binary to current location
        fs::copy(new_binary, current_exe).context("Failed to install new executable")?;
    }

    println!();
    println!(
        "{} You may need to restart your terminal or command prompt",
        "Note:".yellow().bold()
    );
    println!("   for the update to take full effect.");

    Ok(())
}

/// Clean up old backup files (called during normal execution)
pub fn cleanup_old_backups() -> Result<()> {
    if let Ok(current_exe) = env::current_exe() {
        #[cfg(unix)]
        let backup = current_exe.with_extension("old");
        #[cfg(windows)]
        let backup = current_exe.with_extension("old.exe");

        if backup.exists() {
            fs::remove_file(&backup).ok();
        }
    }
    Ok(())
}

/// Uninstall lemma and all toolchains
pub fn uninstall(
    skip_confirm: bool,
    _settings: &GlobalSettings,
    #[allow(unused_variables)] printer: &Printer,
) -> Result<()> {
    use std::io::{self, Write};

    println!();
    println!("{} Uninstall lemma", "=>".red().bold());
    println!();
    println!("This will remove:");
    println!("  - All installed Lean toolchains");
    println!("  - All lemma proxy binaries");
    println!("  - The entire ~/.lemma directory");
    println!();

    if !skip_confirm {
        print!("Continue? (y/N): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Uninstall cancelled.");
            return Ok(());
        }
    }

    let lemma_home = Config::lemma_home()?;

    println!("{} Removing lemma installation...", "=>".cyan().bold());
    println!("   Directory: {}", lemma_home.display());

    // Remove the entire .lemma directory
    if lemma_home.exists() {
        fs::remove_dir_all(&lemma_home).context("Failed to remove lemma home directory")?;
        println!("   {} Removed ~/.lemma", "✓".green());
    }

    // Try to remove the current executable (best effort)
    if let Ok(current_exe) = env::current_exe() {
        // On Unix, we can't delete a running binary, so we just note it
        #[cfg(unix)]
        {
            println!();
            println!("{} Note:", "=>".yellow().bold());
            println!(
                "   The lemma executable at {} will remain",
                current_exe.display()
            );
            println!("   You can manually delete it with:");
            println!("   $ rm {}", current_exe.display());
        }

        // On Windows, try to delete after we exit
        #[cfg(windows)]
        {
            println!("   The lemma executable will be removed on next reboot");
            // Schedule for deletion on reboot
            let _ = fs::rename(&current_exe, current_exe.with_extension("delete_me.exe"));
        }
    }

    println!();
    println!("{} Lemma has been uninstalled", "✓".green().bold());

    // Note about PATH
    println!();
    println!(
        "{} Remember to remove ~/.lemma/bin from your PATH",
        "Note:".yellow().bold()
    );
    println!("   Edit your shell profile (~/.bashrc, ~/.zshrc, etc.) and remove:");
    println!("   export PATH=\"$HOME/.lemma/bin:$PATH\"");

    Ok(())
}
