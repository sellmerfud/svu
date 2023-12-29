
use std::path::Path;
use std::env::current_dir;
use regex::Regex;
use anyhow::Result;
use clap::{Command, Arg, ArgMatches};
use crate::util;
use crate::util::SvError::*;
use crate::svn;
use colored::*;
use super::SvCommand;


pub struct Branch;
struct Options {
    all_branches:        bool,
    all_tags:            bool,
    branch_regex:        Option<Regex>,
    tag_regex:           Option<Regex>,
    path:                String,
}

impl Options {
    fn build_options(matches: &ArgMatches) -> Options {
        let (all_branches, branch_regex) = if matches.contains_id("branch") {
            if let Some(re) = matches.get_one::<Regex>("branch") {
                (false, Some(re.to_owned()))
            }
            else { (true, None) }
        }
        else { (false, None) };

        let (all_tags, tag_regex) = if matches.contains_id("tag") {
            if let Some(re) = matches.get_one::<Regex>("tag") {
                (false, Some(re.to_owned()))
            }
            else { (true, None) }
        }
        else { (false, None) };

        Options {
            all_branches,
            all_tags,
            branch_regex,
            tag_regex,
            path: matches.get_one::<String>("path").unwrap().to_string(),
        }
    }

    fn no_arguments(&self) -> bool {
        self.all_branches == false &&
        self.all_tags == false &&
        self.branch_regex.is_none() &&
        self.tag_regex.is_none()
    }

    fn list_branches(&self) -> bool {
        self.all_branches || self.branch_regex.is_some()
    }

    fn list_tags(&self) -> bool {
        self.all_tags || self.tag_regex.is_some()
    }
}

impl SvCommand for Branch {
    fn name(&self) -> &'static str { "branch" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Display current branch or list branches and tags")
            .after_help("If neither of --branches or --tags is present, the current branch is displayed.\n\
                         If no regex is specified for --branches or --tags then all are listed.\n\
                         Use -- to separate the PATH from --branches or --tags with no regex")
            .arg(
                Arg::new("branch")
                    .short('b')
                    .long("branches")
                    .value_name("regex")
                    .value_parser(Regex::new)
                    .num_args(0..=1)
                    .help("Display branches in the repository")
            )
            .arg(
                Arg::new("tag")
                    .short('t')
                    .long("tags")
                    .value_name("regex")
                    .value_parser(Regex::new)
                    .num_args(0..=1)
                    .help("Display tags in the repository")
            )
            .arg(
                Arg::new("path")
                    .value_name("PATH")
                    .default_value(".")
                    .help("Path to working copy directory")
            )

    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        let options = Options::build_options(matches);
        
        if options.no_arguments() {
            show_current_branch(&options)
        }
        else {
            show_list(&options)
        }
    }
}

fn show_current_branch(options: &Options) -> Result<()> {
    if options.path.starts_with("^") || options.path.contains("://") {
        return Err(General("Cannot show the current branch of a URL".to_string()).into())
    }

    let mut path_buf;
    let mut path = Path::new(options.path.as_str());
    if path.is_relative() {
        path_buf = current_dir()?;
        path_buf.push(path);
        path = path_buf.as_path();
    }

    let (name, revision) = svn::current_branch(path)?;
    println!("Current branch: {} [{}]", name.green(), revision.yellow());
    Ok(())    
}

fn show_list(options: &Options) -> Result<()> {

    if options.list_branches() {
        list_entries("Branches", "http://gemini2.rocsoftware.com/svn/om", &vec!["branches".to_string()], &options.branch_regex)?
    }

    if options.list_tags() {
        list_entries("Tags", "http://gemini2.rocsoftware.com/svn/om", &vec!["tags".to_string()], &options.tag_regex)?
    }

    Ok(())    
}

fn list_entries(header: &str, base_url: &str, prefixes: &Vec<String>, regex: &Option<Regex>) -> Result<()> {
    let all_prefixes = vec!["branches/".to_string(), "tags".to_string()];
    //  If a path matches on of the branch/tag prefixes then we do not consider it
    //  an acceptable entry.  Also the entry must match the regex if present.
    let acceptable = |path: &String| -> bool  {
        !all_prefixes.contains(path) && regex.as_ref().map(|r| r.is_match(path.as_str())).unwrap_or(true)
    };

    println!();
    println!("{}", header);
    println!("----------------------");

    for prefix in prefixes {
        let path_list = svn::path_list(util::join_paths(base_url, prefix).as_str())?;
        for entry in path_list.entries {
            let path = "^/".to_owned() + &util::join_paths(prefix, entry.name);
            if acceptable(&path) {
                println!("{}", path.green());
            }
        }
    }
    Ok(())
}
