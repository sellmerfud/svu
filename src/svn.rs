
use std::env;
use std::ffi::OsStr;
use std::io::Write;
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

impl LogEntry {
    // val msg1st = msg.headOption getOrElse ""
    pub fn msg_1st(&self) -> String {
        if self.msg.is_empty() {
            "".to_string()
        }
        else {
            self.msg[0].clone()
        }
    }
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

#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: String,
    pub item_status: String,
    pub props_status: String,
    pub revision: String,
}

#[derive(Debug, Clone)]
pub struct SvnStatus {
    pub path: String,
    pub entries: Vec<StatusEntry>,
}

pub fn run_svn<S>(args: &[S], cwd: Option<&Path>) -> Result<Output> 
    where S: AsRef<OsStr> + Display
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
        Regex::new(r"^(\d+|HEAD|BASE|PREV|COMMITTED)([+-]\d+)?$")
                .expect("Error parsing REV regular expression")
    })
}

pub fn looks_like_revision(text: &str) -> bool {
    rev_re().is_match(text)
}

fn rev_range_re() -> &'static Regex {
    static REV: OnceLock<Regex> = OnceLock::new();
    REV.get_or_init(|| {
        Regex::new(r"^(?:(?:\d+|HEAD|BASE|PREV|COMMITTED)(?:[+-]\d+)?)(?::(?:\d+|HEAD|BASE|PREV|COMMITTED)(?:[+-]\d+)?)?$")
                .expect("Error parsing REV regular expression")
    })
}

pub fn looks_like_revision_range(text: &str) -> bool {
    rev_range_re().is_match(text)
}

//  Use svn log to verify that the revision string refers to a
//  valid revision.
fn get_revision_number(rev: &str, delta: i32, path: &str) -> Result<String> {
    let rev_str = match delta {
        0          =>  rev.to_string(),
        d if d < 0 => format!("{}:0", rev),
        _          => format!("{}:HEAD", rev),
    };
    let limit = Some(delta.abs() as u32 + 1);
    let entries = log(&[path], &[&rev_str], false, limit, false, false)?;
    match entries.last() {
        Some(log) => Ok(log.revision.to_owned()),
        None      => {
            let msg = format!("Revision cannot be resolved rev={}, delta={}, path={}", rev, delta, path);
            Err(General(msg).into())
        }
    }
}

pub fn resolve_revision(rev_string: &str, path: &str) -> Result<String> {
    fn err(r: &str, d: &str, p: &str) -> Result<String> {
        let msg = format!("Cannot resolve revision '{}{}' for path '{}'", r, d, p);
        Err(General(msg).into())
    }
    match rev_re().captures(rev_string) {
        None => err(rev_string, "", path),
        Some(caps) => {
            match (caps.get(1), caps.get(2)) {
                (Some(rev), None) => get_revision_number(rev.as_str(), 0, path).or(err(rev.as_str(), "", path)),
                (Some(rev), Some(delta)) => {
                    let d = delta.as_str().parse::<i32>()?;
                    get_revision_number(rev.as_str(), d, path).or(err(rev.as_str(), delta.as_str(), path))
               }
               _ => unreachable!("resolve_revision_string, fell through match!")
            }
        }
    }
}

