
use clap::{Command, ArgMatches};
use super::StashCommand;
use anyhow::Result;

pub struct Clear;

impl StashCommand for Clear {
    fn name(&self) -> &'static str { "clear" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Remove all stash entries")
    }
        
    fn run(&self, _matches: &ArgMatches) -> Result<()> {
        println!("stash clear not yet implemented");
        Ok(())
        // show_results(&Options::build_options(matches))
    }
}
