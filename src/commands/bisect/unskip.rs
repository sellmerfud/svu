
use clap::{Command, Arg, ArgMatches};
use super::*;
use anyhow::Result;

pub struct Unskip;

struct Options {
    revisions: Vec<String>,
}

impl BisectCommand for Unskip {
    fn name(&self) -> &'static str { "unskip" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Reinstate previously skipped revisions")
            .arg(
                Arg::new("revision")
                .help("Revision or range of revisions to unskip.\n\
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

fn do_work(_options: &Options) -> Result<()> {
    svn::working_copy_info()?;  // Make sure we are in a working copy.
    if true {
        Ok(())
    }
    else {
        Err(General("Failed..".to_string()).into())
    }
}
