
use clap::Parser;
use super::*;
use anyhow::Result;
use std::fs::remove_file;

/// Clean up after a bisect session
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
    before_help = "\
    Update working copy to this revision.\n\
    If the revision is omitted, the working copy will be restored to its original\n\
    revision from before the bisect session. (also see --no-update)"
)]    
pub struct Reset {
    /// Do not update the working copy.
    #[arg(short, long, conflicts_with("revision"))]
    no_update: bool,
    
    /// Update working copy to this revision
    #[arg(value_name = "REVISION", num_args = 1..=1)]
    revision:  Option<String>,
}

impl Reset {
    pub fn run(&mut self) -> Result<()> {
        let creds = crate::auth::get_credentials()?;
        let wc_info = svn::workingcopy_info()?;  // Make sure we are in a working copy.
        let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
        let wc_path = wc_root.to_string_lossy();
    
        if let Some(data) = load_bisect_data()? {
            if !self.no_update {
                let revision = self.revision.as_ref()
                    .map(|r| svn::resolve_revision(&creds, &r, &wc_path))
                    .unwrap_or(Ok(data.original_rev))?;
                update_workingcopy(&revision)?;
            }
            else {
                let revision = wc_info.commit_rev;
                let msg      = get_1st_log_message(&revision)?;
                println!("Working copy: [{}] {}", revision.yellow(), msg);
            }
    
            remove_file(bisect_data_file()?)?;
            let path = bisect_log_file()?;
            if path.is_file() {
                remove_file(path)?;
            }
        }
        Ok(())
    }
}
