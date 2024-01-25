use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
use colored::Colorize;
use crate::auth::Credentials;
use crate::util::{SvError::*, show_commit};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::fs::File;
use crate::svn::{self, LogEntry};
use crate::util;
use std::fs::OpenOptions;
use serde::{Deserialize, Serialize};
use regex::Regex;
use std::env::current_dir;
use std::collections::HashSet;
use std::fmt::Display;

mod start;
mod good;
mod bad;
mod terms;
mod skip;
mod unskip;
mod log;
mod run;
mod replay;
mod reset;

/// Use binary search to find the commit that introduced a bug
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]
#[command(args_conflicts_with_subcommands = true)]
#[command(flatten_help = false)]
pub struct Bisect {
    #[command(subcommand)]
    command: BisectCommands,
}

#[derive(Debug, Subcommand)]
enum BisectCommands {
    Start(start::Start),
    Bad(bad::Bad),
    Good(good::Good),
    Terms(terms::Terms),
    Skip(skip::Skip),
    Unskip(unskip::Unskip),
    Log(log::Log),
    Run(run::Run),
    Replay(replay::Replay),
    Reset(reset::Reset),
}
use BisectCommands::*;

impl Bisect {

    pub fn run(&mut self) -> Result<()> {
        match &mut self.command {
            Start(cmd)  => cmd.run(),
            Bad(cmd)    => cmd.run(),
            Good(cmd)   => cmd.run(),
            Terms(cmd)  => cmd.run(),
            Skip(cmd)   => cmd.run(),
            Unskip(cmd) => cmd.run(),
            Log(cmd)    => cmd.run(),
            Run(cmd)    => cmd.run(),
            Replay(cmd) => cmd.run(),
            Reset(cmd)  => cmd.run(),
        }
    }
}


fn parse_term(arg: &str) -> Result<String> {
    let commands = HashSet::from(
        [ "start", "bad", "good", "terms", "skip", "unskip", "log", "run", "replay", "reset" ]
    );
    let re = Regex::new(r"^[A-Za-z][-_A-Za-z]*$").unwrap();
    if !re.is_match(arg)  {
        Err(General("Term must start with a letter and contain only letters, '-', or '_'".to_string()).into())
    }
    else if commands.contains(arg) {
        Err(General("Term cannot mask another bisect command".to_string()).into())
    }
    else {
        Ok(arg.to_string())
    }
}


// Common structures and functions used by all of the bisect commands.

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BisectData {
    #[serde(rename(serialize = "localPath", deserialize = "localPath"))]
    local_path:   String,   // No longer used!
    #[serde(rename(serialize = "originalRev", deserialize = "originalRev"))]
    original_rev: String,
    #[serde(rename(serialize = "headRev", deserialize = "headRev"))]
    head_rev:     Option<String>,
    #[serde(rename(serialize = "firstRev", deserialize = "firstRev"))]
    first_rev:    Option<String>,
    #[serde(rename(serialize = "maxRev", deserialize = "maxRev"))]
    max_rev:      Option<String>,
    #[serde(rename(serialize = "minRev", deserialize = "minRev"))]
    min_rev:      Option<String>,
    skipped:      HashSet<String>,
    #[serde(rename(serialize = "termGood", deserialize = "termGood"))]
    term_good:    Option<String>,
    #[serde(rename(serialize = "termBad", deserialize = "termBad"))]
    term_bad:     Option<String>,
}

impl BisectData {
    fn good_name<'a>(&'a self) -> &'a str {
        // bad::Bad.name()
        self.term_good.as_ref()
            .map(|s| s.as_ref())
            .unwrap_or("good")
    }

    fn bad_name<'a>(&'a self) -> &'a str {
        // bad::Bad.name()
        self.term_bad.as_ref()
            .map(|s| s.as_ref())
            .unwrap_or("bad")
    }

    fn is_ready(&self) -> bool {
        self.max_rev.is_some() && self.min_rev.is_some()
    }
}


fn bisect_data_file() -> Result<PathBuf> {
    Ok(util::data_directory()?.join("bisect_data.json"))
}


fn bisect_log_file() -> Result<PathBuf> {
    Ok(util::data_directory()?.join("bisect_log"))
}

fn load_bisect_data() -> Result<Option<BisectData>> {
    let path = bisect_data_file()?;
    if path.is_file() {
        let reader = File::open(path)?;
        let data: BisectData = serde_json::from_reader(reader)?;
        Ok(Some(data))
    } else {
        Ok(None)
    }
}

fn save_bisect_data(data: &BisectData) -> Result<()> {
    let writer = File::create(bisect_data_file()?)?;
    Ok(serde_json::to_writer_pretty(writer, data)?)
}

