
use std::{usize, fs::remove_file};

use clap::{Command, Arg, ArgMatches};
use crate::util::display_svn_datetime;

use super::*;
use anyhow::Result;

pub struct Start;

struct Options {
    good_rev:  Option<String>,
    bad_rev:   Option<String>,
    term_good: Option<String>,
    term_bad:  Option<String>,
}

impl BisectCommand for Start {
    fn name(&self) -> &'static str { "start" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Start a bisect session in the working copy")
            .arg(
                Arg::new("good-rev")
                    .help("A revision that is known to not contain the bug.\n\
                           If not specifed, you must used the 'bisect good' subcommand\n\
                           to specify the good revision.")
              .value_name("revision")
                    .short('g')
                    .long("good")
            )
            .arg(
                Arg::new("bad-rev")
                    .help("A revision that is known to contain the bug.\n\
                          If not specifed, you must used the 'bisect bad' subcommand\n\
                          to specify the bad revision.")
                    .value_name("revision")
                    .short('b')
                    .long("bad")
            )
            .arg(
                Arg::new("term-good")
                    .help("An alternate name for the 'good' subcommand")
                    .value_name("term")
                    .long("term-good")
                    .value_parser(parse_term)
            )
            .arg(
                Arg::new("term-bad")
                    .help("An alternate name for the 'bad' subcommand")
                    .value_name("term")
                    .long("term-bad")
                    .value_parser(parse_term)
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_work(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    Options {
        good_rev:  matches.get_one::<String>("good-rev").map(|s| s.to_string()),
        bad_rev:   matches.get_one::<String>("bad-rev").map(|s| s.to_string()),
        term_good: matches.get_one::<String>("term-good").map(|s| s.to_string()),
        term_bad:  matches.get_one::<String>("term-bad").map(|s| s.to_string()),
    }
}

fn do_work(options: &Options) -> Result<()> {
    let cmd_name: String = std::env::args().take(1).collect();
    let wc_info = svn::working_copy_info()?;  // Make sure we are in a working copy.
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
            // if let Some(rev)
            let good = options.good_rev
                .as_ref()
                .map(|rev| svn::resolve_revision_string(&rev, "."))
                .map_or(Ok(None), |v| v.map(Some))?;
            let bad = options.bad_rev
                .as_ref()
                .map(|rev| svn::resolve_revision_string(&rev, "."))
                .map_or(Ok(None), |v| v.map(Some))?;

            match (&good, &bad) {
                (Some(g), Some(b)) if g == b =>
                    return Err(General("The 'good' and 'bad' revisions cannot be the same".to_string()).into()),
                (Some(g), Some(b)) if g.parse::<usize>()? > b.parse::<usize>()? =>
                    return Err(General("The 'good' revision must be an ancestor of the 'bad' revision".to_string()).into()),
                _ => ()
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
                term_good:    options.term_good.clone(),
                term_bad:     options.term_bad.clone(),
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
            append_to_log("-------------------------------------------------------------")?;
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
            log_bisect_command(&std::env::args().collect())?;
            Ok(())
        }
    }
}
