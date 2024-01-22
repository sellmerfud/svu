
use clap::Parser;
use anyhow::Result;
use super::*;
use std::fs::remove_file;


/// Remove a stash entry
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]    
pub struct Drop {
    /// Id of the stash you wish to apply and drop
    #[arg(value_name = "STASH", value_parser = parse_stash_id, default_value = "stash-0")]
    stash_id: usize,
}

impl Drop {
    pub fn run(&mut self) -> Result<()> {
        svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let mut stash_entries = load_stash_entries()?;
        if self.stash_id < stash_entries.len() {
            let stash = stash_entries.remove(self.stash_id);
            let patch_file = stash_path()?.join(stash.patch_name.as_str());
            save_stash_entries(&stash_entries)?;
            remove_file(patch_file)?;
            println!("Dropped stash: {}", stash.summary_display());
            Ok(())
        }
        else {
            let msg = format!("{} does not exist in the stash", stash_id_display(self.stash_id));
            Err(General(msg).into())
        }
    }
}
