
use clap::{Command, Arg, ArgMatches};
use super::*;
use anyhow::Result;

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
                       revision from before the bisect session.")
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

fn do_work(_options: &Options) -> Result<()> {
    svn::workingcopy_info()?;  // Make sure we are in a working copy.
    if true {
        Ok(())
    }
    else {
        Err(General("Failed..".to_string()).into())
    }
}