//  Resolve a revision string entered by the user.
//  If the string contains a revision keyword or if it contains a delta expression
//  then we must use svn log to get the actual revsion.
//  In order to resovle the string using svn log we need a working copy path.
pub fn resolve_revision_range(rev_string: &str, path: &str) -> Result<String> {
    let parts: Vec<&str> = rev_string.split(":").collect();
    let re = Regex::new(r"[-+]")?;
    match parts.len() {
        1 => resolve_revision(&parts[0], path),
        2 => {
            let a = if re.is_match(&parts[0]) {resolve_revision(&parts[0], path)? } else { parts[0].to_string()} ;
            let b = if re.is_match(&parts[1]) {resolve_revision(&parts[1], path)? } else { parts[1].to_string()} ;
            Ok(format!("{}:{}", a, b))
        }
        _ => {
            let msg = format!("Cannot resolve revision from {} for path {}", rev_string, path);
            Err(General(msg).into())
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

    if let Some(wd) = working_dir.canonicalize().ok() {
        find_it(&wd).map(PathBuf::from)
    }
    else {
        None
    }
}

//  Returns the branch name and current commit revision
//  for the given working copy path.
pub fn current_branch(path: &Path) -> Result<(String, String)> {
    match workingcopy_root(path) {
        Some(wc_root) => {
            let path_info = info(wc_root.to_string_lossy().as_ref(), None)?;
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
        let (commit_rev, commit_author, commit_date) = if let Some(commit) = get_child(&entry, "commit") {
            (
                get_attr(&commit, "revision"),
                get_child_text_or(&commit, "author", "n/a"),
                parse_svn_date_opt(get_child_text(&commit, "date"))
            )
        }
        else {
            ("n/a".to_string(), "n/a".to_string(), *null_date())
        };
        let repo    = get_child(&entry, "repository").unwrap();
        let wc_info = get_child(&entry, "wc-info");

        let entry = SvnInfo {
            path:          get_attr(&entry, "path"),
            repo_rev:      get_attr(&entry, "revision"),
            kind:          get_attr(&entry, "kind"),
            size:          get_attr(&entry, "size").parse::<u64>().ok(),
            url:           get_child_text_or(&entry, "url", "n/a"),
            rel_url:       get_child_text_or(&entry, "relative-url", "n/a"),
            root_url:      get_child_text_or(&repo, "root", "n/a"),
            repo_uuid:     get_child_text_or(&repo, "uuid", "n/a"),
            commit_rev,
            commit_author,
            commit_date,

            wc_path: wc_info.map(|x| get_child_text_or(&x, "wcroot-abspath", "n/a")),
        };
        entries.push(entry);
    }
    Ok(entries)
}

pub fn info<'a>(path: &'a str, revision: Option<&'a str>) -> Result<SvnInfo>
    {

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

pub fn info_list<S>(paths: &[S], revision: Option<S>) -> Result<Vec<SvnInfo>> 
    where S: AsRef<str> + Display {

        let mut args: Vec<&str> = Vec::new();
        args.extend(&["info", "--xml"]);
        let rev_arg: String;
        if let Some(rev) = revision {
            rev_arg = format!("--revision={}", rev);
            args.push(rev_arg.as_str());
        }
        args.extend(paths.iter().map(|s| s.as_ref()).collect::<Vec<&str>>());
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
    let mut entries = vec![];
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
    paths: &[S],
    revisions: &[S],
    include_msg: bool,
    limit: Option<u32>,
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
    let mut path_lists = vec![];
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
pub fn path_lists<S>(paths: &[S]) -> Result<Vec<SvnList>>
    where S: AsRef<str> + Display
{
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
    let mut xx = path_lists(&[path.to_owned()])?;
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
    let writer = File::create(prefixes_file()?)?;
    Ok(serde_json::to_writer_pretty(writer, prefixes)?)
}

//  Verify that the current working directory is within
//  a subversion working copy.
//  Returns the info for the current directory or
//  and Error if not withing a working copy.
pub fn workingcopy_info() -> Result<SvnInfo> {
    if let Ok(wc_info) = info(".", None) {
        Ok(wc_info)
    }
    else {
        Err(General("This command must be run in a serversion working copy directory.".to_string()).into())
    }
}

pub fn in_workingcopy() -> bool {
    workingcopy_info().is_ok()
}

fn parse_svn_status(text: &str) -> Result<SvnStatus> {
    let mut entries: Vec<StatusEntry> = vec![];
    let doc = Document::parse(text)?;
    if let Some(target) = doc.descendants().find(|n| n.has_tag_name("target")) {
        for entry_node in target.children().into_iter() {
            if let Some(wc_node) = get_child(&entry_node, "wc-status") {
                let revision = get_attr(&wc_node, "revision");
                entries.push(StatusEntry {
                    path:         get_attr(&entry_node, "path"),
                    item_status:  get_attr(&wc_node,    "item"),
                    props_status: get_attr(&wc_node,    "props"),
                    revision,
                });
            }
        }
        let path = get_attr(&target, "path");
        Ok(SvnStatus{ path, entries })
    }
    else {
        Err(General("Malformed svn status".to_string()).into())
    }
}

pub fn status<S>(path: S, cwd: Option<&Path>) -> Result<SvnStatus>
    where S: AsRef<str> + Display
{
    let mut args = Vec::new();
    args.extend(vec!["status".to_string(), "--xml".to_string()]);
    args.push(path.to_string());

    let output = run_svn(&args, cwd)?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(parse_svn_status(&text)?)
    }
    else {
        Err(SvnError(output).into())
    }

}

pub fn add<S, T>(paths: &[S], depth: T, auto_props: bool, cwd: Option<&Path>) -> Result<()>
    where S: AsRef<str> + Display,
          T: AsRef<str> + Display
{
    let depth_arg = format!("--depth={}", depth);
    let props_arg = if auto_props { "--auto-props" } else {"--no-auto-props"};
    let mut args = Vec::new();
    args.push("add");
    args.push(depth_arg.as_str());
    args.push(props_arg);
    for path in paths {
        args.push(path.as_ref());
    }
    let output = run_svn(&args, cwd)?;
    if output.status.success() {
        Ok(())
    }
    else {
        Err(SvnError(output).into())        
    }
}

pub fn revert<S, T>(paths: &[S], depth: T, remove_added: bool, cwd: Option<&Path>) -> Result<()>
    where S: AsRef<str> + Display,
          T: AsRef<str> + Display
    {
    let depth_arg = format!("--depth={}", depth);
    let mut args = Vec::new();
    args.push("revert");
    args.push(depth_arg.as_str());
    if remove_added {
        args.push("--remove-added");
    }
    for path in paths {
        args.push(path.as_ref());
    }
    let output = run_svn(&args, cwd)?;
    if output.status.success() {
        Ok(())
    }
    else {
        Err(SvnError(output).into())        
    }
}

pub fn create_patch(patch_file: &Path, cwd: &Path) -> Result<()> {
    let mut args = Vec::new();
    args.push("diff".to_string());
    args.push("--depth=infinity".to_string());
    args.push("--ignore-properties".to_string());
    args.push(".".to_string());
    let output = run_svn(&args, Some(cwd))?;
    if output.status.success() {
        let mut writer = File::create(patch_file)?;
        writer.write_all(&output.stdout)?;
        Ok(())
    }
    else {
        Err(SvnError(output).into())        
    }
}


pub fn apply_patch(patch_file: &Path, dry_run: bool, cwd: Option<&Path>) -> Result<Vec<u8>> {
    let mut args = Vec::new();
    args.push("patch".to_string());
    if dry_run {
        args.push("--dry-run".to_string());
    }
    args.push(patch_file.to_string_lossy().to_string());

    let output = run_svn(&args, cwd)?;
    if output.status.success() {
        Ok(output.stdout)
    }
    else {
        Err(SvnError(output).into())        
    }
}

pub fn update(revision: &str, depth: &str, cwd: Option<&Path>) -> Result<Vec<u8>> {
    let mut args = Vec::new();
    let depth_arg = format!("--depth={}", depth);
    let rev_arg   = format!("--revision={}", revision);
    args.push("update".to_string());
    args.push(depth_arg);
    args.push(rev_arg);
    let output = run_svn(&args, cwd)?;
    if output.status.success() {
        Ok(output.stdout)
    }
    else {
        Err(SvnError(output).into())        
    }
}
