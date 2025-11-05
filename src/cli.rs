//! Command-line interface for Lemma

use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(name = "lemma")]
#[command(about = "A modern Lean4 toolchain manager", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage toolchains
    Toolchain {
        #[command(subcommand)]
        command: ToolchainCommands,
    },

    /// Modify directory toolchain overrides
    Override {
        #[command(subcommand)]
        command: OverrideCommands,
    },

    /// Set the default toolchain
    Default {
        /// Toolchain to set as default
        toolchain: String,
    },

    /// Show the active toolchain and installed toolchains
    Show,

    /// Display the path to a binary in the active toolchain
    Which {
        /// Name of the binary (e.g., lean, lake, leanc)
        binary: String,
    },

    /// Update installed toolchains
    Update {
        /// Specific toolchain to update (updates all if not specified)
        toolchain: Option<String>,
    },

    /// Generate tab-completion scripts for your shell
    Completions {
        /// Shell type
        shell: Shell,
    },

    /// Show information about lemma
    Info,

    /// Self-update lemma
    SelfUpdate,
}

#[derive(Subcommand)]
pub enum ToolchainCommands {
    /// Install a toolchain
    Install {
        /// Toolchain to install (e.g., stable, v4.24.0, owner/repo:tag, https://...)
        toolchain: String,

        /// Force reinstall if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall a toolchain
    Uninstall {
        /// Toolchain to uninstall
        toolchain: String,
    },

    /// List installed toolchains
    List {
        /// Show verbose information
        #[arg(short, long)]
        verbose: bool,
    },

    /// Link a custom toolchain
    Link {
        /// Name for the toolchain
        name: String,

        /// Path to the toolchain directory
        path: String,
    },
}

#[derive(Subcommand)]
pub enum OverrideCommands {
    /// Set directory override for a toolchain
    Set {
        /// Toolchain to use in this directory
        toolchain: String,

        /// Directory to override (defaults to current directory)
        #[arg(long)]
        path: Option<String>,
    },

    /// Remove directory override
    Unset {
        /// Directory to remove override from (defaults to current directory)
        #[arg(long)]
        path: Option<String>,
    },

    /// List all directory overrides
    List,
}
