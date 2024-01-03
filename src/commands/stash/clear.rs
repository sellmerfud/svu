
use clap::{Command, ArgMatches};
use super::StashCommand;
use anyhow::Result;
use crate::svn;
use super::*;
use std::fs::remove_file;

pub struct Clear;

impl StashCommand for Clear {
    fn name(&self) -> &'static str { "clear" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Remove all stash entries")
    }
        
    fn run(&self, _matches: &ArgMatches) -> Result<()> {

        svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let stash_entries_path = stash_entries_file()?;
        let stash_entries      = load_stash_entries()?; 

        // Remove all of the associated patch files
        for stash in &stash_entries {
            let patch_file = stash_path()?.join(stash.patch_name.as_str());
            remove_file(patch_file)?;
        }
        if stash_entries_path.is_file() {
            remove_file(stash_entries_file()?)?;
        }

        if stash_entries.len() == 0 {
            println!("No stash entries to clear");
        } else {
            println!("Cleared {} stash entries", stash_entries.len());
        }

        Ok(())
    }
}

