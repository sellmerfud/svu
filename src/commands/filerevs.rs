

use regex::Regex;
use anyhow::Result;
use clap::Parser;
use colored::*;
use crate::auth::Credentials;
use crate::util::{SvError::*, join_paths, display_svn_datetime};
use crate::svn::{self, Prefixes, SvnInfo};
use chrono::Local;
use std::fmt::Display;

/// Display commit revisions of files across tags and branches
#[derive(Debug, Parser)]
#[command(
    visible_aliases = ["revs"],
    author,
    help_template = crate::app::HELP_TEMPLATE,
    after_help = "\
    If no branches or tags are specified, then only the trunk\n\
    revision is displayed.\n\
    --branch and --tag may be specified multiple times.\n"
)]
pub struct Filerevs {
    /// Include branches that match <REGEX>
    #[arg(short, long = "branch", value_name = "REGEX")]
    branch_regexes: Vec<Regex>,

    /// Include tags that match <REGEX>
    #[arg(short, long = "tag", value_name = "REGEX")]
    tag_regexes: Vec<Regex>,

    /// Include all branches
    #[arg(short = 'B', long, conflicts_with = "branch_regexes")]
    all_branches: bool,

    /// Include all tags
    #[arg(short = 'T', long, conflicts_with = "tag_regexes")]
    all_tags: bool,

    /// PATH or URL to target file
    #[arg(num_args = 1..)]
    paths: Vec<String>,
}

impl Filerevs {
    pub fn run(&mut self) -> Result<()> {
        self.show_results()
    }

    fn show_results(&self) -> Result<()> {
        let creds = crate::auth::get_credentials()?;

        // First make sure all paths are rooted in the same repository
        let path_list = svn::info_list(&creds, &self.paths, None::<String>)?;
        let repo_uuid = &path_list[0].repo_uuid;
        for item in &path_list[1..] {
            if &item.repo_uuid != repo_uuid {
                return Err(
                    General("All paths must refer to the same repository.".to_string()).into()
                );
            }
        }

        let root_url = &path_list[0].root_url;
        let prefix_info = svn::load_prefixes()?;
        let branches = self.get_branches(
            &creds,
            root_url,
            self.all_branches,
            &self.branch_regexes,
            &prefix_info
        )?;
        let tags = self.get_tags(
            &creds,
            root_url,
            self.all_tags,
            &self.tag_regexes,
            &prefix_info
        )?;

        let prefixes: Vec<String> = vec![prefix_info.trunk_prefix]
            .into_iter()
            .chain(branches.chain(tags))
            .collect();
        let mut sorted_prefixes = prefixes.clone();
        sorted_prefixes.sort_by(|a, b| a.len().cmp(&b.len()).reverse()); // Sorteed by length longest first.

        for path_entry in &path_list {
            show_path_result(&creds, root_url, path_entry, &prefixes, &sorted_prefixes)?;
        }
        Ok(())
    }