//  Load and return the bisect data or return a Generaal
//  error if the data file is missing.
fn get_bisect_data() -> Result<BisectData> {
    load_bisect_data()?
        .ok_or(General(format!("You must first start a bisect session with the 'bisect start' subcommand.")).into())
}

fn append_to_log<S>(msg: S) -> Result<()> 
    where S: AsRef<str> + Display
{
    let mut writer = OpenOptions::new()
        .append(true)
        .create(true)
        .open(bisect_log_file()?)?;
    writer.write_all((msg.to_string() + "\n").as_bytes())?;
    Ok(())
}

fn display_log() -> Result<()> {
    let path = bisect_log_file()?;
    if path.is_file() {
        let reader = BufReader::new(File::open(path)?);
        for line in reader.lines() {
            println!("{}", line?);
        }
    }
    Ok(())
}

fn get_1st_log_message(revision: &str) -> Result<String> {
    let logs = svn::log(&None, &[], &[revision.to_string()], true, Some(1), false,false)?;
    let msg = if let Some(log) = logs.first() { log.msg_1st() }
    else { "".to_string() };
    Ok(msg)
}

fn log_bisect_revision(revision: &str, term: &str) -> Result<()> {
    let line = format!("# {}: [{}] {}", term, revision, get_1st_log_message(revision)?);
    append_to_log(line)
}

fn log_bisect_command(cmd_line: &[String]) -> Result<()> {
    let line = format!("{}", cmd_line.join(" "));
    append_to_log(line)
}

//  Convert a revision to a numerica value.
//  !! This assumes that the revision has been
//  resolved and contains only digits !!
fn to_rev_num(rev: &str) -> usize {
    rev.parse().ok().unwrap()
}

fn get_workingcopy_bounds() -> Result<(String, String)> {
    let first = svn::log(&None, &[], &["HEAD:0"], true, Some(1), false,false)?
        .first()
        .unwrap()
        .revision
        .clone();
    let last = svn::log(&None, &[], &["0:HEAD"], true, Some(1), false,false)?
        .first()
        .unwrap()
        .revision
        .clone();
    
    Ok((first.clone(), last.clone()))
}

fn get_extant_revisions(rev1: &str, rev2: &str) -> Result<Vec<String>> {
    let mut revisions = Vec::new();
    let range = format!("{}:{}", rev1, rev2);
    println!("Fetching history from revisions {} to {}", rev1.yellow(), rev2.yellow());
    let logs = svn::log(&None, &[], &[range], false, None, false, false)?;
    for log in &logs {
        revisions.push(log.revision.clone());
    }
    Ok(revisions)
}

fn get_waiting_status(data: &BisectData) -> Option<String> {
    let good = data.good_name();
    let bad  = data.bad_name();

    match (&data.max_rev, &data.min_rev) {
        (None, None)    => Some(format!("status: waiting for both '{}' and '{}' revisions", good, bad)),
        (Some(_), None) => Some(format!("status: waiting for a '{}' revision", good)),
        (None, Some(_)) => Some(format!("status: waiting for a '{}' revision", bad)),
        _               => None
    }
}

fn get_log_entry(revision: &str, with_paths: bool) -> Result<Option<LogEntry>> {
    let log = svn::log(&None, &["."], &[revision], true, Some(1), false, with_paths)?;
    Ok(log.first().map(|l| l.clone()))
}

fn perform_bisect(data: &BisectData) -> Result<bool> {
    if !data.is_ready() {
        return Err(General("fatal: peform_bisect() called when data not ready".to_string()).into())
    }

    let max_rev = data.max_rev.as_ref().unwrap();
    let min_rev = data.min_rev.as_ref().unwrap();
    let extant_revs = get_extant_revisions(max_rev, min_rev)?;
    let candidate_revs = &extant_revs[1..extant_revs.len()-1];
    let non_skipped_revs: Vec<String> = candidate_revs
        .iter()
        .filter_map(|r| if data.skipped.contains(r) { None } else { Some(r.clone()) })
        .collect();

    if non_skipped_revs.is_empty() {
        if !candidate_revs.is_empty() {
            println!("\nThere are only skipped revisions left to test.");
            println!("The first {} commit could be any of:", data.bad_name());
            println!("{} {}", max_rev.yellow(), get_1st_log_message(max_rev)?);
            for rev in candidate_revs {
                println!("{} {}", rev.yellow(), get_1st_log_message(rev)?);
            }
            println!("We cannot bisect more!");
            Ok(true)
        } else {
            println!("\nThe first '{}' revision is: {}", data.bad_name(), max_rev.yellow());
            if let Some(log_entry) = get_log_entry(max_rev, true)? {
                show_commit(&log_entry, true, true);
            }
            Ok(true)
        }
    } else {

        let num = non_skipped_revs.len();
        let steps = match (f64::log10(num as f64) / f64::log10(2.0)) as usize {
            1 => "1 step".to_string(),
            n => format!("{} steps", n)
        };
        let next_rev = &non_skipped_revs[non_skipped_revs.len() / 2];

        println!("Bisecting: {} revisions left to test after this (roughly {}) ", num, steps);
        update_workingcopy(next_rev)?;
        Ok(false)    
    }
}

