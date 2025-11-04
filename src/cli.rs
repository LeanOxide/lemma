//! Command-line interface for Lemma

use clap::{Parser, Subcommand};

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
    /// Initialize lemma configuration
    Init {
        /// Skip PATH configuration
        #[arg(long)]
        no_path: bool,

        /// Default toolchain to install
        #[arg(short, long, default_value = "stable")]
        default_toolchain: String,
    },

    /// Manage toolchains
    Toolchain {
        #[command(subcommand)]
        command: ToolchainCommands,
    },

    /// Set the default toolchain
    Default {
        /// Toolchain to set as default
        toolchain: String,
    },

    /// Update installed toolchains
    Update {
        /// Specific toolchain to update (updates all if not specified)
        toolchain: Option<String>,
    },

    /// Show lemma configuration
    Config {
        /// Show configuration file path
        #[arg(long)]
        path: bool,

        /// Edit configuration file
        #[arg(short, long)]
        edit: bool,
    },

    /// Configure proxy settings
    Proxy {
        #[command(subcommand)]
        command: ProxyCommands,
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
pub enum ProxyCommands {
    /// Set HTTP proxy
    SetHttp {
        /// Proxy URL (e.g., http://proxy.example.com:8080)
        url: String,
    },

    /// Set HTTPS proxy
    SetHttps {
        /// Proxy URL
        url: String,
    },

    /// Set SOCKS5 proxy
    SetSocks {
        /// Proxy URL (e.g., socks5://127.0.0.1:1080)
        url: String,
    },

    /// Set proxy authentication
    SetAuth {
        /// Username
        username: String,

        /// Password
        #[arg(short, long)]
        password: Option<String>,
    },

    /// Clear proxy settings
    Clear,

    /// Show current proxy settings
    Show,
}
