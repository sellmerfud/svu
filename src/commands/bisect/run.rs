
use clap::{Command, Arg, ArgMatches};
use super::*;
use anyhow::Result;

pub struct Run;

struct Options {
    cmd:  String,
    args: Vec<String>,
}

impl BisectCommand for Run {
    fn name(&self) -> &'static str { "run" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Automate the bisect session by running a script")
            .after_help("Note that the script should exit with code 0 if the current source code is good,\n\
                         and exit with a code between 1 and 127 (inclusive), except 125, if the current source code is bad.\n\n\
                         Any other exit code will abort the bisect process. It should be noted that a program that terminates\n\
                        via exit(-1)\n\n\
                        The special exit code 125 should be used when the current source code cannot be tested. If the script\n\
                        exits with this code, the current revision will be skipped (see git bisect skip above). 125 was chosen\n\
                        as the highest sensible value to use for this purpose, because 126 and 127 are used by POSIX shells to\n\
                        signal specific error status (127 is for command not found, 126 is for command found but not executable\n\
                        these details do not matter, as they are normal errors in the script, as far as bisect run is concerned).")
            .arg(
                Arg::new("cmd")
                .help("Name of a command (script) to run")
                .value_name("CMD")
                .num_args(1..=1)
                .required(true)
            )
            .arg(
                Arg::new("args")
                .help("Command line arguments passed to 'cmd'")
                .value_name("ARG")
                .action(clap::ArgAction::Append)
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_work(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    let cmd  = matches.get_one::<String>("cmd").unwrap().to_string();
    let args = match matches.get_many::<String>("args") {
        Some(a) => a.map(|s| s.to_owned()).collect(),
        None => vec![]
    };

    Options {
        cmd,
        args,
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
