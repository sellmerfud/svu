
use clap::Args;
use crate::svn;
use std::path::Path;
use super::*;
use anyhow::Result;
use uuid::Uuid;

/// Push the working copy to the stash and revert the working copy
#[derive(Debug, Args, Clone)]
pub struct PushArgs {
    /// A short description of the stash
    #[arg(short, long)]
    message: Option<String>,

    /// Include unversioned files in the stash
    #[arg(short, long)]
    unversioned: bool,

    /// Do not revert the working copy
    #[arg(short, long)]
    no_revert: bool,
}

#[derive(Debug)]
pub struct Push {
    args: PushArgs,
}


impl Push {
    pub fn new(args: PushArgs) -> Self {
        Push { args }
    }

    pub fn run(&mut self) -> Result<()> {

        let wc_info = svn::workingcopy_info()?; // Make sure we are in a working copy.
        let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
        let items = get_stash_items(&wc_root, self.args.unversioned)?;

        if items.is_empty() {
            println!("No local changes to save");
        } else {
            let (branch, revision) = svn::current_branch(&wc_root)?;
            let description = self
                .args
                .message
                .clone()
                .unwrap_or(get_log_message_1st(&wc_root)?);

            let stash_path = stash_path()?;
            let patch_name = create_patch_name();

            svn::create_patch(&stash_path.join(patch_name.as_str()), &wc_root)?;

            let stash = StashFileEntry {
                branch,
                revision,
                description,
                date: Local::now(),
                patch_name,
                items: items.clone(),
            };
            add_stash_entry(&stash)?;

            if !self.args.no_revert {
                // Lastly we revert the working copy.
                // We will explicitly revert all entries to ensure that the --remove-added flag is honored.
                // For added/unversioned directories we do not need to revert any entries below them
                // as these entries will be reverted recursively with their respective  directories.
                let added_unversioned: Vec<StashItem> = items
                    .iter()
                    .filter(|i| i.is_dir && (i.status == ADDED || i.status == UNVERSIONED))
                    .cloned()
                    .collect();
                let can_skip = |i: &StashItem| -> bool {
                    added_unversioned
                        .iter()
                        .any(|p| i.path.starts_with(&p.path) && i.path != p.path)
                };
                let revert_paths: Vec<String> = items
                    .iter()
                    .filter(|i| !can_skip(i))
                    .map(|i| i.path.clone())
                    .collect();
                svn::revert(&revert_paths, "infinity", true, Some(&wc_root))?;
            }

            println!("Saved working copy state - {}", stash.summary_display());
        }
        Ok(())
    }
}


fn get_log_message_1st(wc_root: &Path) -> Result<String> {
    let log = svn::log(
        &None,
        &[wc_root.to_string_lossy()],
        &["BASE:0".into()],
        true, Some(1),
        false,
        false
    )?;
    Ok(log[0].msg_1st())
}

fn create_patch_name() -> String {
    format!("{}.patch", Uuid::new_v4())
}

