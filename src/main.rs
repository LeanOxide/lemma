mod archive;
mod cli;
mod commands;
mod config;
mod download;
mod errors;
mod github;
mod install;
mod release;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use cli::Cli;

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {:#}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging based on verbosity
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    tracing_subscriber::fmt::init();

    // Dispatch to appropriate command handler
    commands::handle_command(cli.command)
}
