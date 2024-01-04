
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
            .about("Subversion utilities")
            .after_help("For help about a particular command type 'svr help COMMAND'");

        //  Add clap subcommmands
        for sub in subs {
            cmd = cmd.subcommand(sub.clap_command());
        }
        cmd
    }
}