
use clap::Parser;
use super::*;
use anyhow::Result;
use std::collections::HashSet;

/// Reinstate previously skipped revisions
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
    after_help = "If no revision is specified, the current working copy revision is unskipped."
)]    
pub struct Unskip {
    /// Revision or range of revisions to skip.
    #[arg(value_name = "REV|REV:REV")]
    revisions: Vec<String>,
}

impl Unskip {
    pub fn run(&mut self) -> Result<()> {
        let creds = crate::auth::get_credentials()?;
        let wc_info = svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
        let wc_root_str = wc_root.to_string_lossy();
        let _ = get_bisect_data()?;  // Ensure a bisect session has started
    
        let mut skipped = HashSet::<String>::new();
        for rev in &self.revisions {
            skipped.extend(gather_revisions(&creds, rev, &wc_root_str)?);
        }
        //  If not revisions specified, use the working copy rev
        if skipped.is_empty() {
            skipped.insert(wc_info.commit_rev.clone());
        }
    
        mark_unskipped_revisions(&skipped)?;
        log_bisect_command(&std::env::args().collect::<Vec<String>>())?;
    
        let data = get_bisect_data()?; // Fresh copy of data
        if let Some(status) = get_waiting_status(&data) {
            append_to_log(format!("# {}", status))?;
            println!("{}", status);
        }
        Ok(())
    }
}
