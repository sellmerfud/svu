
use std::path::Path;
use std::env::current_dir;
use regex::Regex;
use anyhow::Result;
use clap::Parser;
use crate::auth::Credentials;
use crate::util;
use crate::util::SvError::*;
use crate::svn;
use colored::*;
use std::fmt::Display;

/// Display current branch or list branches and tags.
///
/// With no options, this command will show the current branch checked
/// out to your working copy.
///
/// With options it can be used to list all of the branches and/or tags in
/// the repository. By default this is based on the standard repository structure
/// (^/trunk, ^/branches, ^/tags) but see the `prefix` command if you are using
/// non-standard prefixes for branches and tags.
#[derive(Debug, Parser)]
#[command(
    author,
    help_template = crate::app::HELP_TEMPLATE,
)]
pub struct Branch {
    /// Display branches that match <REGEX>.
    ///
    /// If multiple --branch options are given, then branches matching any
    /// one of the regular expressions are listed.
    #[arg(short, long = "branch", value_name = "REGEX")]
    branch_regexes: Vec<Regex>,

    /// Display tags that match <REGEX>.
    ///
    /// If multiple --tag options are given, then tags matching any
    /// one of the regular expressions are listed.
    #[arg(short, long = "tag", value_name = "REGEX")]
    tag_regexes: Vec<Regex>,

    /// Display all branches in the repository.
    #[arg(short = 'B', long, conflicts_with = "branch_regexes")]
    all_branches: bool,

    /// Display all tags in the repository.
    #[arg(short = 'T', long, conflicts_with = "tag_regexes")]
    all_tags: bool,

    /// Path to working copy directory
    #[arg(default_value = ".")]
    path: String,


}

impl Branch {
    fn no_arguments(&self) -> bool {
        !self.all_branches
            && !self.all_tags
            && self.branch_regexes.is_empty()
            && self.tag_regexes.is_empty()
    }

    fn list_branches(&self) -> bool {
        self.all_branches || !self.branch_regexes.is_empty()
    }

    fn list_tags(&self) -> bool {
        self.all_tags || !self.tag_regexes.is_empty()
    }

    pub fn run(&mut self) -> Result<()> {
        if self.no_arguments() {
            self.show_current_branch()
        } else {
            let creds = crate::auth::get_credentials()?;
            self.show_list(&creds)
        }
    }

    fn show_current_branch(&self) -> Result<()> {
        if self.path.starts_with('^') || self.path.contains("://") {
            return Err(General("Cannot show the current branch of a URL".to_string()).into())
        }

        let mut path_buf;
        let mut path = Path::new(self.path.as_str());
        if path.is_relative() {
            path_buf = current_dir()?;
            path_buf.push(path);
            path = path_buf.as_path();
        }

        let (name, revision) = svn::current_branch(path)?;
        println!("Current branch: {} [{}]", name.green(), revision.yellow());
        Ok(())
    }


    fn show_list(&self, creds: &Option<Credentials>) -> Result<()> {

        let base_url = svn::info(creds, &self.path, None)?.root_url;
        let prefixes = svn::load_prefixes()?;
        let mut all_prefixes = prefixes.branch_prefixes.clone();
        all_prefixes.extend(prefixes.tag_prefixes.clone());

        if self.list_branches() {
            let mut sorted_prefixes = prefixes.branch_prefixes.clone();
            sorted_prefixes.sort();
            self.list_entries(
                creds,
                "Branches",
                &base_url,
                &sorted_prefixes,
                &self.branch_regexes,
                &all_prefixes
            )?
        }
        if self.list_tags() {
            let mut sorted_prefixes = prefixes.tag_prefixes.clone();
            sorted_prefixes.sort();
            self.list_entries(
                creds,
                "Tags",
                &base_url,
                &sorted_prefixes,
                &self.tag_regexes,
                &all_prefixes
            )?
        }
        Ok(())
    }

    fn list_entries<S, T>(
        &self,
        creds: &Option<Credentials>,
        header: &str,
        base_url: &str,
        prefixes: &[S],
        regexes: &[Regex],
        all_prefixes: &[T],
    ) -> Result<()>
    where
        S: AsRef<str> + Display,
        T: AsRef<str> + Display + PartialEq<str>,
    {
        //  If a path matches one of the branch/tag prefixes then we do not consider it
        //  an acceptable entry.  Also the entry must match the regex if present.
        let acceptable = |path: &str| -> bool {
            !all_prefixes.iter().any(|p| p.eq(path))
                && (regexes.is_empty() || regexes.iter().any(|r| r.is_match(path)))
        };

        println!();
        println!("{}", header);
        println!("{}", util::divider(60));

        for prefix in prefixes {
            let relative_prefix = format!("^/{prefix}");
            let path_list = svn::path_list(creds, util::join_paths(base_url, prefix).as_str())?;
            for entry in path_list.entries {
                let path = &util::join_paths(&relative_prefix, entry.name);
                if acceptable(path.as_str()) {
                    println!("{}", path.green());
                }
            }
        }
        Ok(())
    }
}



