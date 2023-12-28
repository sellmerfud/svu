
use clap::ArgMatches;

pub trait SvCommand {
    fn name(&self) -> &'static str;
    fn clap_command(&self) -> clap::Command;
    fn run(&self, matches: &ArgMatches) -> anyhow::Result<()>;
}

pub mod log;
pub mod show;
pub mod ignore;

/// Return a vector of all of the svn subcommands.
pub fn sub_commands<'a>() -> Vec<&'a dyn SvCommand> {
    vec![
        &log::Log,
        &show::Show,
        &ignore::Ignore
    ]
}