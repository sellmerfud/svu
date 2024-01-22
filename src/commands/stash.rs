use anyhow::Result;
use chrono::{DateTime, Local};
use clap::{Parser, Subcommand};
use colored::Colorize;
use crate::util::SvError::*;
use std::borrow::Cow;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::fs::File;
use crate::svn;
use crate::util;
use std::fs::create_dir;
use serde::{Deserialize, Serialize};
use regex::Regex;
use pathdiff::diff_paths;
use std::env::current_dir;

mod push;
mod pop;
mod apply;
mod drop;
mod list;
mod show;
mod clear;

use push::PushArgs;

/// Stash away changes to a dirty working copy
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
    after_help = "\
    Save local changes to your working copy so that you can work\n\
    on something else and then merge the stashed changes back into\n\
    your working copy at a later time.\n\n\
    You can omit the COMMAND to quickly run the 'push' command."
)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = false)]
pub struct Stash {
    #[command(subcommand)]
    command: Option<StashCommands>,

    #[command(flatten)]
    push_args: PushArgs,
}

#[derive(Debug, Subcommand)]
enum StashCommands {
    Push(PushArgs),
    Pop(pop::Pop),
    Apply(apply::Apply),
    Drop(drop::Drop),
    List(list::List),
    Show(show::Show),
    Clear(clear::Clear),
}
use StashCommands::*;

impl Stash {

    pub fn run(&mut self) -> Result<()> {
        match &mut self.command {
            None             => push::Push::new(&self.push_args).run(),
            Some(Push(args)) => push::Push::new(&args).run(),
            Some(Pop(cmd))   => cmd.run(),
            Some(Apply(cmd)) => cmd.run(),
            Some(Drop(cmd))  => cmd.run(),
            Some(List(cmd))  => cmd.run(),
            Some(Show(cmd))  => cmd.run(),
            Some(Clear(cmd)) => cmd.run(),
        }
    }
}



fn parse_stash_id(arg: &str) -> Result<usize> {
    let re = Regex::new(r"^(?:stash-)?(\d+)$")?;
    if let Some(captures) = re.captures(arg) {
        let id = captures .get(1).unwrap() .as_str().parse::<usize>()?;
        Ok(id)
    }
    else {
        Err(General("Stash id must be 'stash-<n>' or '<n>'".to_string()).into())
    }
}

fn stash_id_display(id: usize) -> String {
    format!("stash-{}", id)
}


// Common structures and functions used by all of the stash commands.

fn stash_path() -> Result<PathBuf> {
    let path = util::data_directory()?.join("stash");

    if !path.is_dir() {
        create_dir(path.as_path())?
    }
    Ok(path)
}


fn stash_entries_file() -> Result<PathBuf> {
    Ok(stash_path()?.join("stash_entries.json"))
}

const UNVERSIONED: &'static str = "unversioned";
const NORMAL:      &'static str = "normal";
const ADDED:       &'static str = "added";
#[allow(dead_code)]
const DELETED:     &'static str = "deleted";
#[allow(dead_code)]
const MODIFIED:    &'static str = "modified";

// path for directories wil end with a slash/  (only happens for removed directories)
// revision of the file when it was modified (for display only)
// status will be one of: deleted, modified, added, unversioned
#[derive(Clone, Debug, Serialize, Deserialize)]
  struct StashItem {
    path:     String,
    revision: String,
    status:   String,
    #[serde(rename(serialize = "isDir", deserialize = "isDir"))]
    is_dir:   bool,
}

impl StashItem {
    fn status_letter<'a>(&'a self) -> &'static str {
        match self.status.as_str() {
            UNVERSIONED => "?",
            ADDED       => "A",
            DELETED     => "D",
            MODIFIED    => "M",
            _           => " ",
        }
    }

    fn status_color<'a>(&'a self) -> &'static str {
        match self.status.as_str() {
            UNVERSIONED => "white",
            ADDED       => "green",
            DELETED     => "red",
            MODIFIED    => "magenta",
            _           => "white",
        }
    }
}

use crate::util::datetime_serializer;
//  Stash entries saved to .sv/stash/stash_entries.json
#[derive(Clone, Debug, Serialize, Deserialize)]
struct StashFileEntry {
    branch:      String,
    revision:    String,
    description: String,
    #[serde(with = "datetime_serializer")]
    date:        DateTime<Local>,
    #[serde(rename(serialize = "patchName", deserialize = "patchName"))]
    patch_name:  String,
    items:       Vec<StashItem>,
}

impl StashFileEntry {
    fn summary_display(&self) -> String {
        format!("{} [{}]: {}", self.branch.green(), self.revision.yellow(), self.description)
    }

}

fn load_stash_entries() -> Result<Vec<StashFileEntry>> {
    let path = stash_entries_file()?;
    if path.is_file() {
        let reader = File::open(path)?;
        let entries: Vec<StashFileEntry> = serde_json::from_reader(reader)?;
        Ok(entries)
    } else {

        Ok(vec![])
    }
}


fn add_stash_entry(stash: &StashFileEntry) -> Result<()> {
    let mut entries = load_stash_entries()?;

    entries.insert(0, stash.clone());
    let writer = File::create(stash_entries_file()?)?;
    Ok(serde_json::to_writer_pretty(writer, &entries)?)
}

fn save_stash_entries(entries: &[StashFileEntry]) -> Result<()> {
    let writer = File::create(stash_entries_file()?)?;
    Ok(serde_json::to_writer_pretty(writer, entries)?)
}


