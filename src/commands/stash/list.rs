
use clap::{Command, ArgMatches};
use super::*;
use anyhow::Result;
use crate::svn;

pub struct List;

impl StashCommand for List {
    fn name(&self) -> &'static str { "list" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Display stash entries")
    }
        
    fn run(&self, _matches: &ArgMatches) -> Result<()> {
        svn::working_copy_info()?;  // Make sure we are in a working copy.

        for (index, stash) in load_stash_entries()?.iter().enumerate() {
            println!("{:<8} | {}", stash_id_display(index), stash.summary_display());
        }
        Ok(())
    }
}
