//! Completions command - Generate shell completion scripts

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

use lemma_cli::Cli;
use lemma_config::GlobalSettings;

pub fn execute(shell: Shell, _settings: &GlobalSettings) -> Result<()> {
    let mut cmd = Cli::command();

    generate(shell, &mut cmd, "lemma", &mut io::stdout());

    Ok(())
}