//  Runs `svn status` on the working copy root directory
//  If we are not including unversioned items then we filter them out and build the list
//
//  If we are including unversioned items then it is a bit more complicated:
//  `svn status` will include unversioned directories but will not include their contents
//  So in this case we must add these unversioned directories to the working copy and then
//  run `svn status` a second time.  The add operation is recursive so we only need to
//  do it on the top level unversioned directories.
//  At this point `svn status` will return all of the previously unversioned items as
//  "added" so we must mark them as unversioned in our own item list.
//  So this function will alter the working copy when unversioned items are being stashed.
fn get_stash_items(wc_root: &Path, unversioned: bool) -> Result<Vec<StashItem>> {

    fn get_wc_items(wc_root: &Path, unversioned: bool) -> Result<Vec<StashItem>> {
        let status = svn::status(".", Some(wc_root))?;
        let mut items = Vec::<StashItem>::new();

        for entry in status.entries {
            if entry.item_status != NORMAL && (unversioned || entry.item_status != UNVERSIONED) {
                let is_dir = wc_root.join(&entry.path).is_dir();
                items.push(StashItem {
                    path:     entry.path,
                    revision: entry.revision,
                    status:   entry.item_status,
                    is_dir
                });
            }
        }
        Ok(items)
    }

    fn fixup_unversioned_items<'a>(initial_items: &'a Vec<StashItem>, wc_root: &Path) -> Result<Cow<'a, Vec<StashItem>>> {
        let unversioned_paths: Vec<String> = initial_items
            .iter()
            .filter(|i| i.status == UNVERSIONED)
            .map(|i| i.path.clone())
            .collect();
        
        if unversioned_paths.is_empty() {
            Ok(Cow::Borrowed(initial_items))
        }
        else {
            //  Any files inside of unversioned directories will not have been included when we
            //  ran `svn status``. So We run `svn add`` to recursively add all of the unversioned
            //  files/directories to the working copy.            
            //  If we had unversioned directories then we will need to fixup the status of the files/directories 
            //  that we just added.  We do this by running `svn status` again so that it picks up all of the
            //  new items.  But the unversioned items will now have a status of "added" so we must set those
            //  status values back to "unversioned" so we can restore the properly when the stash is reapplied.
            //  If there were no unversioned directores in the initial list then this is not necessary.

            svn::add(&unversioned_paths, &"infinity", false, Some(wc_root))?;

            if initial_items.iter().any(|i| i.is_dir && i.status == UNVERSIONED) {

                let new_items = get_wc_items(wc_root, false)?;
                let mut fixed_items = Vec::<StashItem>::new();
                for item in new_items {
                    if item.status == ADDED &&
                       unversioned_paths.iter().any(|p| item.path.starts_with(p.as_str())) {
                        fixed_items.push(StashItem { status: UNVERSIONED.to_string(), ..item});
                    }
                    else {
                        fixed_items.push(item)
                    }
                }
                Ok(Cow::Owned(fixed_items) )
            }
            else {
                Ok(Cow::Borrowed(initial_items))
            }
        }
    }

    match get_wc_items(wc_root, unversioned)? {
        items if unversioned => Ok(fixup_unversioned_items(&items, &wc_root)?.into_owned()),
        items => Ok(items)
    }
}

fn apply_stash(stash: &StashFileEntry, wc_root: &Path, dry_run: bool) -> Result<()> {
    let path_re    = Regex::new(r"^([ADUCG>])(\s+)(.+)$")?;
    let patch_file = stash_path()?.join(&stash.patch_name);
    let cwd        = current_dir()?;
    let stdout     = svn::apply_patch(&patch_file, dry_run, Some(&wc_root))?;
    let mut last_status = "".to_string();

    for line in stdout.lines() {
        let line = line?;
        if let Some(captures) = path_re.captures(line.as_str()) {
            // let x = &captures[1];
            let status = &captures[1];
            let space  = &captures[2];
            let path   = &captures[3];
            let rel_path = match status {
                ">" => path.to_string(), // Not a path
                _   => {
                    diff_paths(&wc_root.join(path), &cwd).unwrap().to_string_lossy().to_string()
                }
            };
            let color = match status {
                "C" => "red",
                "G" => "magenta",
                ">" if last_status == "C" => "red",
                ">" if last_status == "G" => "magenta",
                _ => "white",
            };
            let new_line = format!("{}{}{}", status, space, rel_path);
            println!("{}", new_line.color(color));
            last_status = status.to_string();
        }
        else {
            println!("{}", line);
        }
    }

    if !dry_run {
      // The working copy has been restored via the patch, but and files that were
      // `unversioned`` when the stash was created will not appear as `added``.
      // We must run `svn revert` on each unversioned item so that it will
      // once again become unversioned.
        let unversioned: Vec<StashItem> = stash.items.
            iter()
            .filter_map(|i| if i.status == UNVERSIONED { Some(i.clone()) } else { None })
            .collect();

        if !unversioned.is_empty() {
            let unversioned_dirs: Vec<String> = unversioned
                .iter()
                .filter_map(|i| if i.is_dir { Some(i.path.clone()) } else { None} )
                .collect();
            let can_skip = |i: &StashItem| -> bool {
                unversioned_dirs.iter().any(|d| i.path.starts_with(d)  && i.path != *d)
            };
            let revert_paths: Vec<String> = unversioned
                .iter()
                .filter_map(|i| if can_skip(i) { None } else { Some(i.path.clone()) })
                .collect();
            svn::revert(&revert_paths, "infinity", false, Some(&wc_root))?;
        }

        println!("Updated working copy state: {}", stash.summary_display());
    }
    Ok(())
}
