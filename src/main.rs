mod archive;
mod cli;
mod config;
mod download;
mod errors;
mod github;
mod install;
mod release;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use cli::{Cli, Commands, ProxyCommands};
use config::Config;

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

    match cli.command {
        Commands::Init {
            no_path,
            default_toolchain,
        } => {
            cmd_init(no_path, &default_toolchain)?;
        }
        Commands::Install { toolchain, force } => {
            cmd_install(&toolchain, force)?;
        }
        Commands::Uninstall { toolchain } => {
            cmd_uninstall(&toolchain)?;
        }
        Commands::List { verbose } => {
            cmd_list(verbose)?;
        }
        Commands::Default { toolchain } => {
            cmd_default(&toolchain)?;
        }
        Commands::Update { toolchain } => {
            cmd_update(toolchain.as_deref())?;
        }
        Commands::Config { path, edit } => {
            cmd_config(path, edit)?;
        }
        Commands::Proxy { command } => {
            cmd_proxy(command)?;
        }
        Commands::Info => {
            cmd_info()?;
        }
        Commands::SelfUpdate => {
            cmd_self_update()?;
        }
    }

    Ok(())
}

fn cmd_init(no_path: bool, default_toolchain: &str) -> Result<()> {
    println!("{} Initializing lemma...", "=>".green().bold());

    let config = Config::default();
    config.save()?;

    let config_path = Config::config_path()?;
    println!("   Created config at: {}", config_path.display());

    if !no_path {
        println!(
            "\n{} Add the following to your shell profile:",
            "Note:".yellow().bold()
        );
        let lemma_home = Config::lemma_home()?;
        println!(
            "   export PATH=\"{}:$PATH\"",
            lemma_home.join("bin").display()
        );
    }

    println!(
        "\n{} Run 'lemma install {}' to install the default toolchain",
        "Next:".green().bold(),
        default_toolchain
    );

    Ok(())
}

fn cmd_install(toolchain: &str, force: bool) -> Result<()> {
    let config = Config::load()?;
    let installer = install::Installer::new(config)?;
    installer.install(toolchain, force)?;
    Ok(())
}

fn cmd_uninstall(_toolchain: &str) -> Result<()> {
    println!("{} Uninstall support coming soon...", "=>".yellow().bold());
    Ok(())
}

fn cmd_list(_verbose: bool) -> Result<()> {
    println!("{} Listing toolchains...", "=>".green().bold());
    println!("   No toolchains installed yet.");
    println!("   Run 'lemma install stable' to install the stable toolchain.");
    Ok(())
}

fn cmd_default(_toolchain: &str) -> Result<()> {
    println!("{} Setting default toolchain...", "=>".yellow().bold());
    Ok(())
}

fn cmd_update(_toolchain: Option<&str>) -> Result<()> {
    println!("{} Update support coming soon...", "=>".yellow().bold());
    Ok(())
}

fn cmd_config(show_path: bool, edit: bool) -> Result<()> {
    let config_path = Config::config_path()?;

    if show_path {
        println!("{}", config_path.display());
        return Ok(());
    }

    if edit {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
        std::process::Command::new(editor)
            .arg(&config_path)
            .status()?;
        return Ok(());
    }

    let config = Config::load()?;
    println!("{} Current configuration:", "=>".green().bold());
    println!("{}", toml::to_string_pretty(&config)?);

    Ok(())
}

fn cmd_proxy(command: ProxyCommands) -> Result<()> {
    let mut config = Config::load()?;

    match command {
        ProxyCommands::SetHttp { url } => {
            config.network.http_proxy = Some(url.clone());
            config.save()?;
            println!("{} HTTP proxy set to: {}", "=>".green().bold(), url);
        }
        ProxyCommands::SetHttps { url } => {
            config.network.https_proxy = Some(url.clone());
            config.save()?;
            println!("{} HTTPS proxy set to: {}", "=>".green().bold(), url);
        }
        ProxyCommands::SetSocks { url } => {
            config.network.socks_proxy = Some(url.clone());
            config.save()?;
            println!("{} SOCKS proxy set to: {}", "=>".green().bold(), url);
        }
        ProxyCommands::SetAuth { username, password } => {
            let password = if let Some(pw) = password {
                pw
            } else {
                rpassword::prompt_password("Password: ")?
            };
            config.network.proxy_auth = Some(format!("{}:{}", username, password));
            config.save()?;
            println!("{} Proxy authentication configured", "=>".green().bold());
        }
        ProxyCommands::Clear => {
            config.network.http_proxy = None;
            config.network.https_proxy = None;
            config.network.socks_proxy = None;
            config.network.proxy_auth = None;
            config.save()?;
            println!("{} Proxy settings cleared", "=>".green().bold());
        }
        ProxyCommands::Show => {
            println!("{} Current proxy configuration:", "=>".green().bold());
            if let Some(ref proxy) = config.network.http_proxy {
                println!("   HTTP:  {}", proxy);
            } else {
                println!("   HTTP:  {}", "not set".dimmed());
            }
            if let Some(ref proxy) = config.network.https_proxy {
                println!("   HTTPS: {}", proxy);
            } else {
                println!("   HTTPS: {}", "not set".dimmed());
            }
            if let Some(ref proxy) = config.network.socks_proxy {
                println!("   SOCKS: {}", proxy);
            } else {
                println!("   SOCKS: {}", "not set".dimmed());
            }
            if config.network.proxy_auth.is_some() {
                println!("   Auth:  {}", "configured".green());
            } else {
                println!("   Auth:  {}", "not set".dimmed());
            }
        }
    }

    Ok(())
}

fn cmd_info() -> Result<()> {
    println!("{}", "Lemma - A Modern Lean4 Toolchain Manager".bold());
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("{}", "Installation Directory:".bold());
    let lemma_home = Config::lemma_home()?;
    println!("  {}", lemma_home.display());
    println!();
    println!("{}", "Configuration File:".bold());
    let config_path = Config::config_path()?;
    println!("  {}", config_path.display());

    Ok(())
}

fn cmd_self_update() -> Result<()> {
    println!(
        "{} Self-update support coming soon...",
        "=>".yellow().bold()
    );
    Ok(())
}
