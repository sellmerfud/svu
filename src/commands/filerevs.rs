

use regex::Regex;
use anyhow::Result;
use clap::{Command, Arg, ArgMatches};
use colored::*;
use super::SvCommand;
use crate::util::{SvError::*, join_paths, display_svn_datetime};
use crate::svn::{self, Prefixes, SvnInfo};
use chrono::Local;
pub struct FileRevs;

#[derive(Debug)]
struct Options {
    all_branches:   bool,
    all_tags:       bool,
    branch_regexes: Vec<Regex>,
    tag_regexes:    Vec<Regex>,
    paths:          Vec<String>,
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

        let paths = match matches.get_many::<String>("paths") {
            Some(paths) => paths.map(|s| s.to_owned()).collect(),
            None => vec![]
        };

        Options {
            all_branches:   matches.get_flag("all-branches"),
            all_tags:      matches.get_flag("all-tags"),
            branch_regexes,
            tag_regexes,
            paths,
        }
    }
}

impl SvCommand for FileRevs {
    fn name(&self) -> &'static str { "filerevs" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Display commit revisions of files across tags and branches")
            .after_help("If no branches or tags are specified, then only the trunk\n\
                         revision is displayed.\n\
                         --branch and --tag may be specified multiple times.\n"
            )
            .arg(
                Arg::new("branch")
                    .short('b')
                    .long("branch")
                    .value_name("regex")
                    .value_parser(Regex::new)
                    .action(clap::ArgAction::Append)
                    .help("Include branches that match <regex>")
            )
            .arg(
                Arg::new("tag")
                    .short('t')
                    .long("tag")
                    .value_name("regex")
                    .value_parser(Regex::new)
                    .action(clap::ArgAction::Append)
                    .help("Include tags that match <regex>")
            )
            .arg(
                Arg::new("all-branches")
                    .short('B')
                    .long("all-branches")
                    .action(clap::ArgAction::SetTrue)
                    .conflicts_with("branch")
                    .help("Display all branches")
            )
            .arg(
                Arg::new("all-tags")
                    .short('T')
                    .long("all-tags")
                    .action(clap::ArgAction::SetTrue)
                    .conflicts_with("tag")
                    .help("Display all tags")
            )
            .arg(
                Arg::new("paths")
                .value_name("PATH")
                .action(clap::ArgAction::Append)
                .required(true)
                .help("PATH or URL to target file")
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        show_results(&Options::build_options(matches))
    }
}

fn get_branches(root_url: &str, all: bool, regexes: &Vec<Regex>, prefixes: &Prefixes) -> Result<impl Iterator<Item = String>> {
    let mut branches = Vec::<String>::new();
    if all || !regexes.is_empty() {
        let mut all_prefixes = prefixes.branch_prefixes.clone();
        all_prefixes.extend(prefixes.tag_prefixes.clone());
        let mut branch_prefixes = prefixes.branch_prefixes.clone();
        branch_prefixes.sort();
        let acceptable = |branch: &String| -> bool {
            !all_prefixes.contains(branch) &&
            (all || regexes.into_iter().any(|re| re.is_match(&branch)))
        };

        for prefix in &branch_prefixes {
            let path_list = svn::path_list(&join_paths(root_url, prefix))?;
            for entry in &path_list.entries {
                let branch = join_paths(prefix, &entry.name);
                if acceptable(&branch) {
                    branches.push(branch);
                }
            }
        }        
    }
    Ok(branches.into_iter())
}

