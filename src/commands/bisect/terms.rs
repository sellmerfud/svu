
use clap::{Command, Arg, ArgMatches};
use super::*;
use anyhow::Result;

pub struct Terms;

struct Options {
    show_good: bool,
    show_bad:  bool
}

impl BisectCommand for Terms {
    fn name(&self) -> &'static str { "terms" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Display the currently defined terms for good/bad")
            .after_help("If no options are specified, then both terms are displayed")
            .arg(
                Arg::new("term-good")
                    .help("Display the term for the 'good' subcommand")
                    .long("term-good")
                    .action(clap::ArgAction::SetTrue)
            )
            .arg(
                Arg::new("term-bad")
                    .help("Display the term for the 'bad' subcommand")
                    .long("term-bad")
                    .action(clap::ArgAction::SetTrue)
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_work(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    let neither = !(matches.get_flag("term-good") || matches.get_flag("term-bad"));
    Options {
        show_good: matches.get_flag("term-good") || neither,
        show_bad: matches.get_flag("term-bad")   || neither,
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
