
use clap::{Command, Arg, ArgMatches};
use super::*;
use anyhow::Result;
use std::fs::remove_file;

pub struct Pop;

struct Options {
    stash_id:   usize,
    dry_run:    bool,
}

impl StashCommand for Pop {
    fn name(&self) -> &'static str { "pop" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Remove a stash entry and apply it to the working copy")
            .arg(
                Arg::new("dry-run")
                    .short('n')
                    .long("dry-run")
                    .action(clap::ArgAction::SetTrue)
                    .help("Show the patch output but do not update the working copy\n\
                    or remove the stash entry"),
            )
            .arg(
                Arg::new("stash-id")
                    .help("Id of the stash you wish to apply and drop.\n\
                           Can be 'stash-<n>' or simply <n> where <n> is the stash number.\n\
                           If omitted, stash-0 is applied by default.")
                    .value_parser(parse_stash_id)
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_pop(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    Options {
        stash_id:  matches.get_one::<usize>("stash-id").copied().unwrap_or(0),
        dry_run:   matches.get_flag("dry-run"),
    }
}

fn do_pop(options: &Options) -> Result<()> {
    svn::workingcopy_info()?;  // Make sure we are in a working copy.
    let mut stash_entries = load_stash_entries()?;
    if options.stash_id < stash_entries.len() {
        let stash = stash_entries.remove(options.stash_id);
        let wc_root = svn::workingcopy_root(Path::new(".")).unwrap();
        apply_stash(&stash, &wc_root, options.dry_run)?;

        if !options.dry_run {
            let patch_file = stash_path()?.join(stash.patch_name.as_str());
            save_stash_entries(&stash_entries)?;
            remove_file(patch_file)?;
            println!("Dropped stash: {}", stash.summary_display());
        }
        Ok(())
    }
    else {
        let msg = format!("{} does not exist in the stash", stash_id_display(options.stash_id));
        Err(General(msg).into())
    }
}
