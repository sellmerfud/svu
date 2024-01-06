
use clap::{Command, Arg, ArgMatches};
use crate::svn;

use super::*;
use anyhow::Result;
use std::collections::HashSet;

pub struct Skip;

struct Options {
    revisions: Vec<String>,
}

impl BisectCommand for Skip {
    fn name(&self) -> &'static str { "skip" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Skip revisions.  They will no longer be considered")
            .arg(
                Arg::new("revision")
                .help("Revision or range of revisions to skip.\n\
                       May be specified mulitple times.\n\
                       If not specified, then the current working copy\n\
                       revision is used.")
                .value_name("REVISION|REV:REV")
                .action(clap::ArgAction::Append)
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_work(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    let revisions = match matches.get_many::<String>("revision") {
        Some(revs) => revs.map(|s| s.to_owned()).collect(),
        None => vec![]
    };

    Options {
        revisions,
    }
}


fn do_work(options: &Options) -> Result<()> {
    let wc_info = svn::workingcopy_info()?;  // Make sure we are in a working copy.
    let wc_root = svn::workingcopy_root(&current_dir()?).unwrap();
    let wc_root_path = wc_root.to_string_lossy();
    let _ = get_bisect_data()?;  // Ensure a bisect session has started

    let mut skipped = HashSet::<String>::new();
    for rev in &options.revisions {
        skipped.extend(gather_revisions(rev, &wc_root_path)?);
    }
    //  If not revisions specified, use the working copy rev
    if skipped.is_empty() {
        skipped.insert(wc_info.commit_rev.clone());
    }

    mark_skipped_revisions(&skipped)?;
    log_bisect_command(&std::env::args().collect::<Vec<String>>())?;

    let data = get_bisect_data()?; // Fresh copy of data
    if let Some(status) = get_waiting_status(&data) {
        append_to_log(format!("# {}", status))?;
        println!("{}", status);
    }

    Ok(())
}
