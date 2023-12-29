
use std::env;
use std::sync::OnceLock;
use std::process::{Command, Output};
use std::path::{Path, PathBuf};
use std::fs::File;
use chrono::{DateTime, Local};
use roxmltree::{Document, Node};
use anyhow::Result;
use crate::util::SvError::*;
use crate::util::{parse_svn_date_opt, null_date, data_directory};
use regex::Regex;
use std::fmt::Display;
use serde::{Deserialize, Serialize};

//  Get the name of the svn command to run
//  Use "svn" (on the path as the default)
fn svn_cmd() -> &'static String {
    static SVN_CMD: OnceLock<String> = OnceLock::new();
    SVN_CMD.get_or_init(|| {
        env::var("SV_SVN").map(|s| s.clone()).unwrap_or("svn".to_string())
    })
}

#[derive(Debug, Clone)]
pub struct FromPath {
    pub path: String,
    pub revision: String,
}

#[derive(Debug, Clone)]
pub struct LogPath {
    pub path: String,
    pub kind: String,
    pub action: String,
    pub text_mods: bool,
    pub prop_mods: bool,
    pub from_path: Option<FromPath>,
}
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub revision: String,
    pub author:   String,
    pub date:     DateTime<Local>,
    pub msg:      Vec<String>,
    pub paths:    Vec<LogPath>,
}

#[derive(Debug, Clone)]
pub struct SvnInfo {
    pub path:           String,
    pub repo_rev:       String,
    pub kind:           String,
    pub size:           Option<u64>,
    pub url:            String,
    pub rel_url:        String,
    pub root_url:       String,
    pub repo_uuid:      String,
    pub commit_rev:     String,
    pub commit_author:  String,
    pub commit_date:    DateTime<Local>,
    pub wc_path:        Option<String>,  
}

#[derive(Debug, Clone)]
pub struct ListEntry {
    pub name:          String,
    pub kind:          String,
    pub size:          Option<u64>,
    pub commit_rev:    String,
    pub commit_author: String,
    pub commit_date:   DateTime<Local>
}

#[derive(Debug, Clone)]
pub struct SvnList {
    pub path:    String,
    pub entries: Vec<ListEntry>
}

pub fn run_svn<P>(args: &Vec<String>, cwd: Option<P>) -> Result<Output>
    where P: AsRef<Path>
{
    let mut cmd = Command::new(svn_cmd());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    Ok(cmd.args(args).output()?)
}

pub const CWD: Option<&Path> = None;

fn get_attr(n: &Node, name: &str) -> String { 
    n.attribute(name).unwrap_or("").to_owned()
}

fn attr_is(n: &Node, name: &str, target: &str) -> bool { 
    n.attribute(name).map(|a| a == target).unwrap_or(false)
}

fn get_text(n: &Node) -> String { 
    match n.first_child() {
        Some(node) => node.text().unwrap().to_owned(),
        None => "".to_owned()
    }
}

fn get_child<'a, 'i>(parent: &Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    parent.children().find(|c| c.has_tag_name(name))
}

fn get_child_text(parent: &Node, name: &str) -> Option<String> {
    parent.children().find(|n| n.has_tag_name(name))
                     .map(|n| get_text(&n))
}

fn get_child_text_or(parent: &Node, name: &str, default: &str) -> String {
    parent.children().find(|n| n.has_tag_name(name))
                     .map(|n| get_text(&n))
                     .unwrap_or(default.to_owned())
}



fn rev_re() -> &'static Regex {
    static REV: OnceLock<Regex> = OnceLock::new();
    REV.get_or_init(|| {
        Regex::new(r"^(\d+|HEAD|BASE|PREV|COMMITTED)(?::(\d+|HEAD|BASE|PREV|COMMITTED)|([+-])(\d+))?$")
                .expect("Error parsing REV regular expression")
    })
}


pub fn looks_like_revision(text: &str) -> bool {
    rev_re().is_match(text)
}

//  Resolve a revision string entered by the user.
//  If the string contains a revision keyword or if it contains a delta expression
//  then we must use svn log to get the actual revsion.
//  In order to resovle the string using svn log we need a working copy path.
pub fn resolve_revision_string(rev_string: &str, path: &str) -> Result<String> {
    match rev_re().captures(rev_string) {
        None => {
            let msg = format!("Cannot resolve revision from {} for path {}", rev_string, path);
            Err(General(msg).into())
        }
        Some(caps) => {
            let result = match (caps.get(1), caps.get(2), caps.get(3), caps.get(4)) {
                (Some(_), None, None, None)              => rev_string.to_owned(),
                (Some(_), Some(_), None, None)           => rev_string.to_owned(),
                (Some(rev), None, Some(op), Some(delta)) => {
                    let test_rev = if op.as_str() == "-"  { format!("{}:0", rev.as_str())} else { format!("{}:HEAD", rev.as_str()) };
                    let revs = vec![test_rev.as_str()];
                    let limit = delta.as_str().parse::<u16>().ok().map(|v| v + 1);
                    let entries = log(&vec![path], &revs, false, limit, false, false)?;
                    entries.last().unwrap().revision.to_owned()
               }
               _ => unreachable!("resolve_revision_string, fell through match!")
            };
            Ok(result)
        }
    }
}


