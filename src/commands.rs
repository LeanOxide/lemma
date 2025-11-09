//! Command handlers
//!
//! This module contains all command implementations, organized into
//! separate files for better maintainability.

pub mod completions;
pub mod default;
pub mod fetch;
pub mod install;
pub mod link;
pub mod list;
pub mod r#override;
pub mod proxy_mode;
pub mod run;
pub mod self_update;
pub mod show;
pub mod uninstall;
pub mod update;
pub mod which;

use anyhow::Result;

use crate::cli::{Commands, SelfCommands, ToolchainCommands};

use crate::config::Config;

/// Dispatch and execute a command
pub fn handle_command(command: Commands) -> Result<()> {
    match command {
        Commands::Toolchain { command } => handle_toolchain_command(command),

        Commands::Override { command } => {
            // Ensure setup for override commands
            Config::ensure_setup()?;
            r#override::execute(command)
        }

        Commands::Default { toolchain } => {
            // Ensure setup for default command
            Config::ensure_setup()?;
            default::execute(&toolchain)
        }

        Commands::Show => show::execute(),

        Commands::Which { binary, toolchain } => which::execute(&binary, toolchain.as_deref()),

        Commands::Update { toolchain } => update::execute(toolchain.as_deref()),

        Commands::Run { toolchain, command } => run::execute(&toolchain, &command),

        Commands::Completions { shell } => completions::execute(shell),

        Commands::Fetch {
            package,
            modules,
            auto,
            dry_run,
            path,
        } => fetch::execute(&package, modules, auto, dry_run, path),

        Commands::Self_ { command } => handle_self_command(command),
    }
}

/// Handle self subcommands
fn handle_self_command(command: SelfCommands) -> Result<()> {
    match command {
        SelfCommands::Update => self_update::update(),
        SelfCommands::Uninstall { yes } => self_update::uninstall(yes),
    }
}

/// Handle toolchain subcommands
fn handle_toolchain_command(command: ToolchainCommands) -> Result<()> {
    match command {
        ToolchainCommands::Install { toolchain, force } => {
            // Ensure setup on first install
            Config::ensure_setup()?;
            install::execute(&toolchain, force)
        }

        ToolchainCommands::Uninstall { toolchain } => uninstall::execute(&toolchain),

        ToolchainCommands::List { verbose } => list::execute(verbose),

        ToolchainCommands::Link { name, path } => {
            // Ensure setup for link command too
            Config::ensure_setup()?;
            link::execute(&name, &path)
        }
    }
}
