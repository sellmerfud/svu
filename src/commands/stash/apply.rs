
use clap::Parser;
use super::*;
use anyhow::Result;
use crate::svn;

/// Apply a stash to the working copy
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]    
pub struct Apply {
    /// Show the patch output but do not update the working copy
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Id of the stash you wish to apply
    #[arg(value_name = "STASH", value_parser = parse_stash_id, default_value = "stash-0")]
    stash_id: usize,
}

impl Apply {
    pub fn run(&mut self) -> Result<()> {
        svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let stash_entries = load_stash_entries()?;
    
        if self.stash_id < stash_entries.len() {
            let stash = &stash_entries[self.stash_id];
            let wc_root = svn::workingcopy_root(Path::new(".")).unwrap();
            apply_stash(stash, &wc_root, self.dry_run)?;
            Ok(())
        }
        else {
            let msg = format!("{} does not exist in the stash", stash_id_display(self.stash_id));
            Err(General(msg).into())
        }
    }
}
