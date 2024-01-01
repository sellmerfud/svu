
use clap::{Command, Arg, ArgMatches};
use super::StashCommand;
use anyhow::Result;

pub struct Apply;

impl StashCommand for Apply {
    fn name(&self) -> &'static str { "apply" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Apply a stash entry to the working copy")
            .arg(
                Arg::new("dry-run")
                    .short('n')
                    .long("dry-run")
                    .action(clap::ArgAction::SetTrue)
                    .help("Show the patch output but do not update the working copy"),
            )
    }
        
    fn run(&self, _matches: &ArgMatches) -> Result<()> {
        println!("stash apply not yet implemented");
        Ok(())
        // show_results(&Options::build_options(matches))
    }
}
