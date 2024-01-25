
use std::fs::remove_file;
use std::collections::HashSet;
use clap::Parser;
use crate::util::display_svn_datetime;

use super::*;
use anyhow::Result;

/// Start a bisect session in the working copy
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]    
pub struct Start {
    /// A revision that is known to not contain the bug
    #[arg(short, long = "good", value_name = "REV")]
    good_rev: Option<String>,

    /// A revision that is known to contain the bug
    #[arg(short, long = "bad", value_name = "REV")]
    bad_rev: Option<String>,

    /// An alternate name for the 'good' subcommand
    #[arg(long, value_name = "TERM", value_parser = parse_term)]
    term_good: Option<String>,

    /// An alternate name for the 'bad' subcommand
    #[arg(long, value_name = "TERM", value_parser = parse_term)]
    term_bad: Option<String>,

}

impl Start {
    pub fn run(&mut self) -> Result<()> {
        let cmd_name: String = std::env::args().take(1).collect();
        let creds = crate::auth::get_credentials()?;
        let wc_info = svn::workingcopy_info()?;  // Make sure we are in a working copy.
        
        match load_bisect_data()? {
            Some(data) => {
                let mut stat = "".to_string();
                if let Some(msg) = get_waiting_status(&data) {
                    stat = format!("{}\n", msg);
                }
                let msg = format!("bisect session already in progress!\n{}\n\
                                  Use '{} bisect reset' to reset your working copy", stat, cmd_name);
                Err(General(msg).into())
            },
            None => {
                let good = self.good_rev.as_ref()
                    .map(|rev| svn::resolve_revision(&creds, &rev, "."))
                    .transpose()?;
                let bad = self.bad_rev.as_ref()
                    .map(|rev| svn::resolve_revision(&creds, &rev, "."))
                    .transpose()?;
    
                match (&good, &bad) {
                    (Some(g), Some(b)) if to_rev_num(g) == to_rev_num(b) =>
                        return Err(General("The 'good' and 'bad' revisions cannot be the same".to_string()).into()),
                    (Some(g), Some(b)) if to_rev_num(g) > to_rev_num(b) =>
                        return Err(General("The 'good' revision must be an ancestor of the 'bad' revision".to_string()).into()),
                    _ => ()
                }
    
                if self.term_good.is_some() && self.term_good == self.term_bad {
                    return Err(General("The 'good' and 'bad' terms cannot be the same.".to_string()).into())
                }

                let (head_rev, first_rev) = get_workingcopy_bounds()?;
                let data = BisectData {
                    local_path:   current_dir()?.to_string_lossy().to_string(),
                    original_rev: wc_info.commit_rev.clone(),
                    head_rev:     Some(head_rev),
                    first_rev:    Some(first_rev),
                    max_rev:      bad,
                    min_rev:      good,
                    skipped:      HashSet::new(),
                    term_good:    self.term_good.clone(),
                    term_bad:     self.term_bad.clone(),
                };
    
                save_bisect_data(&data)?;
                if let Ok(log_file) = bisect_log_file() {
                    if log_file.exists() {
                        remove_file(log_file)?;  // Remove any existing log file
                    }
                }
    
                append_to_log("#! /usr/bin/env sh\n")?;
                append_to_log(format!("# {} bisect log file {}", cmd_name, display_svn_datetime(&Local::now())))?;
                append_to_log(format!("# Initiated from: {}", current_dir()?.to_string_lossy()))?;
                append_to_log(format!("# {}", util::divider(72)))?;
                append_to_log("set -e\n")?;
                if let Some(rev) = &data.max_rev {
                    log_bisect_revision(rev, data.bad_name())?;
                }
                if let Some(rev) = &data.min_rev {
                    log_bisect_revision(rev, data.good_name())?;
                }
                if let Some(status) = get_waiting_status(&data) {
                    append_to_log(format!("# {}", status))?;
                    println!("{}", status);
                }
    
                if data.is_ready() {
                    perform_bisect(&data)?;
                }
                log_bisect_command(&std::env::args().collect::<Vec<String>>())?;
                Ok(())
            }
        }
    }
}

