
use clap::{Command, Arg, ArgMatches};
use super::*;
use anyhow::Result;
use crate::svn;

pub struct Apply;

struct Options {
    stash_id:   usize,
    dry_run:    bool,
}

impl StashCommand for Apply {
    fn name(&self) -> &'static str { "apply" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Apply a stash entry to the working copy")
            .arg(
                Arg::new("dry-run")
                    .short('n')
                    .long("dry-run")
                    .action(clap::ArgAction::SetTrue)
                    .help("Show the patch output but do not update the working copy"),
            )
            .arg(
                Arg::new("stash-id")
                    .help("Id of the stash you wish to apply.\n\
                           Can be 'stash-<n>' or simply <n> where <n> is the stash number.\n\
                           If omitted, stash-0 is applied by default.")
                    .value_parser(parse_stash_id)
            )
}
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_apply(&build_options(matches))?;
        Ok(())
    }
}


fn build_options(matches: &ArgMatches) -> Options {
    Options {
        stash_id:  matches.get_one::<usize>("stash-id").copied().unwrap_or(0),
        dry_run:   matches.get_flag("dry-run"),
    }
}

fn do_apply(options: &Options) -> Result<()> {
    svn::working_copy_info()?;  // Make sure we are in a working copy.
    let stash_entries = load_stash_entries()?;

    if options.stash_id < stash_entries.len() {
        let stash = &stash_entries[options.stash_id];
        let wc_root = svn::workingcopy_root(Path::new(".")).unwrap();
        apply_stash(stash, &wc_root, options.dry_run)?;
        Ok(())
    }
    else {
        let msg = format!("{} does not exist in the stash", stash_id_display(options.stash_id));
        Err(General(msg).into())
    }
}