pub fn workingcopy_root(working_dir: &Path) -> Option<PathBuf> {

    fn find_it(path: &Path) -> Option<&Path> {
        let mut target = PathBuf::from(path);
        target.push(".svn");
        if target.is_dir() {
            Some(path)
        }
        else if let Some(parent) = path.parent() {
            // Relative paths with a single component will return an emtpy parent
            if parent == Path::new("") {
                None
            }
            else {
                find_it(parent)
            }
        }
        else {
            None
        }
    }

    find_it(working_dir).map(|p| p.to_path_buf())
}

//  Returns the branch name and current commit revision
//  for the given working copy path.
pub fn current_branch(path: &Path) -> Result<(String, String)> {
    match workingcopy_root(path) {
        Some(wc_root) => {
            let path_info = info(wc_root.to_string_lossy(), None)?;
            Ok((path_info.rel_url, path_info.commit_rev))
        }
        None => {
            let disp = path.to_string_lossy();
            let msg = format!("{} is not part of a subversion working copy", disp.trim_end_matches("/."));
            Err(General(msg).into())
        }
    }
}

fn parse_svn_info(text: &str) -> Result<Vec<SvnInfo>> {
    let mut entries: Vec<SvnInfo> = vec![];
    let doc = Document::parse(text)?;
    for entry in doc.descendants().filter(|n| n.has_tag_name("entry")) {
        let commit  = get_child(&entry, "commit").unwrap();
        let repo    = get_child(&entry, "repository").unwrap();
        let wc_info = get_child(&entry, "wc-info");

        let entry = SvnInfo {
            path:          get_attr(&entry, "path"),
            repo_rev:      get_attr(&entry, "revision"),
            kind:          get_attr(&entry, "kind"),
            size:          get_attr(&entry, "size").parse::<u64>().ok(),
            url:           get_child_text_or(&entry, "url", "n/a"),
            rel_url:       get_child_text_or(&entry, "relative-url", "n/a"),
            root_url:      get_child_text_or(&repo, "relative-url", "n/a"),
            repo_uuid:     get_child_text_or(&repo, "uuid", "n/a"),
            commit_rev:    get_attr(&commit, "revision"),
            commit_author: get_child_text_or(&commit, "author", "n/a"),
            commit_date:   parse_svn_date_opt(get_child_text(&commit, "date")),

            wc_path: wc_info.map(|x| get_child_text_or(&x, "wcroot-abspath", "n/a")),
        };
        entries.push(entry);
    }
    Ok(entries)
}

pub fn info<S>(path: S, revision: Option<S>) -> Result<SvnInfo>
    where S: AsRef<str> + Display {

    let mut args = Vec::new();
    args.extend(vec!["info".to_string(), "--xml".to_string()]);
    if let Some(rev) = revision {
        args.push(format!("--revision={}", rev));
    }
    args.push(path.to_string());
    let output = run_svn(&args, CWD)?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        let info = parse_svn_info(&text)?;
        Ok(info[0].clone())
    }
    else {
        Err(SvnError(output).into())
    }
}

pub fn info_list<S>(paths: &Vec<String>, revision: Option<S>) -> Result<Vec<SvnInfo>> 
    where S: AsRef<str> + Display {

        let mut args = Vec::new();
        args.extend(vec!["info".to_string(), "--xml".to_string()]);
        if let Some(rev) = revision {
            args.push(format!("--revision={}", rev));
        }
        args.extend(paths.to_vec());
        let output = run_svn(&args, CWD)?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            parse_svn_info(&text)
        }
        else {
            Err(SvnError(output).into())
        }
}

fn get_log_entry_paths(log_entry: &Node) -> Vec<LogPath> {
    let mut paths: Vec<LogPath> = vec![];
    for path_node in log_entry.descendants().filter(|n| n.has_tag_name("path")) {
        let from_path = if path_node.has_attribute("copyfrom-path") {
            Some(FromPath {
                    path:     get_attr(&path_node, "copyfrom-path"),
                    revision: get_attr(&path_node, "copyfrom-rev")
                })
        }
        else {
            None
        };

        let log_path = LogPath {
            path:      get_text(&path_node),
            kind:      get_attr(&path_node, "kind"),
            action:    get_attr(&path_node, "action"),
            text_mods: attr_is(&path_node, "text-mods", "true"),
            prop_mods: attr_is(&path_node, "prop-mods", "true"),
            from_path: from_path        
        };

        paths.push(log_path);
    }

    paths
}



fn parse_svn_log(text: &str) -> Result<Vec<LogEntry>> {
    let mut entries: Vec<LogEntry> = vec![];
    let doc = Document::parse(text)?;
    for log_entry in doc.descendants().filter(|n| n.has_tag_name("logentry")) {

        let entry = LogEntry {
            revision: get_attr(&log_entry, "revision"),
            author:   get_child_text_or(&log_entry, "author", "n/a"),
            date:     parse_svn_date_opt(get_child_text(&log_entry, "date")),
            msg:      get_child_text_or(&log_entry, "msg", "").split("\n").map(|s| s.to_owned()).collect(),
            paths:    get_log_entry_paths(&log_entry)
        };
        entries.push(entry);
    }
    Ok(entries)
}

