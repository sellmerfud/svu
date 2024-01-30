

use regex::Regex;
use anyhow::Result;
use clap::Parser;
use crate::auth::Credentials;
use crate::svn::{self, LogEntry};
use crate::util;
use colored::*;
use chrono::{DateTime, Local};

/// Display formatted log entries
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
    after_help = "\
    By default shows only the first line of each commit message (see --full)\n\
    If only 1 revision is given and it is not a range then :0 is appended to make it a range.\n\
    If no revision is given and the first path looks like a revision it will be treated as one."
)]
pub struct Log {
    /// Limit the number of commits displayed
    #[arg(short, long, value_name = "NUM")]
    limit: Option<u32>,

    /// Display the author of each commit
    #[arg(short, long)]
    author: bool,

    /// Display the date of each commit
    #[arg(short, long)]
    date: bool,

    /// Display the date and time of each commit
    #[arg(short, long)]
    time: bool,

    /// Display the full commit message
    #[arg(short, long)]
    full: bool,

    /// Shorthand for --author --time --full
    #[arg(short, long)]
    verbose: bool,

    /// Display commits incoming with next update
    #[arg(short, long)]
    incoming: bool,

    /// Display the paths affected by each commit
    #[arg(short = 'p', long)]
    show_paths: bool,

    /// Do not cross copies while traversing history
    #[arg(long)]
    stop_on_copy: bool,

    /// Output the commits in the reverse order
    #[arg(long)]
    reverse: bool,

    /// Specify a revision or a range of revisions
    #[arg(short, long = "revision", value_name = "REV", num_args = 0.., conflicts_with = "incoming")]
    revisions: Vec<String>,

    /// Disply only commits with a matching message
    #[arg(short = 'm', long = "match", value_name = "REGEX", num_args = 0..)]
    regexes: Vec<Regex>,

    /// Limit commits to specific paths [default: .]
    #[arg(value_name = "PATH", num_args = 0..)]
    paths: Vec<String>,
}
impl Log {

    pub fn run(&mut self) -> Result<()> {

        // The incoming flag is a shortcut for -rHEAD:BASE
        if self.incoming {
            self.revisions = vec!["HEAD:BASE".to_owned()];
        }

        self.author = self.author || self.verbose;
        self.time = self.time || self.verbose;
        self.full = self.full || self.verbose;
        self.date = self.date && !self.time;

        if self.paths.is_empty() {
            self.paths.push(".".to_string());
        }

        self.show_results()
    }


    fn show_results(&self) -> Result<()> {

        fn parent_dir(path: &str) -> String {
            let re = Regex::new(r"^(.*)/[^/]+").expect("Error parsing parent_dir regex");
            let mut local_path = path.to_owned();
            local_path = local_path.trim_end_matches('/').to_owned();

            if let Some(caps) = re.captures(&local_path) {
                caps[1].to_owned()
            } else {
                ".".to_owned()
            }
        }


        let creds = crate::auth::get_credentials()?;

        let mut entries = self.get_log_entries(&creds)?;

        //  In the case where we are showing `incoming` commits
        //  we will have a single revision of "HEAD:BASE".
        //  It is possible that the "BASE" revision already exists
        //  in the working copy and thus will not be `incoming` so
        //  we do not want to display it.
        let omit_rev = 
            if !entries.is_empty() && self.revisions.len() == 1 && self.revisions[0] == "HEAD:BASE" {
                let wc_path = self.paths.first().map(|p| p.as_str()).unwrap_or(".");
                let path_info = svn::info(&creds, wc_path, None)?;
                if path_info.kind == "dir" {
                    Some(path_info.commit_rev)
                } else {
                    let parent_info = svn::info(&creds, &parent_dir(wc_path), None)?;
                    Some(parent_info.commit_rev)
                }
            } else {
                None
            };

        //  Get the length of the longest revision string and author name
        let (max_rev_len, max_author_len) = entries.iter().fold((0, 0), |(max_r, max_a), e| {
            (max_r.max(e.revision.len()), max_a.max(e.author.len()))
        });

        let build_prefix = |revision: &str, author: &str, date: &DateTime<Local>| -> String {

            let rev_str = format!("{:width$}", revision.yellow(), width=max_rev_len);
            let author_str = format!("{:width$}", author.cyan(), width=max_author_len);
            let date_str = if self.time {
                util::display_svn_datetime(date).magenta()
            } else {
                util::display_svn_date(date).magenta()
            };


            match (self.author, self.date || self.time) {
                (true, true) => format!("{} {} {}", rev_str, author_str, date_str),
                (true, false) => format!("{} {}", rev_str, author_str),
                (false, true) => format!("{} {}", rev_str, date_str),
                (false, false) => rev_str
            }
        };

        if self.reverse {
            entries.reverse();
        }

        for LogEntry { revision, author, date, msg, paths } in &entries {
            if Some(revision) != omit_rev.as_ref() {
                let msg_1st = msg.first().map(|s| s.as_str()).unwrap_or("");
                let prefix = build_prefix(revision, author, date);

                if self.full {
                    println!("\n{}", prefix);
                    for line in msg {
                        println!("{}", line);
                    }
                } else {
                    println!("{} {}", prefix, msg_1st);
                }

                if self.show_paths {
                    for path in paths {
                        println!("{}", util::formatted_log_path(path))
                    }
                }
            }
        }

        Ok(())
    }

    fn get_log_entries(&self, creds: &Option<Credentials>) -> Result<Vec<LogEntry>> {
        let mut revisions = self.revisions.clone();
        let mut paths = self.paths.clone();

        //  If no revisions are specified and the first 'path' looks like a revision
        //  then treat it as one, appending :0 if it does not have a range.
        if revisions.is_empty()
            && !paths.is_empty()
            && svn::looks_like_revision_range(paths[0].as_str()) {
            revisions = vec![paths.remove(0)];
        };

        //  Resolve any revisions that contains names such as HEAD or
        // that contain rev-3 type expressions.
        let resolve_path = paths.first().map(|p| p.as_str()).unwrap_or(".");
        let mut resolved_revs = Vec::new();
        for rev in &revisions {
            resolved_revs.push(svn::resolve_revision_range(creds, rev.as_str(), resolve_path)?);
        }

        if resolved_revs.len() == 1 && !resolved_revs[0].contains(':') {
            resolved_revs[0] = format!("{}:0", resolved_revs[0]);
        }

        let entries = svn::log(
            creds,
            &paths,
            &resolved_revs,
            true, // include_msg
            self.limit,
            self.stop_on_copy,
            self.show_paths,
        )?;

        //  Check any regular expressions entered by the user.
        //  Include the entry if it matches at least one of them.
        if self.regexes.is_empty() {
            Ok(entries)
        } else {
            let matching = |entry: &LogEntry| -> bool {
                let msg = entry.msg.join("\n");
                self.regexes.iter().any(|r| r.is_match(msg.as_str()))
            };
            let new_entries = entries.into_iter().filter(matching).collect();
            Ok(new_entries)
        }
    }
}



