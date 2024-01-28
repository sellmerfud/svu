
use clap::Parser;
use super::*;
use anyhow::Result;
use std::sync::OnceLock;


pub fn good_term() -> &'static String {
    static ARG0: OnceLock<String> = OnceLock::new();
    ARG0.get_or_init(|| {
        load_bisect_data()
            .ok()
            .flatten()
            .and_then(|data| data.term_good)
            .unwrap_or("".to_owned())
    })
}

fn get_good_term() -> Option<&'static str> {
    let term = good_term();
    if term.is_empty() { None } else { Some(term) }
}


/// Mark a revision as good  (It does not contain the bug)
#[derive(Debug, Parser)]
#[command(
    author,
    visible_alias = get_good_term(),
    help_template = crate::app::HELP_TEMPLATE,
)]    
pub struct Good {
    /// The good revision. If omitted use the current working copy revison
    #[arg(value_name = "REV")]
    revision: Option<String>,
}

impl Good {
    pub fn run(&mut self) -> Result<()> {
        let creds = crate::auth::get_credentials()?;
        let wc_info = svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
        let data = get_bisect_data()?;
        let revision = match &self.revision {
            Some(rev) => svn::resolve_revision(&creds, rev, wc_root.to_string_lossy().as_ref())?,
            None      => wc_info.commit_rev,
        };
    
        // The new good revision can come before the exisiing minRev
        // This allow the user to recheck a range of commits.
        // The new good revision cannot be greater than or equal to the maxRev
        if data.max_rev.is_some() && to_rev_num(&revision) >= to_rev_num(data.max_rev.as_ref().unwrap())  {
            println!("The '{}' revision must be older than the '{}' revision", data.good_name(), data.bad_name());
        }
        else {
            let _ = mark_good_revision(&revision);
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
