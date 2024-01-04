
use clap::{Command, Arg, ArgMatches};
use crate::svn::resolve_revision;
use super::*;
use anyhow::Result;
use std::fs::remove_file;
use std::env::current_dir;

pub struct Reset;

struct Options {
    revision:  Option<String>,
    no_update: bool,
}

impl BisectCommand for Reset {
    fn name(&self) -> &'static str { "reset" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Clean up after a bisect session")
            .arg(
                Arg::new("revision")
                .help("Update working copy to this revision.\n\
                       If omitted, the working copy will be restored to its original\n\
                       revision from before the bisect session. (also see --no-update")
                .value_name("REVISION")
                .num_args(1..=1)
            )
            .arg(
                Arg::new("no-update")
                .help("Do not update the working copy.\n\
                       It will remain in its current state.")
                .short('n')
                .long("no-update")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("revision")
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_work(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    Options {
        revision: matches.get_one::<String>("revision").map(|s| s.to_string()),
        no_update: matches.get_flag("no-update"),
    }
}

fn do_work(options: &Options) -> Result<()> {
    let wc_info = svn::workingcopy_info()?;  // Make sure we are in a working copy.
    let wc_root = svn::workingcopy_root(&current_dir()?).unwrap();
    let wc_path = wc_root.to_string_lossy();

    if let Some(data) = load_bisect_data()? {
        if !options.no_update {
            let revision = options.revision.as_ref()
                .map(|r| resolve_revision(&r, &wc_path))
                .unwrap_or(Ok(data.original_rev))?;
            update_workingcopy(&revision)?;
        }
        else {
            let revision = wc_info.commit_rev;
            let msg      = get_1st_log_message(&revision)?;
            println!("Working copy: [{}] {}", revision.yellow(), msg);
        }

        remove_file(bisect_data_file()?)?;
        let path = bisect_log_file()?;
        if path.is_file() {
            remove_file(path)?;
        }
    }
    Ok(())
}
