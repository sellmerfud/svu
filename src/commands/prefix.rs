
use anyhow::Result;
use clap::{Command, Arg, ArgMatches};
use crate::svn;
use crate::util::SvError::*;

use super::SvCommand;


pub struct Prefix;

#[derive(Debug)]
struct Options {
    // #[arg(value_parser = parse_prefix)]
    add_branch: Vec<String>,
    rem_branch: Vec<String>,
    add_tag:    Vec<String>,
    rem_tag:    Vec<String>,
    trunk:      Option<String>,
}

impl Options {
    fn build_options(matches: &ArgMatches) -> Options {

        let add_branch = match matches.get_many::<String>("add-branch") {
            Some(prefixes) => prefixes.map(|s| s.to_owned()).collect(),
            None => vec![]
        };

        let rem_branch = match matches.get_many::<String>("rem-branch") {
            Some(prefixes) => prefixes.map(|s| s.to_owned()).collect(),
            None => vec![]
        };

        let add_tag = match matches.get_many::<String>("add-tag") {
            Some(prefixes) => prefixes.map(|s| s.to_owned()).collect(),
            None => vec![]
        };

        let rem_tag = match matches.get_many::<String>("rem-tag") {
            Some(prefixes) => prefixes.map(|s| s.to_owned()).collect(),
            None => vec![]
        };

        let trunk = matches.get_one::<String>("set-trunk")
                           .map(|p| p.to_owned());

        Options {
            add_branch,
            rem_branch,
            add_tag,
            rem_tag,
            trunk,
        }
    }
}

fn parse_prefix(arg: &str) -> Result<String> {
    if !arg.starts_with("^/") {
        Err(General("Prefix must begin with '^/'".to_string()).into())
    }
    else if arg.len() == 2 {
        Err(General("Prefix cannot refer to the repository root".to_string()).into())
    }
    else {
        Ok(arg[2..].trim_end_matches("/").to_string())
    }
}

impl SvCommand for Prefix {
    fn name(&self) -> &'static str { "prefix" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Display and configure repository prefixes")
            .after_help("By default `sv` assumes that your repository is using the default prefixes:\n\
                         ^/trunk\n\
                         ^/branches\n\
                         ^/tags\n\
                         \n\
                         You can use this command to configure other prefixes so that the `branch` and\n\
                         `filerevs` commands can find them.\n\
                         \n\
                         All prefixes must start with '^/'")
            .arg(
                Arg::new("add-branch")
                    .long("add-branch")
                    .value_name("prefix")
                    .action(clap::ArgAction::Append)
                    .value_parser(parse_prefix)
                    .help("Add a branch prefix")
            )
            .arg(
                Arg::new("rem-branch")
                    .long("rem-branch")
                    .value_name("prefix")
                    .value_parser(parse_prefix)
                    .action(clap::ArgAction::Append)
                    .help("Remove a branch prefix")
            )
            .arg(
                Arg::new("add-tag")
                    .long("add-tag")
                    .value_name("prefix")
                    .value_parser(parse_prefix)
                    .action(clap::ArgAction::Append)
                    .help("Add a tag prefix")
            )
            .arg(
                Arg::new("rem-tag")
                    .long("rem-tag")
                    .value_name("prefix")
                    .value_parser(parse_prefix)
                    .action(clap::ArgAction::Append)
                    .help("Remove a tag prefix")
            )
            .arg(
                Arg::new("set-trunk")
                    .long("set-trunk")
                    .value_name("prefix")
                    .value_parser(parse_prefix)
                    .help("Set the trunk prefix")
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        prefix_operations(&Options::build_options(matches))
    }
}


fn prefix_operations(options: &Options) -> Result<()> {
    let mut prefixes = svn::load_prefixes()?;
    let mut modified = false;

    if let Some(trunk_prefix) = &options.trunk {
        prefixes.trunk_prefix = trunk_prefix.clone();
        modified = true;
    }

    if !options.add_branch.is_empty() || !options.rem_branch.is_empty() {
        let to_add: Vec<String> = options.add_branch
            .iter()
            .filter(|a| !prefixes.branch_prefixes.contains(a))
            .map(|e| e.clone()).collect();
        prefixes.branch_prefixes.extend(to_add);

        prefixes.branch_prefixes = prefixes.branch_prefixes
            .into_iter()
            .filter(|e| !options.rem_branch.contains(&e))
            .collect();

        if prefixes.branch_prefixes.is_empty() {
            prefixes.branch_prefixes.push("branches".to_string());
        }
        modified = true;
    }

    if !options.add_tag.is_empty() || !options.rem_tag.is_empty() {
        let to_add: Vec<String> = options.add_tag
            .iter()
            .filter(|a| !prefixes.tag_prefixes.contains(a))
            .map(|e| e.clone()).collect();
        prefixes.tag_prefixes.extend(to_add);

        prefixes.tag_prefixes = prefixes.tag_prefixes
            .into_iter()
            .filter(|e| !options.rem_tag.contains(&e))
            .collect();
        
        if prefixes.tag_prefixes.is_empty() {
            prefixes.tag_prefixes.push("tags".to_string());
        }
        modified = true;
    }

    if modified {
        svn::save_prefixes(&prefixes)?;
    }

    //  Finally display all of the configured prefixes to stdout.
    println!("Trunk prefix");
    println!("-----------------------------------------");
    println!("^/{}", prefixes.trunk_prefix);

    println!("\nBranch prefixes");
    println!("-----------------------------------------");
    let mut sorted = prefixes.branch_prefixes;
    sorted.sort();
    for prefix in &sorted {
        println!("^/{}", prefix);
    }

    println!("\nTag prefixes");
    println!("-----------------------------------------");
    let mut sorted = prefixes.tag_prefixes;
    sorted.sort();
    for prefix in &sorted {
        println!("^/{}", prefix);
    }
    Ok(())
}