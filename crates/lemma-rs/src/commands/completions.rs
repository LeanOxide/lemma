//! Completions command - Generate shell completion scripts

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

use crate::cli::Cli;
use crate::settings::GlobalSettings;

pub fn execute(shell: Shell, _settings: &GlobalSettings) -> Result<()> {
    let mut cmd = Cli::command();

    generate(shell, &mut cmd, "lemma", &mut io::stdout());

    Ok(())
}
