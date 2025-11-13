//! Command handlers
//!
//! This module contains all command implementations, organized into
//! separate files for better maintainability.

pub mod build;
pub mod cache;
pub mod completions;
pub mod default;
pub mod dir;
pub mod fetch;
pub mod find;
pub mod init;
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
use lemma_output::Printer;

/// Dispatch and execute a command
pub fn handle_command(command: Commands, settings: GlobalSettings) -> Result<()> {
    // Create printer from settings for consistent output handling
    let printer = Printer::new(
        settings.is_quiet(),
        settings.is_verbose(),
        settings.use_colors(),
    );
    match command {
        Commands::Lean { command } => handle_toolchain_command(command, &settings, &printer),

        Commands::Override { command } => {
            // Ensure setup for override commands
            Config::ensure_setup()?;
            r#override::execute(command, &settings, &printer)
        }

        Commands::Default { toolchain } => {
            // Ensure setup for default command
            Config::ensure_setup()?;
            default::execute(&toolchain, &settings, &printer)
        }

        Commands::Show => show::execute(&settings, &printer),

        Commands::Which { binary, toolchain } => {
            which::execute(&binary, toolchain.as_deref(), &settings, &printer)
        }

        Commands::Run { path, bin, args } => {
            run::execute(path.as_deref(), bin.as_deref(), &args, &settings, &printer)
        }

        Commands::Completions { shell } => completions::execute(shell, &settings, &printer),

        Commands::Fetch {
            package,
            modules,
            auto,
            dry_run,
            path,
        } => fetch::execute(&package, modules, auto, dry_run, path, &settings, &printer),

        Commands::Cache { command } => cache::execute(command, &settings, &printer),

        Commands::Init {
            name,
            path,
            bare,
            std,
            exe,
            lib,
            no_readme,
        } => init::execute(
            name, path, bare, std, exe, lib, no_readme, &settings, &printer,
        ),

        Commands::Build {
            path,
            clear,
            out_dir,
            targets,
        } => build::execute(
            path.as_deref(),
            clear,
            out_dir.as_deref(),
            &targets,
            &settings,
            &printer,
        ),

        Commands::Self_ { command } => handle_self_command(command, &settings, &printer),
    }
}

/// Handle self subcommands
fn handle_self_command(
    command: SelfCommands,
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    match command {
        SelfCommands::Update => self_update::update(settings, printer),
        SelfCommands::Uninstall { yes } => self_update::uninstall(yes, settings, printer),
    }
}

/// Handle toolchain subcommands
fn handle_toolchain_command(
    command: ToolchainCommands,
    settings: &GlobalSettings,
    printer: &Printer,
) -> Result<()> {
    match command {
        ToolchainCommands::Install {
            toolchain,
            force,
            lean_release_json_url: lean_downloads_json_url,
        } => {
            // Ensure setup on first install
            Config::ensure_setup()?;
            install::execute(
                &toolchain,
                force,
                lean_downloads_json_url.as_deref(),
                settings,
                printer,
            )
        }

        ToolchainCommands::Uninstall { toolchain } => {
            uninstall::execute(&toolchain, settings, printer)
        }

        ToolchainCommands::List {
            only_installed,
            only_available,
            lean_release_json_url: lean_downloads_json_url,
        } => list::execute(
            only_installed,
            only_available,
            lean_downloads_json_url.as_deref(),
            settings,
            printer,
        ),

        ToolchainCommands::Dir { toolchain } => {
            Config::ensure_setup()?;
            dir::execute(toolchain.as_deref(), settings, printer)
        }

        ToolchainCommands::Find { request } => {
            Config::ensure_setup()?;
            find::execute(request.as_deref(), settings, printer)
        }

        ToolchainCommands::Link { name, path } => {
            // Ensure setup for link command too
            Config::ensure_setup()?;
            link::execute(&name, &path, settings, printer)
        }

        ToolchainCommands::Upgrade { toolchain } => {
            upgrade::execute(toolchain.as_deref(), settings, printer)
        }
    }
}