fn get_tags(root_url: &str, all: bool, regexes: &Vec<Regex>, prefixes: &Prefixes) -> Result<impl Iterator<Item = String>> {
    let mut tags = Vec::<String>::new();
    if all || !regexes.is_empty() {
        let mut all_prefixes = prefixes.tag_prefixes.clone();
        all_prefixes.extend(prefixes.tag_prefixes.clone());
        let mut tag_prefixes = prefixes.tag_prefixes.clone();
        tag_prefixes.sort();
        let acceptable = |tag: &String| -> bool {
            !all_prefixes.contains(tag) &&
            (all || regexes.into_iter().any(|re| re.is_match(&tag)))
        };

        for prefix in &tag_prefixes {
            let path_list = svn::path_list(&join_paths(root_url, prefix))?;
            for entry in &path_list.entries {
                let tag = join_paths(prefix, &entry.name);
                if acceptable(&tag) {
                    tags.push(tag);
                }
            }
        }        
    }
    Ok(tags.into_iter())
}

fn show_results(options: &Options) -> Result<()> {
        // First make sure all paths are rooted in the same repository
    let path_list = svn::info_list(&options.paths, None::<String>)?;
    let repo_uuid = &path_list[0].repo_uuid;
    for item in &path_list[1..] {
        if &item.repo_uuid != repo_uuid {
            return Err(General("All paths must refer to the same repository.".to_string()).into());
        }
    }

    let root_url    = &path_list[0].root_url;
    let prefix_info = svn::load_prefixes()?;
    let branches    = get_branches(root_url, options.all_branches, &options.branch_regexes, &prefix_info)?;
    let tags        = get_tags(root_url, options.all_tags, &options.tag_regexes, &prefix_info)?;

    let prefixes: Vec<String> = vec![prefix_info.trunk_prefix]
        .into_iter()
        .chain(branches.chain(tags))
        .collect();
    let mut sorted_prefixes = prefixes.clone();
    sorted_prefixes.sort_by(|a, b| a.len().cmp(&b.len()).reverse());  // Sorteed by length longest first.

    for path_entry in &path_list {
        show_path_result(root_url, path_entry, &prefixes, &sorted_prefixes)?;
    }
    Ok(())
}

//  We must determine the path to the file relative
//  to its subversion prefix.
//  Find find the url entry for one of our prefixes
//  so we can determine where the relative path starts.
fn get_svn_rel_path(rel_url: &str, sorted_prefixes: &Vec<String>) -> Result<String> {
    let prefix = sorted_prefixes.iter().find(|p| {
        rel_url[2..].starts_with(*p) // Skip the leading ^/
    });
    match prefix {
        Some(prefix) =>
            // Skip ^/<prefix>/
            Ok(rel_url[prefix.len() + 3..].to_string()),
        None => {
            let msg = format!("Cannot determine relative path for {}", rel_url);
            Err(General(msg).into())
        }
    }
}

fn max_width(label: &str, value_widths: impl Iterator<Item = usize>) -> usize {
    value_widths.fold(label.len(), |m, v| m.max(v))
}

