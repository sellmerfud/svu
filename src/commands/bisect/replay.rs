
use clap::Parser;
use super::*;
use anyhow::Result;
use std::process;

/// Replay a bisect session from a log file
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]    
pub struct Replay {
    /// Path to log file
    #[arg(num_args = 1..=1, required = true)]
    log_fiie: String,
}

impl Replay {
    pub fn run(&mut self) -> Result<()> {
        svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let wc_root = svn::workingcopy_root(&current_dir()?).unwrap();
        let mut args = Vec::new();
        args.push(self.log_fiie.clone());
    
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
}
