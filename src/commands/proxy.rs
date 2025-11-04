//! Proxy command - Configure proxy settings

use anyhow::Result;
use colored::Colorize;

use crate::cli::ProxyCommands;
use crate::config::Config;

pub fn execute(command: ProxyCommands) -> Result<()> {
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
