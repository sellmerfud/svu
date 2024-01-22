
use clap::Parser;
use super::*;
use anyhow::Result;
use std::process;
use std::collections::HashSet;

/// Automate the bisect session by running a script
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
    after_help = "\
    Note that the script should exit with code 0 if the current source code is good,\n\
    and exit with a code between 1 and 127 (inclusive), except 125, if the current source code is bad.\n\n\
    Any other exit code will abort the bisect process. It should be noted that a program that terminates\n\
    via exit(-1) leaves $? = 255, (see the exit(3) manual page), as the value is chopped with & 0377.\n\n\
   The special exit code 125 should be used when the current source code cannot be tested. If the script\n\
   exits with this code, the current revision will be skipped (see git bisect skip above). 125 was chosen\n\
   as the highest sensible value to use for this purpose, because 126 and 127 are used by POSIX shells to\n\
   signal specific error status (127 is for command not found, 126 is for command found but not executable\n\
   these details do not matter, as they are normal errors in the script, as far as bisect run is concerned)."
)]    
pub struct Run {
    /// Name of a command (script) to run
    #[arg(value_name = "CMD", num_args = 1..=1, required = true)]
    cmd: String,

    /// Command line arguments passed to CMD
    #[arg(value_name = "ARG")]
    args: Vec<String>,
}

impl Run {
    pub fn run(&mut self) -> Result<()> {
        let _       = svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let wc_root = svn::workingcopy_root(&current_dir()?).unwrap();
        let data    = get_bisect_data()?;  // Make sure a bisect session has benn started
    
        if let Some(status) = get_waiting_status(&data) {
            println!("{}", status);
        }
    
        if !data.is_ready() {
            let msg = format!("'bisect run' cannot be used until a '{}' revision and a '{}' revision have been specified",
                data.good_name(), data.bad_name());
            Err(General(msg).into())
        }
        else {
            
            loop {
                let wc_info = svn::workingcopy_info()?;
                let data    = get_bisect_data()?;
                let cmd     = process::Command::new(self.cmd.as_str())
                .current_dir(&wc_root)
                .args(self.args.iter())
                .stdout(process::Stdio::inherit())
                .stderr(process::Stdio::inherit())
                .output()?;
        
                let exit_code = match cmd.status.code() {
                    Some(code) => code,
                    None => {
                        let msg = format!("Command '{}' failed to execute", self.cmd);
                        return Err(General(msg).into())
                    }
                };
                    
                match exit_code {
                    0 => {
                        display_command(&data.good_name());
                        let complete = mark_good_revision(&wc_info.commit_rev)?;
                        log_command(&data.good_name())?;
                        if complete { break }
                    }
                    125 => {
                        display_command("skip");
                        let mut revs = HashSet::new();
                        revs.insert(wc_info.commit_rev.clone());
                        let complete = mark_skipped_revisions(&revs)?;
                        log_command("skip")?;
                        if complete { break }
    
                    },
                    code if code < 128 => {
                        display_command(&data.bad_name());
                        let complete = mark_bad_revision(&wc_info.commit_rev)?;
                        log_command(&data.bad_name())?;
                        if complete { break }
                    }
                    code => {
                        let msg = format!("'bisect run' failed. Command '{}' returned unrecoverable error coce ({})",
                        self.cmd, code);
                        return Err(General(msg).into())
                    }
                }
            }
            Ok(())
        }
    }
}


fn display_command(name: &str) -> () {
    let cmd: String = std::env::args().take(1).collect();
    println!("{} bisect {}", cmd, name);
}

fn log_command(name: &str) -> Result<()> {
    let cmd: String = std::env::args().take(1).collect();
    let cmd_line = vec![cmd, "bisect".to_string(), name.to_string()];
    log_bisect_command(&cmd_line)?;
    Ok(())
}
