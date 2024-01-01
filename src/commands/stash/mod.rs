use anyhow::Result;
use chrono::{DateTime, Local};
use clap::{Command, ArgMatches};
use colored::Colorize;
use crate::util::SvError::*;
use super::SvCommand;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::fs::File;
use crate::svn;
use crate::util;
use std::fs::create_dir;
use serde::{Deserialize, Serialize};

pub trait StashCommand {
    fn name(&self) -> &'static str;
    fn clap_command(&self) -> clap::Command;
    fn run(&self, matches: &ArgMatches) -> anyhow::Result<()>;
}

pub mod push;
pub mod list;
pub mod pop;
pub mod apply;
pub mod drop;
pub mod show;
pub mod clear;

/// Return a vector of all of the stash subcommands.
pub fn stash_commands<'a>() -> Vec<&'a dyn StashCommand> {
    vec![
        &push::Push,
        &list::List,
        &pop::Pop,
        &apply::Apply,
        &drop::Drop,
        &show::Show,
        &clear::Clear,
    ]
}


pub struct Stash;

impl SvCommand for Stash {
    fn name(&self) -> &'static str { "stash" }

    fn clap_command(&self) -> Command {
        let mut cmd = Command::new(self.name())
            .about("Stash away changes to a dirty working copy")
            .flatten_help(true)
            .before_help("Save local changes to your working copy so that you can work\n\
                          on something else and then merge the stashed changes back into\n\
                          your working copy at a later time.\n\n\
                          You can omit the COMMAND to quickly run the 'push' command.")
            // .args(push::Push::push_args().iter().map(|a| a.clone().hide(true)));
            .args(push::Push::push_args());

        //  Add clap subcommmands
        for sub in stash_commands() {
            cmd = cmd.subcommand(sub.clap_command());
        }
        cmd
             
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        match matches.subcommand() {
            Some((name, sub_matches)) => {
                if let Some(command) = stash_commands().iter().find(|cmd| cmd.name() == name) {
                    command.run(sub_matches)
                } else {
                    Err(General(format!("Fatal: stash command '{}' not found!", name)).into())
                }
            }
            None => {
                //  If no command given, use push command by default
                push::Push.run(matches)
            }
        }    
    }
}


// Common structure and functions used by all of the stash commands.

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
    fn path_display<'a>(&'a self) -> Cow<'a, str> {
        if self.is_dir {
            Cow::Owned(self.path.to_owned() + "/")
        } else {
            Cow::Borrowed(self.path.as_str())
        }
    }

    fn status_display<'a>(&'a self) -> String {
        match self.status.as_str() {
            UNVERSIONED => "?".to_string(),
            ADDED       => "A".green().to_string(),
            DELETED     => "D".red().to_string(),
            MODIFIED    => "M".magenta().to_string(),
            _           => " ".to_string()
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

// pub fn deserialize<'de, D>(date: DateTime<Local>, deserializer: D) -> Result<D::Ok>
//     where D: Deserializer<'de>
// {
//     let s = display_svn_datetime(&date);
//     deserializer.
//     deserializer.deserialize_str(&s)
// }

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


fn save_stash_entry(stash: &StashFileEntry) -> Result<()> {
    let mut entries = load_stash_entries()?;

    entries.insert(0, stash.clone());
    let writer = File::create(stash_entries_file()?)?;
    Ok(serde_json::to_writer_pretty(writer, &entries)?)
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
