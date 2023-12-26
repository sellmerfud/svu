
// use anyhow::Result;
use clap::Command;
use crate::commands::{sub_commands, SvCommand};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct App<'a> {
    pub commands: Vec<&'a dyn SvCommand>,
    pub clap: Command,
}

impl <'a> App<'a> {
    pub fn new() -> Self {
        let commands = sub_commands();
        let clap = Self::build_app(&commands);
        App { commands, clap }
    }

    fn build_app<'b>(subs: &'b Vec<&'b dyn SvCommand>) -> Command
    {
        let mut cmd = Command::new("svr")
            .version(VERSION)
            .after_help("The available commands are:\n\
                         log       Display formatted log entries\n\
                         branch    Display current branch or list branches\n\
                         show      Show the details of a given revision\n\
                         filerevs  Display commit revisions of files across tags and branches\n\
                         stash     Stash away changes to a dirty working copy\n\
                         bisect    Use binary search to find the commit that introduced a bug\n\
                         ignore    Write ignore properties to stdout in .gitignore format\n\
                         help      Display help information\n\
                         \n\
                         For help about a particular command type 'sv help <command>'");

        //  Add clap subcommmands
        for sub in subs.iter() {
            cmd = cmd.subcommand(sub.clap_command());
        }
        cmd
    }
}