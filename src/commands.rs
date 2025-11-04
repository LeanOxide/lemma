//! Command handlers
//!
//! This module contains all command implementations, organized into
//! separate files for better maintainability.

pub mod config;
pub mod default;
pub mod info;
pub mod init;
pub mod install;
pub mod list;
pub mod proxy;
pub mod self_update;
pub mod uninstall;
pub mod update;

use anyhow::Result;

use crate::cli::Commands;

/// Dispatch and execute a command
pub fn handle_command(command: Commands) -> Result<()> {
    match command {
        Commands::Init {
            no_path,
            default_toolchain,
        } => init::execute(no_path, &default_toolchain),

        Commands::Install { toolchain, force } => install::execute(&toolchain, force),

        Commands::Uninstall { toolchain } => uninstall::execute(&toolchain),

        Commands::List { verbose } => list::execute(verbose),

        Commands::Default { toolchain } => default::execute(&toolchain),

        Commands::Update { toolchain } => update::execute(toolchain.as_deref()),

        Commands::Config { path, edit } => config::execute(path, edit),

        Commands::Proxy { command } => proxy::execute(command),

        Commands::Info => info::execute(),

        Commands::SelfUpdate => self_update::execute(),
    }
}
