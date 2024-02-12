
use clap::Parser;
use anyhow::Result;

use crate::commands::*;

pub(crate) const HELP_TEMPLATE: &str = "\
{name}  v{version}

{about}

{usage-heading}
{tab}{usage}

{all-args}{after-help}";


pub trait Run {
    fn run(&mut self) -> Result<()>;
}

/// Subversion utilities
///
/// A collection of useful commands for working with Subversion repositories.
#[derive(Debug, Parser)]
#[command(
    about,
    long_about,
    author,
    help_template = HELP_TEMPLATE,
    propagate_version = true,
    infer_subcommands = true,
    version,
)]

pub enum Commands {
    Log(log::Log),
    Branch(branch::Branch),
    Show(show::Show),
    Filerevs(filerevs::Filerevs),
    Stash(stash::Stash),
    Bisect(bisect::Bisect),
    Prefix(prefix::Prefix),
    Ignore(ignore::Ignore),
}

use Commands::*;

impl Run for Commands{
    fn run(&mut self) -> Result<()> {
        match self {
            Log(cmd)      => cmd.run(),
            Branch(cmd)   => cmd.run(),
            Show(cmd)     => cmd.run(),
            Filerevs(cmd) => cmd.run(),
            Stash(cmd)    => cmd.run(),
            Bisect(cmd)   => cmd.run(),
            Prefix(cmd)   => cmd.run(),
            Ignore(cmd)   => cmd.run(),
        }
    }
}
