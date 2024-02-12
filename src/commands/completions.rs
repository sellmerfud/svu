
use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use crate::util::SvError::*;
use clap_complete::{generate, shells::Shell};

/// Generate shell completions and write them to stdout.
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]

pub struct Completions {
    /// Target shell for completions.  Omit to use current shell.
    #[arg()]
    shell: Option<Shell>,
}

impl Completions {
    pub fn run(&mut self) -> Result<()> {
        let shell = self.shell
            .or(Shell::from_env())
            .ok_or(General("Cannot determine shell".to_owned()))?;
        let mut clap_cmd = crate::app::Commands::command();
        let name = clap_cmd.get_name().to_owned();
        generate(shell, &mut clap_cmd, name, &mut std::io::stdout());
        Ok(())
    }
}
