
use clap::ArgMatches;
use std::path::Path;
use std::sync::OnceLock;

pub trait SvCommand {
    fn name(&self) -> &'static str;
    fn clap_command(&self) -> clap::Command;
    fn run(&self, matches: &ArgMatches) -> anyhow::Result<()>;
}

mod log;
mod branch;
mod show;
mod filerevs;
mod stash;
mod bisect;
mod prefix;
mod ignore;

/// Return a vector of all of the sv subcommands.
pub fn sub_commands<'a>() -> Vec<&'a dyn SvCommand> {
    vec![
        &log::Log,
        &branch::Branch,
        &show::Show,
        &filerevs::FileRevs,
        &stash::Stash,
        &bisect::Bisect,
        &prefix::Prefix,
        &ignore::Ignore
    ]
}

pub fn arg0() -> &'static String {
    static ARG0: OnceLock<String> = OnceLock::new();
    ARG0.get_or_init(|| {
        let full = std::env::args().take(1).collect::<String>();
        Path::new(&full).file_name().unwrap().to_string_lossy().to_string()
    })
}

