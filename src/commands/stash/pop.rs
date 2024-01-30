
use clap::Parser;
use super::*;
use anyhow::Result;
use std::fs::remove_file;

/// Remove a stash entry and apply it to the working copy
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]
pub struct Pop {
    /// Show the patch output but do not update the working copy
    /// or remove the stash entry
    #[arg(short = 'n', long)]
    dry_run: bool,

    /// Id of the stash you wish to apply and drop
    #[arg(value_name = "STASH", value_parser = parse_stash_id, default_value = "stash-0")]
    stash_id: usize,
}

impl Pop {
    pub fn run(&mut self) -> Result<()> {
        let wc_info = svn::workingcopy_info()?; // Make sure we are in a working copy.
        let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
        let mut stash_entries = load_stash_entries()?;
        if self.stash_id < stash_entries.len() {
            let stash = stash_entries.remove(self.stash_id);
            apply_stash(&stash, &wc_root, self.dry_run)?;

            if !self.dry_run {
                let patch_file = stash_path()?.join(stash.patch_name.as_str());
                save_stash_entries(&stash_entries)?;
                remove_file(patch_file)?;
                println!("Dropped stash: {}", stash.summary_display());
            }
            Ok(())
        } else {
            let msg = format!(
                "{} does not exist in the stash",
                stash_id_display(self.stash_id)
            );
            Err(General(msg).into())
        }
    }
}
