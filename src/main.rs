mod archive;
mod cli;
mod commands;
mod config;
mod download;
mod help;
mod install;
mod release;
mod toolchain;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use std::path::PathBuf;

use cli::Cli;
use commands::proxy_mode;

fn main() {
    // Clean up old self-update backup files
    let _ = commands::self_update::cleanup_old_backups();

    if let Err(e) = run() {
        eprintln!("{} {:#}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

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
