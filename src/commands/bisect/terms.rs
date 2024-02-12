
use clap::Parser;
use super::*;
use anyhow::Result;

/// Display the currently defined terms for `good` and  `bad`.
///
/// The terms `good` and `bad` can be redfined using options on the `svu bisect start` command.
/// This allows you to tailor the terms to the type of change that you are searching for.
/// Use `svu bisect terms` to display the current terms.
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
    after_help = "If no options are specified, then both terms are displayed.",
)]
pub struct Terms {
    /// Display the term for the 'good' subcommand
    #[arg(long)]
    term_good: bool,

    /// Display the term for the 'bad' subcommand
    #[arg(long)]
    term_bad: bool,
}

impl Terms {
    pub fn run(&mut self) -> Result<()> {
        let _ = svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let data = get_bisect_data()?;

        if self.term_good {
            println!("{}", data.good_name());
        } else if self.term_bad {
            println!("{}", data.bad_name());
        } else {
            println!("The term for the good state is {}", data.good_name().green());
            println!("The term for the bad  state is {}", data.bad_name().red());
            if let Some(status) = get_waiting_status(&data) {
                println!("{}", status);
            }
        }
        Ok(())
    }
}
