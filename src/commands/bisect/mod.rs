use anyhow::Result;
use chrono::Local;
use clap::{Command, ArgMatches};
use colored::Colorize;
use crate::util::{SvError::*, show_commit};
use super::SvCommand;
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

pub trait BisectCommand {
    fn name(&self) -> &'static str;
    fn clap_command(&self) -> clap::Command;
    fn run(&self, matches: &ArgMatches) -> anyhow::Result<()>;
}

pub mod start;
pub mod good;
pub mod bad;
pub mod terms;
pub mod skip;
pub mod unskip;
pub mod run;
pub mod log;
pub mod replay;
pub mod reset;

/// Return a vector of all of the bisect subcommands.
pub fn bisect_commands<'a>() -> Vec<&'a dyn BisectCommand> {
    vec![
        &start::Start,
        &bad::Bad,
        &good::Good,
        &terms::Terms,
        &skip::Skip,
        &unskip::Unskip,
        &run::Run,
        &log::Log,
        &replay::Replay,
        &reset::Reset,
    ]
}


pub struct Bisect;

impl SvCommand for Bisect {
    fn name(&self) -> &'static str { "bisect" }

    fn clap_command(&self) -> Command {
        let mut cmd = Command::new(self.name())
            .about("Use binary search to find the commit that introduced a bug")
            .flatten_help(true);

        //  Add clap subcommmands
        for sub in bisect_commands() {
            cmd = cmd.subcommand(sub.clap_command());
        }
        cmd
             
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        if let Some((name, sub_matches)) = matches.subcommand() {
            if let Some(command) = bisect_commands().iter().find(|cmd| cmd.name() == name) {
                command.run(sub_matches)
            } else {
                Err(General(format!("Fatal: bisect command '{}' not found!", name)).into())
            }
        } else {
            //  If user does not supply a command name
            //  then show the help message
            //  (Need a new mutable clap ref)
            Ok(Bisect.clap_command().print_help()?)
        }
    }
}

fn parse_term(arg: &str) -> Result<String> {
    let re = Regex::new(r"^[A-Za-z][-_A-Za-z]*$").unwrap();
    if !re.is_match(arg)  {
        Err(General("Term must start with a letter and contain only letters, '-', or '_'".to_string()).into())
    }
    else if bisect_commands().iter().any(|c| c.name() == arg) {
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
        self.term_good.as_ref().map(|s| s.as_ref()).unwrap_or(good::Good.name())
    }

    fn bad_name<'a>(&'a self) -> &'a str {
        // bad::Bad.name()
        self.term_bad.as_ref().map(|s| s.as_ref()).unwrap_or(bad::Bad.name())
    }

    fn is_ready(&self) -> bool { self.max_rev.is_some() && self.min_rev.is_some() }
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
    match load_bisect_data()? {
        Some(data) => Ok(data),
        None => {
            let msg = format!("You must first start a bisect session with the 'bisect start' subcommand.");
            Err(General(msg).into())
        }
    }
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
    let logs = svn::log(&vec![], &vec![revision.to_string()], true, Some(1), false,false)?;
    let msg = if let Some(log) = logs.first() { log.msg_1st() }
    else { "".to_string() };
    Ok(msg)
}

fn log_bisect_revision(revision: &str, term: &str) -> Result<()> {
    let line = format!("# {}: [{}] {}", term, revision, get_1st_log_message(revision)?);
    append_to_log(line)
}

fn log_bisect_command(cmd_line: &Vec<String>) -> Result<()> {
    let line = format!("{}", cmd_line.join(" "));
    append_to_log(line)
}

fn display_bisect_command(cmd_line: &Vec<String>) -> () {
    println!("{}", cmd_line.join(" "));
}

//  Convert a revision to a numerica value.
//  !! This assumes that the revision has been
//  resolved and contains only digits !!
fn to_rev_num(rev: &str) -> usize {
    rev.parse().ok().unwrap()
}

fn get_workingcopy_bounds() -> Result<(String, String)> {
    let first = svn::log(&vec![], &vec!["HEAD:0"], true, Some(1), false,false)?
        .first().unwrap().revision.clone();
    let last = svn::log(&vec![], &vec!["0:HEAD"], true, Some(1), false,false)?
        .first().unwrap().revision.clone();
    Ok((first.clone(), last.clone()))
}

fn get_extant_revisions(rev1: &str, rev2: &str) -> Result<Vec<String>> {
    let mut revisions = Vec::new();
    let range = format!("{}:{}", rev1, rev2);
    println!("Fetching history from revisions {} to {}", rev1.yellow(), rev2.yellow());
    let logs = svn::log(&vec![], &vec![range], false, None, false, false)?;
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
    let log = svn::log(&vec!["."], &vec![revision], true, Some(1), false, with_paths)?;
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
    let wc_root = svn::workingcopy_root(&current_dir()?).unwrap();
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
