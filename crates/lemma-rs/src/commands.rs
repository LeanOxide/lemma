//! Command handlers
//!
//! This module contains all command implementations, organized into
//! separate files for better maintainability.

pub mod completions;
pub mod default;
pub mod dir;
pub mod fetch;
pub mod find;
pub mod install;
pub mod link;
pub mod list;
pub mod r#override;
pub mod proxy_mode;
pub mod run;
pub mod self_update;
pub mod show;
pub mod uninstall;
pub mod upgrade;
pub mod which;

use anyhow::Result;
use lemma_cli::{Commands, SelfCommands, ToolchainCommands};
use lemma_config::{Config, GlobalSettings};

/// Dispatch and execute a command
pub fn handle_command(command: Commands, settings: GlobalSettings) -> Result<()> {
    match command {
        Commands::Lean { command } => handle_toolchain_command(command, &settings),

        Commands::Override { command } => {
            // Ensure setup for override commands
            Config::ensure_setup()?;
            r#override::execute(command, &settings)
        }

        Commands::Default { toolchain } => {
            // Ensure setup for default command
            Config::ensure_setup()?;
            default::execute(&toolchain, &settings)
        }

        Commands::Show => show::execute(&settings),

        Commands::Which { binary, toolchain } => {
            which::execute(&binary, toolchain.as_deref(), &settings)
        }

        Commands::Run { toolchain, command } => run::execute(&toolchain, &command, &settings),

        Commands::Completions { shell } => completions::execute(shell, &settings),

        Commands::Fetch {
            package,
            modules,
            auto,
            dry_run,
            path,
        } => fetch::execute(&package, modules, auto, dry_run, path, &settings),

        Commands::Self_ { command } => handle_self_command(command, &settings),
    }
}

/// Handle self subcommands
fn handle_self_command(command: SelfCommands, settings: &GlobalSettings) -> Result<()> {
    match command {
        SelfCommands::Update => self_update::update(settings),
        SelfCommands::Uninstall { yes } => self_update::uninstall(yes, settings),
    }
}

/// Handle toolchain subcommands
fn handle_toolchain_command(command: ToolchainCommands, settings: &GlobalSettings) -> Result<()> {
    match command {
        ToolchainCommands::Install { toolchain, force } => {
            // Ensure setup on first install
            Config::ensure_setup()?;
            install::execute(&toolchain, force, settings)
        }

        ToolchainCommands::Uninstall { toolchain } => uninstall::execute(&toolchain, settings),

        ToolchainCommands::List {
            only_installed,
            only_available,
        } => list::execute(only_installed, only_available, settings),

        ToolchainCommands::Dir { toolchain } => {
            Config::ensure_setup()?;
            dir::execute(toolchain.as_deref(), settings)
        }

        ToolchainCommands::Find { request } => {
            Config::ensure_setup()?;
            find::execute(request.as_deref(), settings)
        }

        ToolchainCommands::Link { name, path } => {
            // Ensure setup for link command too
            Config::ensure_setup()?;
            link::execute(&name, &path, settings)
        }

        ToolchainCommands::Upgrade { toolchain } => {
            upgrade::execute(toolchain.as_deref(), settings)
        }
    }
}