fn get_chunks(prefixes: &Vec<String>, num_cpus: usize) -> Vec<Vec<String>> {
    let per_chunck  = prefixes.len() / num_cpus;
    let extra       = prefixes.len() % num_cpus;
    let mut chunks  = Vec::new();
    let mut current = Vec::<String>::new();
    let mut it      = prefixes.iter();

    while let Some(prefix) = it.next() {
        let plus_one = if chunks.len() + 1 <= extra { 1 } else { 0 };
        let limit = per_chunck + plus_one;
        if current.len() == limit {
            chunks.push(current);
            current = Vec::new();
        }
        current.push(prefix.to_string());
    }
    chunks.push(current);
    chunks
}
// /this/is/the/users/path              
// Location        Revision  Author  Date         Size
// --------------  --------  ------  -----------  ----------
// trunk               7601
// branches/8.1        7645
// tags/8.1.1-GA       7625
fn show_path_result(root_url: &str, path_entry: &SvnInfo, prefixes: &Vec<String>, sorted_prefixes: &Vec<String>) -> Result<()> {
    use std::{thread, io};
    struct Entry(String, Option<Box<SvnInfo>>);

    fn process_prefixes(root_url: &str, rel_path: &str, prefixes: Vec<String>) -> io::Result<Vec<Entry>> {
        let mut results = Vec::<Entry>::new();
        for prefix in prefixes {
            let path = join_paths(join_paths(root_url, prefix.as_str()), rel_path);
            let info = svn::info(path.as_str(), Some("HEAD")).ok().map(|i| Box::new(i));
            results.push(Entry(prefix, info));
        }
        Ok(results)
    }

    let rel_path = &get_svn_rel_path(&path_entry.rel_url, sorted_prefixes)?;
    let mut results = Vec::<Entry>::new();

    let num_cpus = num_cpus::get();
    if num_cpus > 1 {
        let prefix_chunks = get_chunks(prefixes, num_cpus);
        let mut threads = vec![];
        for prefix_list in prefix_chunks {
            let r = root_url.to_string();
            let p = rel_path.clone();
            threads.push(
                thread::spawn(move || process_prefixes(&r,&p, prefix_list))
            );
        }
    
        for thread in threads {
            for result in thread.join().unwrap()? {
                results.push(result);
            }
        }
    }
    else {
        for prefix in prefixes {
            // let path = join_paths(join_paths(root_url, prefix), rel_path);
            let path = join_paths(join_paths(root_url, prefix.as_str()), rel_path.as_str());
            let info = svn::info(path.as_str(), Some("HEAD")).ok().map(|i| Box::new(i));
            results.push(Entry(prefix.clone(), info));
        }
    }

    const LOCATION: &str = "Location";
    const REVISION: &str = "Revision";
    const AUTHOR: &str   = "Author";
    const DATE: &str     = "Date";
    const SIZE: &str     = "Size";

    let location_width = max_width(LOCATION, results.iter()
                                                    .map(|r| r.0.len() + 2));
    let revision_width = max_width(REVISION, results.iter()                                                    
                                                    .map(|r| {
                                                        match &r.1 {
                                                            Some(info) => info.commit_rev.len(),
                                                            None => 0
                                                        }
                                                    }));
    let author_width   = max_width(REVISION, results.iter()
                                                    .map(|r| {
                                                        match &r.1 {
                                                            Some(info) => info.commit_author.len(),
                                                            None => 0
                                                        }
                                                    }));
    let date_width     = display_svn_datetime(&Local::now()).len();
    let size_width     = 8;
    let col_sep        = " ";

    println!();
    if path_entry.kind == "dir" {
        println!("{}", (rel_path.to_owned() + "/").blue());
    } else {
        println!("{}", rel_path.blue());
    }
    // Headers
    print!("{:location_width$}{}", LOCATION, col_sep);
    print!("{:revision_width$}{}", REVISION, col_sep);
    print!("{:author_width$}{}",   AUTHOR, col_sep);
    print!("{:date_width$}{}",     DATE, col_sep);
    println!("{:size_width$}",     SIZE);

    print!("{:->location_width$}{}", "-", col_sep);
    print!("{:->revision_width$}{}", "-", col_sep);
    print!("{:->author_width$}{}",   "-", col_sep);
    print!("{:->date_width$}{}",     "-", col_sep);
    println!("{:->size_width$}{}",   "-", col_sep);

    for Entry(prefix, opt_info) in &results {
        let loc = "^/".to_string() + prefix;
        if let Some(info) = opt_info {
            let size = info.size.map(|s| s.to_string()).unwrap_or("n/a".to_string());
            print!("{:location_width$}{}", (loc.as_str()).green(), col_sep);
            print!("{:>revision_width$}{}", info.commit_rev.yellow(), col_sep);
            print!("{:author_width$}{}",   info.commit_author.cyan(), col_sep);
            print!("{:date_width$}{}",     display_svn_datetime(&info.commit_date).magenta(), col_sep);
            println!("{:>size_width$}",     size);
                }
        else {
            println!("{:location_width$}{}{}", loc.green(), col_sep, "<does not exist>".red());
        }
    }
    Ok(())
}
