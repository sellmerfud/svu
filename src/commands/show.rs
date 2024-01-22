
use anyhow::Result;
use clap::Parser;
use crate::svn;
use crate::util;

/// Show the details of a commit
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
    after_help = "\
    The revision defaults to the current working copy revision.\n\
    If no revision is given and the first path looks like a revision it will be treated as one.\n\
    If no path is given it defaults to the current working copy directory."
)]    
pub struct Show {
    /// The commit revision
    #[arg(short, long, value_name = "REV")]
    revision:  Option<String>,
    
    /// Show the diff output of the commit
    #[arg(short = 'd', long)]
    show_diff: bool,

    /// Display the date of each commit
    #[arg(short = 'p', long)]
    show_paths: bool,

    /// Display the date and time of each commit
    #[arg(short, long)]
    no_message: bool,

    /// Limit commits to specific paths [default: .]
    #[arg(value_name = "PATH", num_args = 0..)]
    paths: Vec<String>,
}

impl Show {
    pub fn run(&mut self) -> Result<()> {
        let mut paths = self.paths.iter().map(|p| p.as_str()).collect::<Vec<&str>>();
        
        //  If no revisions are specified and the first 'path' looks like a revision
        //  then treat it as one, appending :0 if it does not have a range.
        if self.revision.is_none()  &&
           !paths.is_empty()   &&
           svn::looks_like_revision(paths[0]) {
            self.revision = Some(paths.remove(0).to_string());
        }
        
        if paths.is_empty() {
            paths.push(".");
        }

        let creds = crate::auth::get_credentials()?;

        //  Resolve the revision if necessary and coerce it into
        //  a vector
        //  In some cases when the revision is PREV, it may not produce a log entry 
        //  even though 'svn info' would succeed.  To work around this oddity
        //  we append :0 to the revision and limit the log to 0 entry.
        let mut rev_vector = Vec::<&str>::new();
        let mut resolved_rev: String;
        if let Some(rev) = &self.revision {
            resolved_rev = svn::resolve_revision_range(&creds, rev.as_str(), paths[0])?;
            resolved_rev += ":0";
            rev_vector.push(resolved_rev.as_str());
        }

        let log_entry = &svn::log(&creds, &paths, &rev_vector, true, Some(1), false, true)?[0];
        util::show_commit(&log_entry, !self.no_message, self.show_paths);
        if self.show_diff {            
            println!();
            let lines = svn::change_diff(paths[0], &log_entry.revision)?;
            for line in &lines {
                util::print_diff_line(line);
            }
        }
        Ok(())
    }
}
