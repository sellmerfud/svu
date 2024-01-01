
use clap::{Command, ArgMatches};
use super::StashCommand;
use anyhow::Result;

pub struct Drop;

impl StashCommand for Drop {
    fn name(&self) -> &'static str { "drop" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Remove a stash entry")
    }
        
    fn run(&self, _matches: &ArgMatches) -> Result<()> {
        println!("stash drop not yet implemented");
        Ok(())
        // show_results(&Options::build_options(matches))
    }
}
