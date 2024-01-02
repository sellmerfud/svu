
use clap::{Command, Arg, ArgMatches};
use super::StashCommand;
use anyhow::Result;
use super::*;
use std::fs::remove_file;

pub struct Drop;

struct Options {
    stash_id:   usize,
}



impl StashCommand for Drop {
    fn name(&self) -> &'static str { "drop" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Remove a stash entry")
            .arg(
                Arg::new("stash-id")
                    .help("Id of the stash you wish to drop.\n\
                           Can be 'stash-<n>' or simply <n> where <n> is the stash number.\n\
                           If omitted, stash-0 is dropped by default.")
                    .value_parser(parse_stash_id)
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_drop(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    Options {
        stash_id:  matches.get_one::<usize>("stash-id").copied().unwrap_or(0)
    }
}

fn do_drop(options: &Options) -> Result<()> {
    svn::working_copy_info()?;  // Make sure we are in a working copy.
    let mut stash_entries = load_stash_entries()?;
    if options.stash_id < stash_entries.len() {
        let stash = stash_entries.remove(options.stash_id);
        let patch_file = stash_path()?.join(stash.patch_name.as_str());
        save_stash_entries(&stash_entries)?;
        remove_file(patch_file)?;
        println!("Dropped stash: {}", stash.summary_display());
        Ok(())
    }
    else {
        let msg = format!("{} does not exist in the stash", stash_id_display(options.stash_id));
        Err(General(msg).into())
    }

}