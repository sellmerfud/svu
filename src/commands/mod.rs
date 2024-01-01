
use clap::ArgMatches;

pub trait SvCommand {
    fn name(&self) -> &'static str;
    fn clap_command(&self) -> clap::Command;
    fn run(&self, matches: &ArgMatches) -> anyhow::Result<()>;
}

pub mod log;
pub mod branch;
pub mod show;
pub mod filerevs;
pub mod stash;
pub mod prefix;
pub mod ignore;

/// Return a vector of all of the sv subcommands.
pub fn sub_commands<'a>() -> Vec<&'a dyn SvCommand> {
    vec![
        &log::Log,
        &branch::Branch,
        &show::Show,
        &filerevs::FileRevs,
        &stash::Stash,
        &prefix::Prefix,
        &ignore::Ignore
    ]
}
