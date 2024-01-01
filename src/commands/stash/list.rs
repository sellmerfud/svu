
use clap::{Command, ArgMatches};
use super::StashCommand;
use anyhow::Result;

pub struct List;

impl StashCommand for List {
    fn name(&self) -> &'static str { "list" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Display stash entries")
    }
        
    fn run(&self, _matches: &ArgMatches) -> Result<()> {
        println!("stash list not yet implemented");
        Ok(())
        // show_results(&Options::build_options(matches))
    }
}
