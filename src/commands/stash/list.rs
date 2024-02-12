
use clap::Parser;
use super::*;
use anyhow::Result;
use crate::svn;

/// Display stash entries.
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]
pub struct List;

impl List {
    pub fn run(&mut self) -> Result<()> {
        svn::workingcopy_info()?; // Make sure we are in a working copy.

        for (index, stash) in load_stash_entries()?.iter().enumerate() {
            println!(
                "{:<8} | {}",
                stash_id_display(index),
                stash.summary_display()
            );
        }
        Ok(())
    }
}
