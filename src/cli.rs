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

    /// Install a toolchain
    Install {
        /// Toolchain to install (e.g., stable, nightly, v4.0.0, owner/repo:tag)
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
