
use clap::Parser;
use anyhow::Result;
use crate::svn;
use super::*;
use std::fs::remove_file;

/// Remove all stash entries
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]
pub struct Clear;

impl Clear {
    pub fn run(&mut self) -> Result<()> {
        svn::workingcopy_info()?; // Make sure we are in a working copy.
        let stash_entries_path = stash_entries_file()?;
        let stash_entries = load_stash_entries()?;

        // Remove all of the associated patch files
        for stash in &stash_entries {
            let patch_file = stash_path()?.join(stash.patch_name.as_str());
            remove_file(patch_file)?;
        }
        if stash_entries_path.is_file() {
            remove_file(stash_entries_file()?)?;
        }

        if stash_entries.is_empty() {
            println!("No stash entries to clear");
        } else {
            println!("Cleared {} stash entries", stash_entries.len());
        }

        Ok(())
    }
}
