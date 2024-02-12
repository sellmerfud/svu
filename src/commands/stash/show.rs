
use clap::Parser;
use super::*;
use anyhow::Result;
use crate::util::{display_svn_datetime, print_diff_line};
use crate::svn;
use std::env::current_dir;
use std::io::{BufReader, BufRead};
use pathdiff::diff_paths;

/// Display the details of a stash entry.
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]
pub struct Show {
    /// Show contents of the the patch that was used to create the stash entry.
    #[arg(short = 'd', long)]
    show_diff: bool,

    /// Id of the stash entry you wish to display.
    #[arg(value_name = "STASH", value_parser = parse_stash_id, default_value = "stash-0")]
    stash_id: usize,
}

impl Show {
    pub fn run(&mut self) -> Result<()> {

        let wc_info = svn::workingcopy_info()?; // Make sure we are in a working copy.
        let stash_entries = load_stash_entries()?;

        if self.stash_id < stash_entries.len() {
            let cwd = current_dir()?;
            let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
            let stash = &stash_entries[self.stash_id];
            let patch_file = stash_path()?.join(stash.patch_name.as_str());
            let rel_patch = diff_paths(&patch_file, &cwd).unwrap();

            println!(
                "{:<11}| {}",
                stash_id_display(self.stash_id),
                stash.summary_display()
            );
            println!(
                "{:<11}| {}",
                "created",
                display_svn_datetime(&stash.date).magenta()
            );
            println!(
                "{:<11}| {}",
                "patch file",
                rel_patch.to_string_lossy().blue()
            );
            println!("{:->70}", "-");
            for item in &stash.items {
                let mut pathname = item.path.clone();
                //  append '/' for directories
                if item.is_dir {
                    pathname.push('/');
                }
                let path = Path::new(pathname.as_str());
                // First create the full path to the item relative to the working copy root.
                // Then make that relative to the current working directory.
                let rel_path = diff_paths(&wc_root.join(path), &cwd).unwrap();
                let revision = match item.status.as_str() {
                    UNVERSIONED => "unversioned",
                    ADDED => "new",
                    _ => item.revision.as_str(),
                };
                let color = item.status_color();
                println!(
                    "{} {} [{}]",
                    item.status_letter().color(color),
                    rel_path.to_string_lossy().color(color),
                    revision.yellow()
                );
            }

            if self.show_diff {
                println!();
                let file = File::open(patch_file)?;
                for line in BufReader::new(file).lines() {
                    print_diff_line(line?.as_str());
                }
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