    fn get_branches(
        &self,
        creds: &Option<Credentials>,
        root_url: &str,
        all: bool,
        regexes: &[Regex],
        prefixes: &Prefixes
    ) -> Result<impl Iterator<Item = String>> {
        let mut branches = Vec::<String>::new();
        if all || !regexes.is_empty() {
            let mut all_prefixes = prefixes.branch_prefixes.clone();
            all_prefixes.extend(prefixes.tag_prefixes.clone());
            let mut branch_prefixes = prefixes.branch_prefixes.clone();
            branch_prefixes.sort();
            let acceptable = |branch: &String| -> bool {
                !all_prefixes.contains(branch)
                    && (all || regexes.iter().any(|re| re.is_match(branch)))
            };

            for prefix in &branch_prefixes {
                let path_list = svn::path_list(creds, &join_paths(root_url, prefix))?;
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

    fn get_tags(
        &self,
        creds: &Option<Credentials>,
        root_url: &str,
        all: bool,
        regexes: &[Regex],
        prefixes: &Prefixes
    ) -> Result<impl Iterator<Item = String>> {
        let mut tags = Vec::<String>::new();
        if all || !regexes.is_empty() {
            let mut all_prefixes = prefixes.tag_prefixes.clone();
            all_prefixes.extend(prefixes.tag_prefixes.clone());
            let mut tag_prefixes = prefixes.tag_prefixes.clone();
            tag_prefixes.sort();
            let acceptable = |tag: &String| -> bool {
                !all_prefixes.contains(tag)
                    && (all || regexes.iter().any(|re| re.is_match(tag)))
            };

            for prefix in &tag_prefixes {
                let path_list = svn::path_list(creds, &join_paths(root_url, prefix))?;
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
}


//  We must determine the path to the file relative
//  to its subversion prefix.
//  Find find the url entry for one of our prefixes
//  so we can determine where the relative path starts.
fn get_svn_rel_path<S>(rel_url: &str, sorted_prefixes: &[S]) -> Result<String>
where
    S: AsRef<str> + Display,
{
    // Skip the leading ^/
    sorted_prefixes
        .iter()
        .find(|p| rel_url[2..].starts_with(p.as_ref()))
        .map(|p| rel_url[p.as_ref().len() + 3..].to_string())
        .ok_or(General(format!("Cannot determine relative path for {}", rel_url)).into())
}

fn max_width(label: &str, value_widths: impl Iterator<Item = usize>) -> usize {
    value_widths.fold(label.len(), |m, v| m.max(v))
}

// /this/is/the/users/path
// Location        Revision  Author  Date         Size
// --------------  --------  ------  -----------  ----------
// trunk               7601
// branches/8.1        7645
// tags/8.1.1-GA       7625
fn show_path_result(
    creds: &Option<Credentials>,
    root_url: &str,
    path_entry: &SvnInfo,
    prefixes: &[String],
    sorted_prefixes: &[String]
) -> Result<()> {
    use rayon::prelude::*;

    struct Entry(String, Option<Box<SvnInfo>>);

    let rel_path = &get_svn_rel_path(&path_entry.rel_url, sorted_prefixes)?;
    let results = prefixes
        .par_iter()
        .map(|prefix| {
            let path = join_paths(join_paths(root_url, prefix.as_str()), rel_path.as_str());
            let info = svn::info(creds, path.as_str(), Some("HEAD"))
                .ok()
                .map(Box::new);
            Entry(prefix.clone(), info)
        })
        .collect::<Vec<Entry>>();

    const LOCATION: &str = "Location";
    const REVISION: &str = "Revision";
    const AUTHOR: &str   = "Author";
    const DATE: &str     = "Date";
    const SIZE: &str     = "Size";

    let location_width = max_width(LOCATION, results.iter().map(|r| r.0.len() + 2));
    let revision_width = max_width(
        REVISION,
        results.iter().map(|r| match &r.1 {
            Some(info) => info.commit_rev.len(),
            None => 0,
        })
    );
    let author_width = max_width(
        REVISION,
        results.iter().map(|r| match &r.1 {
            Some(info) => info.commit_author.len(),
            None => 0,
        })
    );
    let date_width = display_svn_datetime(&Local::now()).len();
    let size_width = 8;
    let col_sep    = " ";

    println!();
    if path_entry.kind == "dir" {
        println!("{}", (rel_path.to_owned() + "/").blue());
    } else {
        println!("{}", rel_path.blue());
    }
    // Headers
    print!("{:location_width$}{}", LOCATION, col_sep);
    print!("{:revision_width$}{}", REVISION, col_sep);
    print!("{:author_width$}{}", AUTHOR, col_sep);
    print!("{:date_width$}{}", DATE, col_sep);
    println!("{:size_width$}", SIZE);

    print!("{:->location_width$}{}", "-", col_sep);
    print!("{:->revision_width$}{}", "-", col_sep);
    print!("{:->author_width$}{}", "-", col_sep);
    print!("{:->date_width$}{}", "-", col_sep);
    println!("{:->size_width$}{}", "-", col_sep);

    for Entry(prefix, opt_info) in &results {
        let loc = "^/".to_string() + prefix;
        if let Some(info) = opt_info {
            let size = info
                .size
                .map(|s| s.to_string())
                .unwrap_or("n/a".to_string());
            print!("{:location_width$}{}", (loc.as_str()).green(), col_sep);
            print!("{:>revision_width$}{}", info.commit_rev.yellow(), col_sep);
            print!("{:author_width$}{}", info.commit_author.cyan(), col_sep);
            print!("{:date_width$}{}", display_svn_datetime(&info.commit_date).magenta(), col_sep);
            println!("{:>size_width$}", size);
                }
        else {
            println!("{:location_width$}{}{}", loc.green(), col_sep, "<does not exist>".red());
        }
    }
    Ok(())
}