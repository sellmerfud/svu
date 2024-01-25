
use clap::Parser;
use super::*;
use anyhow::Result;
use std::sync::OnceLock;


pub fn bad_term() -> &'static String {
    static ARG0: OnceLock<String> = OnceLock::new();
    ARG0.get_or_init(|| {
        load_bisect_data()
            .ok()
            .flatten()
            .map(|data| data.term_bad)
            .flatten()
            .unwrap_or("".to_owned())
    })
}

fn get_bad_term() -> Option<&'static str> {
    let term = bad_term();
    if term.is_empty() { None } else { Some(term) }
}

/// Mark a revision as bad  (It contains the bug)
#[derive(Debug, Parser)]
#[command(
    author,
    visible_alias = get_bad_term(),
    help_template = crate::app::HELP_TEMPLATE,
)]    
pub struct Bad {
    /// The bad revision. If omitted use the current working copy revison
    #[arg(value_name = "REV")]
    revision: Option<String>,
}

impl Bad {
    pub fn run(&mut self) -> Result<()> {
        let creds = crate::auth::get_credentials()?;
        let wc_info = svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
        let data = get_bisect_data()?;
        let revision = match &self.revision {
            Some(rev) => svn::resolve_revision(&creds, &rev, &wc_root.to_string_lossy().as_ref())?,
            None      => wc_info.commit_rev,
        };
    
          // The new bad revision can come after the existing maxRev
          // This allows the user to recheck a range of commits.
          // The new bad revision cannot be less than or equal to the minRev
        if data.min_rev.is_some() && to_rev_num(&revision) <= to_rev_num(&data.min_rev.as_ref().unwrap())  {
            println!("The '{}' revision must be more recent than the '{}' revision", data.bad_name(), data.good_name());
        }
        else {
            let _ = mark_bad_revision(&revision);
            log_bisect_command(&std::env::args().collect::<Vec<String>>())?;
        }
        
        let data = get_bisect_data()?; // Fresh copy of data
        if let Some(status) = get_waiting_status(&data) {
            append_to_log(format!("# {}", status))?;
            println!("{}", status);
        }
        Ok(())
    }
}
