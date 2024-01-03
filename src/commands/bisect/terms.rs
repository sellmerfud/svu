
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
                    .conflicts_with("term-bad")
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
    Options {
        show_good: matches.get_one::<bool>("term-good").copied().unwrap_or(false),
        show_bad: matches.get_one::<bool>("term-bad").copied().unwrap_or(false),
    }
}

fn do_work(options: &Options) -> Result<()> {
    let _ = svn::workingcopy_info()?;  // Make sure we are in a working copy.
    let data = get_bisect_data()?;

    if options.show_good {
        println!("{}", data.good_name());
    }
    else if options.show_bad {
        println!("{}", data.bad_name());
    }
    else {
        println!("The term for the good state is {}", data.good_name().green());
        println!("The term for the bad  state is {}", data.bad_name().red());
        if let Some(status) = get_waiting_status(&data) {
            println!("{}", status);
        }
    }
    Ok(())
}
