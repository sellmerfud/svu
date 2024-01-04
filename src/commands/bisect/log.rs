
use clap::{Command, ArgMatches};
use super::*;
use anyhow::Result;

pub struct Log;

impl BisectCommand for Log {
    fn name(&self) -> &'static str { "log" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Show the bisect log")
    }
        
    fn run(&self, _matches: &ArgMatches) -> Result<()> {
        do_work()?;
        Ok(())
    }
}


fn do_work() -> Result<()> {
    let _ = svn::workingcopy_info()?;  // Make sure we are in a working copy.
    let _ = get_bisect_data()?;        // Ensure a bisect session has started
    display_log()?;
    Ok(())
}
