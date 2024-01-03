
use clap::{Command, Arg, ArgMatches};
use super::*;
use anyhow::Result;

pub struct Replay;

struct Options {
    log_fiie: String,
}

impl BisectCommand for Replay {
    fn name(&self) -> &'static str { "replay" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Replay the bisect session from a log file")
            .arg(
                Arg::new("log_file")
                .help("Path to log file.")
                .value_name("FILE")
                .num_args(1..=1)
                .required(true)
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_work(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    Options {
        log_fiie: matches.get_one::<String>("log_file").unwrap().to_string(),
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
