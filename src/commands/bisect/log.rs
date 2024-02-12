
use clap::Parser;
use super::*;
use anyhow::Result;

/// Print the bisect log to stdout.
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]
pub struct Log;

impl Log {
    pub fn run(&mut self) -> Result<()> {
        let _ = svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let _ = get_bisect_data()?;        // Ensure a bisect session has started
        display_log()?;
        Ok(())
    }
}
