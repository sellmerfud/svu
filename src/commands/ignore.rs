
use anyhow::Result;
use clap::{Command, Arg, ArgMatches};
use crate::auth::{Credentials, push_creds};
use crate::svn;
use crate::util;
use crate::util::SvError::*;
use std::path::Path;
use std::fmt::Display;
use super::SvCommand;

pub struct Ignore;
struct Options {
    path:        String,
}

impl Options {
    fn build_options(matches: &ArgMatches) -> Options {
        let path = matches.get_one::<String>("path").map(|s| s.clone()).unwrap_or(".".to_string());
        Options { path }
    }
}

impl SvCommand for Ignore {
    fn name(&self) -> &'static str { "ignore" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Write svn:ignore properties to stdout in .gitignore format")
            .after_help("PATH must refer to a working directory (not a repository URL.\n\
                        If PATH is ommitted the current working directory is used by default."
            )
            .arg(
                Arg::new("path")
                .value_name("PATH")
                .help("Limit commits to given paths (default is '.')")
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        write_ignore_entries(&Options::build_options(matches))
    }
}

fn is_directory<S>(path: S) -> bool
    where S: AsRef<str> + Display {
        Path::new(path.as_ref()).is_dir()
}

fn is_working_directory(creds: &Option<Credentials>, path: &str) -> Result<bool> {
    let info = svn::info(creds, path, None)?;
    Ok(info.wc_path.is_some() && info.kind == "dir")
}

fn get_ignores(creds: &Option<Credentials>, path: &str, global: bool) -> Result<Option<String>> {
    let prop = (if global { "svn:global-ignores" } else { "svn:ignore" }).to_owned();
    let mut args = Vec::new();
    args.push("pget".to_owned());
    push_creds(&mut args, creds);
    args.push(prop);
    args.push(path.to_string());
    let output = svn::run_svn(&args, svn::USE_CWD)?;
    if output.status.success() {
        Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
    }
    else {
        Ok(None)
    }

}

fn write_ignore_entries(options: &Options) -> Result<()> {
    let creds = crate::auth::get_credentials()?;
    let prefix_len = options.path.trim_end_matches('/').len() + 1; // Add one for trailing slash

    fn ignore_lines(lines: &String) -> impl Iterator<Item = &str> {
        lines
        .split("\n")
        .map(|l| l.trim())  // Clean up and skip blank lines
        .filter(|l| !l.is_empty())
        .into_iter()
    }

    fn svn_ignore(creds: &Option<Credentials>, dir_path: &str, prefix_len: usize) -> Result<()> {
        let print_ignores = |global: bool| -> Result<()> {
            if let Some(ignore_output) = get_ignores(creds, dir_path, global)? {
                for ignore in ignore_lines(&ignore_output) {
                    let ignore_path  = if global {
                        // Start path with ** for global ignores
                        let base_path   = util::join_paths(dir_path, "**");
                        util::join_paths(base_path, ignore.to_owned().trim_end_matches('/'))
                    } else {
                        util::join_paths(dir_path, ignore.to_owned().trim_end_matches('/'))
                    };
                    //  Directory entries end with a slash
                    let suffix = if is_directory(&ignore_path) { "/" } else { "" };
                    // We prefix each path with a slash so that it refers to the
                    // specific entry as per .gitignore rules.
                    // See: https://git-scm.com/docs/gitignore
                    println!("/{}{}", &ignore_path[prefix_len..], suffix);
                }                
            }
            Ok(())                        
        };

        print_ignores(false)?;
        print_ignores(true)?;

        //  Recursively process all subdirectories
        let path_list = svn::path_list(&creds, dir_path)?;
        for sub_dir in &path_list.entries {
            if sub_dir.kind == "dir" {
                let subdir_path = util::join_paths(dir_path, sub_dir.name.trim_end_matches('/'));
                svn_ignore(creds, &subdir_path, prefix_len)?;
            }
        }
        Ok(())
    }

    if !is_working_directory(&creds, &options.path)? {
        let msg  = format!("{} is not a subversion working copy directory", options.path);
        Err(General(msg).into())
    }
    else {
        svn_ignore(&creds, &options.path, prefix_len)
    }
}