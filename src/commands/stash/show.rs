
use clap::{Command, Arg, ArgMatches};
use super::StashCommand;
use anyhow::Result;

pub struct Show;

impl StashCommand for Show {
    fn name(&self) -> &'static str { "show" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Show the details of a stash entry")
            .arg(
                Arg::new("diff")
                    .short('d')
                    .long("diff")
                    .action(clap::ArgAction::SetTrue)
                    .help("Display the patch file differences"),
            )
    }
        
    fn run(&self, _matches: &ArgMatches) -> Result<()> {
        println!("stash show not yet implemented");
        Ok(())
        // show_results(&Options::build_options(matches))
    }
}
