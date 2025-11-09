//! Lemma - A modern Lean4 toolchain manager
//!
//! Lemma provides a comprehensive solution for managing Lean4 toolchains with features like:
//! - Full proxy support (HTTP, HTTPS, SOCKS5) with authentication
//! - Custom source configuration and mirrors
//! - Automatic toolchain resolution from project files
//! - Directory-based toolchain overrides
//! - Self-updating capabilities
//!
//! ## Architecture
//!
//! Lemma operates in two modes:
//!
//! 1. **Direct mode**: When invoked as `lemma`, it provides toolchain management commands
//! 2. **Proxy mode**: When invoked as `lean`, `lake`, etc., it acts as a proxy to the
//!    appropriate toolchain binary, ensuring consistent toolchain usage across a project
//!
//! ## Module Organization
//!
//! - `archive`: Archive extraction (tar.gz, zip, zstd)
//! - `cli`: Command-line interface definitions
//! - `commands`: Command implementations (install, update, show, etc.)
//! - `config`: Configuration management and persistence
//! - `download`: HTTP download client with retry logic and progress reporting
//! - `help`: Help text and documentation
//! - `install`: Toolchain installation and verification
//! - `release`: Release channel and version resolution
//! - `toolchain`: Toolchain descriptor parsing and resolution logic

mod archive;
mod cli;
mod commands;
mod config;
mod download;
mod help;
mod install;
mod release;
mod sparse_cache;
mod toolchain;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

use cli::Cli;
use commands::proxy_mode;

/// Entry point for lemma
///
/// Performs initialization tasks and delegates to the main `run()` function.
/// Any errors from `run()` are printed to stderr with user-friendly formatting.
fn main() {
    // Clean up old self-update backup files
    let _ = commands::self_update::cleanup_old_backups();

    if let Err(e) = run() {
        eprintln!("{} {:#}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

/// Main execution logic for lemma
///
/// Determines the operating mode (direct or proxy) based on how the binary was invoked:
/// - If invoked as `lean`, `lake`, etc.: enters proxy mode to execute the appropriate tool
/// - If invoked as `lemma`: parses CLI arguments and dispatches to command handlers
///
/// # Returns
///
/// - `Ok(())` on successful execution
/// - `Err` if any command fails or configuration is invalid
fn run() -> Result<()> {
    // Detect if we're being invoked as a proxy tool
    // by checking argv[0] (the name used to invoke this binary)
    let arg0 = std::env::args().next().map(PathBuf::from);
    let binary_name = arg0
        .as_ref()
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str());

    // If invoked as one of the proxy tools (lean, lake, etc.), enter proxy mode
    if let Some(name) = binary_name {
        if proxy_mode::PROXY_TOOLS.contains(&name) {
            return proxy_mode::execute(name);
        }
    }

    // Otherwise, run as normal lemma CLI
    let cli = Cli::parse();

    // Setup logging based on verbosity
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    tracing_subscriber::fmt::init();

    // Dispatch to appropriate command handler
    commands::handle_command(cli.command)
}