fn update_workingcopy(revision: &String) -> Result<()> {
    let msg = get_1st_log_message(revision)?;
    let wc_info = svn::workingcopy_info()?;
    let wc_root = PathBuf::from(wc_info.wc_path.unwrap());
    println!("Updating working copy: [{}] {}", revision.yellow(), msg);
    svn::update(revision, "infinity", Some(&wc_root))?;
    Ok(())
}

//  Returns true if perform_bisect() reports that that session
//  is complete.
//  If this revision was previously skipped, it is no longer skipped.
//  to start performing bisects
fn mark_good_revision(revision: &str) -> Result<bool> {
    let mut data = get_bisect_data()?;
    data.skipped.remove(revision);
    data.min_rev = Some(revision.to_string());
    save_bisect_data(&data)?;
    log_bisect_revision(revision, &data.good_name())?;
    if data.is_ready() {
        perform_bisect(&data)
    } else {
        Ok(false)
    }
}

//  Returns true if perform_bisect() reports that that session
//  is complete.
//  If this revision was previously skipped, it is no longer skipped.
//  to start performing bisects
fn mark_bad_revision(revision: &str) -> Result<bool> {
    let mut data = get_bisect_data()?;
    data.skipped.remove(revision);
    data.max_rev = Some(revision.to_string());
    save_bisect_data(&data)?;
    log_bisect_revision(revision, &data.bad_name())?;
    if data.is_ready() {
        perform_bisect(&data)
    } else {
        Ok(false)
    }
}

//  Returns true if the perform_bisect() reports that the session is complete
fn mark_skipped_revisions(incoming_skipped: &HashSet<String>) -> Result<bool> {
    let mut data = get_bisect_data()?;
    let mut new_skipped: Vec<String> =
        incoming_skipped
        .difference(&data.skipped)
        .cloned()
        .collect();

    if new_skipped.is_empty() {
        Ok(false)
    } else {
        let skipped: HashSet<String> =
            data.skipped
            .union(incoming_skipped)
            .cloned()
            .collect();       
        data = BisectData { skipped, ..data };
        save_bisect_data(&data)?;
        new_skipped.sort_by(|a, b| b.cmp(a)); // Sorted most recent first
        for rev in &new_skipped {
            log_bisect_revision(rev, "skip")?;
        }

        if data.is_ready() {
            perform_bisect(&data)
        }
        else {
            Ok(false)
        }
    }
}

//  Returns true if the perform_bisect() reports that the session is complete
fn mark_unskipped_revisions(incoming_unskipped: &HashSet<String>) -> Result<bool> {
    let mut data = get_bisect_data()?;
    let mut new_unskipped: Vec<String> =
        incoming_unskipped
        .intersection(&data.skipped)
        .cloned()
        .collect();

    if new_unskipped.is_empty() {
        Ok(false)
    } else {
        let skipped: HashSet<String> =
            data.skipped
            .difference(incoming_unskipped)
            .cloned()
            .collect();       
        data = BisectData { skipped, ..data };
        save_bisect_data(&data)?;
        new_unskipped.sort_by(|a, b| b.cmp(a)); // Sorted most recent first
        for rev in &new_unskipped {
            log_bisect_revision(rev, "unskip")?;
        }

        if data.is_ready() {
            perform_bisect(&data)
        }
        else {
            Ok(false)
        }
    }
}

//  Used by skip and unskip.
//  Each rev_str is either a single revision or a range rev:rev.
//  These value should be resovlved to be well formed (see: resolved_revision_range())
//  This function gathers the actual revision numbers that lie within each range
//  for the given path.
fn gather_revisions(creds: &Option<Credentials>, rev_str: &str, path: &str) -> Result<HashSet<String>> {
    let mut revisions = HashSet::new();

    if rev_str.contains(':') {
        let resolved = svn::resolve_revision_range(&creds, rev_str, path)?;
        let entries = svn::log(&None, &[path], &[&resolved], false, None, false, false)?;
        revisions.extend(entries.iter().map(|e| e.revision.clone()));
    }
    else {
        revisions.insert(svn::resolve_revision(&creds, rev_str, path)?);
    }

    Ok(revisions)
}
