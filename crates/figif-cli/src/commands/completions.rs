//! `figif completions` command - generate shell completions.

use clap::CommandFactory;
use clap_complete::generate;
use color_eyre::eyre::Result;
use std::io;

use super::{Cli, CompletionsArgs};

pub fn run(args: CompletionsArgs) -> Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();

    generate(args.shell, &mut cmd, name, &mut io::stdout());

    Ok(())
}