//  Run the svn log command
pub fn log<S>(
    paths: &Vec<S>,
    revisions: &Vec<S>,
    include_msg: bool,
    limit: Option<u16>,
    stop_on_copy: bool,
    include_paths: bool) -> Result<Vec<LogEntry>>
        where S: AsRef<str> + Display
    {

    let mut args = Vec::new();
    args.extend(vec!["log".to_string(), "--xml".to_string()]);
    if !include_msg  { args.push("--quiet".to_string()) }
    if stop_on_copy  { args.push("--stop-on_copy".to_string()) }
    if include_paths { args.push("--verbose".to_string()) }
    args.extend(limit.into_iter().map(|l| format!("--limit={}", l)));
    args.extend(revisions.into_iter().map(|r| format!("--revision={}", r)));
    args.extend(paths.into_iter().map(|p| p.to_string()));
    let output = run_svn(&args, CWD)?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        parse_svn_log(&text)
    }
    else {
        Err(SvnError(output).into())
    }
}


fn parse_svn_list(text: &str) -> Result<Vec<SvnList>> {
    let mut path_lists: Vec<SvnList> = vec![];
    let doc = Document::parse(text)?;
    for list_node in doc.descendants().filter(|n| n.has_tag_name("list")) {
        let path = get_attr(&list_node, "path");
        let mut entries: Vec<ListEntry> = vec![];

        for entry_node in list_node.children().filter(|n| n.has_tag_name("entry")) {
            let (commit_rev, commit_author, commit_date) =
                if let Some(commit_node) = get_child(&entry_node, "commit") {
                    (get_attr(&commit_node, "revision"),
                    get_child_text_or(&commit_node, "author", "n/a"),
                    parse_svn_date_opt(get_child_text(&commit_node, "date")))

                }
                else {
                    ("n/a".to_owned(), "n/a".to_owned(), *null_date())
                };
            let entry = ListEntry {
                name: get_child_text_or(&entry_node, "name", ""),
                kind: get_attr(&entry_node, "kind"),
                size: get_child_text(&entry_node, "size").map(|s| s.parse::<u64>().unwrap()),
                commit_rev,
                commit_author,
                commit_date,
            };
            entries.push(entry);
        }
        path_lists.push(SvnList { path, entries });
    }
    Ok(path_lists)
}


// Get svn list for multiple paths
pub fn path_lists(paths: &Vec<String>) -> Result<Vec<SvnList>> {
    if paths.is_empty() {
        Ok(vec![])
    }
    else {
        let mut args = vec!["list".to_owned(), "--xml".to_owned()];
        args.extend(paths.into_iter().map(|p| p.to_string()));
        let output = run_svn(&args, CWD)?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            parse_svn_list(&text)
        }
        else {
            Err(SvnError(output).into())
        }   
    }
}

//  Get svn list for a single path.
pub fn path_list(path: &str) -> Result<SvnList> {
    let mut xx = path_lists(&vec![path.to_owned()])?;
    Ok(xx.remove(0))
}

pub fn change_diff(path: &str, commit_rev: &str) -> Result<Vec<String>> {
    let args = vec![
        "diff".to_string(),
        "--change".to_string(),
        commit_rev.to_string(),
        path.to_string()
    ];

    let output = run_svn(&args, CWD)?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(text.split("\n").map(|l| l.to_string()).collect())
    }
    else {
        Err(SvnError(output).into())
    }   
}

fn prefixes_file() -> Result<PathBuf> {
    match data_directory() {
        Ok(dir) => Ok(dir.join("prefixes.json")),
        e @Err(_) => e.into()
    }
}
#[derive(Serialize, Deserialize)]
pub struct Prefixes {
    #[serde(rename(serialize = "trunkPrefix", deserialize = "trunkPrefix"))]
    pub trunk_prefix:    String,
    #[serde(rename(serialize = "branchPrefixes", deserialize = "branchPrefixes"))]
    pub branch_prefixes: Vec<String>,
    #[serde(rename(serialize = "tagPrefixes", deserialize = "tagPrefixes"))]
    pub tag_prefixes:    Vec<String>
}

pub fn load_prefixes() -> Result<Prefixes> {
    let path = prefixes_file()?;
    if path.is_file() {
        let reader = File::open(path)?;
        let prefixes: Prefixes = serde_json::from_reader(reader)?;
        Ok(prefixes)
    } else {
        //  Return the defaults
        Ok(Prefixes {
            trunk_prefix:    "trunk".to_string(),
            branch_prefixes: vec!["branches".to_string()],
            tag_prefixes:    vec!["tags".to_string()],
        })
    }
}

pub fn save_prefixes(prefixes: &Prefixes) -> Result<()> {
    // let writer = OpenOptions::new()
    //     .write(true)
    //     .truncate(true)
    //     .create(true)
    //     .open(prefixes_file()?)?;
    let writer = File::create(prefixes_file()?)?;
    Ok(serde_json::to_writer_pretty(writer, prefixes)?)
}
