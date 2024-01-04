
use clap::{Command, Arg, ArgMatches};
use super::*;
use anyhow::Result;
use std::process;

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

fn do_work(options: &Options) -> Result<()> {
    svn::workingcopy_info()?;  // Make sure we are in a working copy.
    let wc_root = svn::workingcopy_root(&current_dir()?).unwrap();
    let mut args = Vec::new();
    args.push("-c".to_string());
    args.push(options.log_fiie.clone());

    let cmd = process::Command::new("/bin/sh")
        .current_dir(wc_root)
        .args(args)
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit())
        .output()?;

    if cmd.status.success() {
        Ok(())
    }
    else {
        Err(General("Log replay did not finish successfully".to_string()).into())
    }

}
