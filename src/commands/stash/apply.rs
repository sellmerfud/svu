
use clap::Parser;
use super::*;
use anyhow::Result;
use crate::svn;

/// Apply a stash to the working copy.
/// 
/// The stash entry is applied to the working copy but remains in the stash.
/// To apply the stash and remove the entry use svu stash pop.
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
        let wc_info = svn::workingcopy_info()?; // Make sure we are in a working copy.
        let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
        let stash_entries = load_stash_entries()?;

        if self.stash_id < stash_entries.len() {
            let stash = &stash_entries[self.stash_id];
            apply_stash(stash, &wc_root, self.dry_run)?;
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
