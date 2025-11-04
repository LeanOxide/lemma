//! Command handlers
//!
//! This module contains all command implementations, organized into
//! separate files for better maintainability.

pub mod config;
pub mod default;
pub mod info;
pub mod init;
pub mod install;
pub mod link;
pub mod list;
pub mod proxy;
pub mod self_update;
pub mod uninstall;
pub mod update;

use anyhow::Result;

use crate::cli::{Commands, ToolchainCommands};

/// Dispatch and execute a command
pub fn handle_command(command: Commands) -> Result<()> {
    match command {
        Commands::Init {
            no_path,
            default_toolchain,
        } => init::execute(no_path, &default_toolchain),

        Commands::Toolchain { command } => handle_toolchain_command(command),

        Commands::Default { toolchain } => default::execute(&toolchain),

        Commands::Update { toolchain } => update::execute(toolchain.as_deref()),

        Commands::Config { path, edit } => config::execute(path, edit),

        Commands::Proxy { command } => proxy::execute(command),

        Commands::Info => info::execute(),

        Commands::SelfUpdate => self_update::execute(),
    }
}

/// Handle toolchain subcommands
fn handle_toolchain_command(command: ToolchainCommands) -> Result<()> {
    match command {
        ToolchainCommands::Install { toolchain, force } => install::execute(&toolchain, force),

        ToolchainCommands::Uninstall { toolchain } => uninstall::execute(&toolchain),

        ToolchainCommands::List { verbose } => list::execute(verbose),

        ToolchainCommands::Link { name, path } => link::execute(&name, &path),
    }
}
