
use clap::{Command, Arg, ArgMatches};
use super::StashCommand;
use anyhow::Result;

pub struct Pop;

impl StashCommand for Pop {
    fn name(&self) -> &'static str { "pop" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Remove a stash entry and apply it to the working copy")
            .arg(
                Arg::new("dry-run")
                    .short('n')
                    .long("dry-run")
                    .action(clap::ArgAction::SetTrue)
                    .help("Show the patch output but do not update the working copy\n\
                    or remove the stash entry"),
            )
    }
        
    fn run(&self, _matches: &ArgMatches) -> Result<()> {
        println!("stash pop not yet implemented");
        Ok(())
        // show_results(&Options::build_options(matches))
    }
}
