
use std::path::Path;
use std::env::current_dir;
use regex::Regex;
use anyhow::Result;
use clap::{Command, Arg, ArgMatches};
use crate::auth::Credentials;
use crate::util;
use crate::util::SvError::*;
use crate::svn;
use colored::*;
use super::SvCommand;
use std::fmt::Display;


pub struct Branch;
struct Options {
    all_branches:        bool,
    all_tags:            bool,
    branch_regexes:      Vec<Regex>,
    tag_regexes:         Vec<Regex>,
    path:                String,
}

impl Options {
    fn build_options(matches: &ArgMatches) -> Options {
        let branch_regexes = match matches.get_many::<Regex>("branch") {
            Some(regexes) => regexes.map(|r| r.to_owned()).collect(),
            None => vec![]
        };

        let tag_regexes = match matches.get_many::<Regex>("tag") {
            Some(regexes) => regexes.map(|r| r.to_owned()).collect(),
            None => vec![]
        };

        Options {
            all_branches: matches.get_flag("all-branches"),
            all_tags:     matches.get_flag("all-tags"),
            branch_regexes,
            tag_regexes,
            path: matches.get_one::<String>("path").unwrap().to_string(),
        }
    }

    fn no_arguments(&self) -> bool {
        self.all_branches == false &&
        self.all_tags == false &&
        self.branch_regexes.is_empty() &&
        self.tag_regexes.is_empty()
    }

    fn list_branches(&self) -> bool {
        self.all_branches || !self.branch_regexes.is_empty()
    }

    fn list_tags(&self) -> bool {
        self.all_tags || !self.tag_regexes.is_empty()
    }
}

impl SvCommand for Branch {
    fn name(&self) -> &'static str { "branch" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Display current branch or list branches and tags")
            .aliases(vec!["b", "br"])
            .after_help("If neither of --branches or --tags is present, the current branch is displayed.\n\
                         If no regex is specified for --branches or --tags then all are listed.\n\
                         Use -- to separate the PATH from --branches or --tags with no regex")
            .arg(
                Arg::new("branch")
                    .short('b')
                    .long("branches")
                    .value_name("regex")
                    .value_parser(Regex::new)
                    .action(clap::ArgAction::Append)
                    .help("Display branches in the repository that match <regex>")
            )
            .arg(
                Arg::new("tag")
                    .short('t')
                    .long("tags")
                    .value_name("regex")
                    .value_parser(Regex::new)
                    .action(clap::ArgAction::Append)
                    .help("Display tags in the repository that match <regex>")
            )
            .arg(
                Arg::new("all-branches")
                    .short('B')
                    .long("all-branches")
                    .action(clap::ArgAction::SetTrue)
                    .conflicts_with("branch")
                    .help("Display all branches in the repository")
            )
            .arg(
                Arg::new("all-tags")
                    .short('T')
                    .long("all-tags")
                    .action(clap::ArgAction::SetTrue)
                    .conflicts_with("tag")
                    .help("Display all tags in the repository")
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
            let creds = crate::auth::get_credentials()?;
            show_list(&creds, &options)
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

fn show_list(creds: &Option<Credentials>, options: &Options) -> Result<()> {

    let base_url = svn::info(creds, &options.path, None)?.root_url;
    let prefixes = svn::load_prefixes()?;
    let mut all_prefixes = prefixes.branch_prefixes.clone();
    all_prefixes.extend(prefixes.tag_prefixes.clone());

    if options.list_branches() {
        let mut sorted_prefixes = prefixes.branch_prefixes.clone();
        sorted_prefixes.sort();
        list_entries(creds, "Branches", &base_url, &sorted_prefixes, &options.branch_regexes, &all_prefixes)?
    }
    if options.list_tags() {
        let mut sorted_prefixes = prefixes.tag_prefixes.clone();
        sorted_prefixes.sort();
        list_entries(creds, "Tags", &base_url, &sorted_prefixes, &options.tag_regexes, &all_prefixes)?
    }
    Ok(())    
}

fn list_entries<S, T>(creds: &Option<Credentials>,header: &str, base_url: &str, prefixes: &[S], regexes: &[Regex], all_prefixes: &[T]) -> Result<()> 
    where S: AsRef<str> + Display,
          T: AsRef<str> + Display + PartialEq<str>,
{
    //  If a path matches one of the branch/tag prefixes then we do not consider it
    //  an acceptable entry.  Also the entry must match the regex if present.
    let acceptable = |path: &str| -> bool  {
        !all_prefixes.iter().any(|p| p.eq(path)) &&
        (regexes.is_empty() || regexes.iter().any(|r| r.is_match(path)))
    };

    println!();
    println!("{}", header);
    println!("{}", util::divider(60));

    for prefix in prefixes {
        let path_list = svn::path_list(creds, util::join_paths(base_url, prefix).as_str())?;
        for entry in path_list.entries {
            let path = &util::join_paths(prefix, entry.name);
            if acceptable(path.as_str()) {
                println!("{}{}", "^/".green(), path.green());
            }
        }
    }
    Ok(())
}
