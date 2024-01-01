
use clap::{Command, Arg, ArgMatches};
use super::*;
use regex::Regex;
use anyhow::Result;
use crate::util::{display_svn_datetime, print_diff_line};
use crate::svn::{self, workingcopy_root};
use std::env::current_dir;
use std::io::{BufReader, BufRead};
use pathdiff::diff_paths;
pub struct Show;

struct Options {
    show_diff:  bool,
    stash_id:   usize,
}


fn parse_stash_id(arg: &str) -> Result<usize> {
    let re = Regex::new(r"^(?:stash-)?(\d+)$")?;
    if let Some(captures) = re.captures(arg) {
        let id = captures .get(1).unwrap() .as_str().parse::<usize>()?;
        Ok(id)
    }
    else {
        Err(General("Stash id must be 'stash-<n>' or '<n>'".to_string()).into())
    }
}

impl StashCommand for Show {
    fn name(&self) -> &'static str { "show" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Show the details of a stash entry")
            .arg(
                Arg::new("diff")
                    .short('d')
                    .long("diff")
                    .action(clap::ArgAction::SetTrue)
                    .help("Display the patch file differences"),
            )
            .arg(
                Arg::new("stash-id")
                    .help("Id of the stash you wish to show.\n\
                           Can be 'stash-<n>' or simply <n> where <n> is the stash number.\n\
                           If omitted, stash-0 is shown by default.")
                    .value_parser(parse_stash_id)
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {

        do_show(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    Options {
        show_diff: matches.get_flag("diff"),
        stash_id:  matches.get_one::<usize>("stash-id").copied().unwrap_or(0)
    }
}


fn do_show(options: &Options) -> Result<()> {
    
    svn::working_copy_info()?;  // Make sure we are in a working copy.
    let stash_entries = load_stash_entries()?;

    if options.stash_id < stash_entries.len() {
        let cwd        = current_dir()?;
        let wc_root    = workingcopy_root(&cwd).unwrap();
        let stash      = &stash_entries[options.stash_id];
        let patch_file = stash_path()?.join(stash.patch_name.as_str());
        let rel_patch  = diff_paths(&patch_file, &cwd).unwrap();

        println!("stash     : {}", stash.summary_display());
        println!("created   : {}", display_svn_datetime(&stash.date).magenta());
        println!("patch file: {}", rel_patch.to_string_lossy().blue());
        println!("{:->70}", "-");
        for item in &stash.items {
            // First create the full path to the item relative to the working copy root.
            // Then make that relative to the current working directory.
            let rel_path  = diff_paths(&wc_root.join(&item.path), &cwd).unwrap();
            let revision  = if item.status == UNVERSIONED {
                "unversioned"
            } else {
                item.revision.as_str()
            };
            println!("{} {} [{}]", item.status_display(), rel_path.to_string_lossy(), revision.yellow())
        }

        if options.show_diff {
            println!();
            let file = File::open(patch_file)?;
            for line in BufReader::new(file).lines() {
                print_diff_line(line?.as_str());
            }
        }

        Ok(())
    }
    else {
        let msg = format!("stash-{} does not exist in the stash", options.stash_id);
        Err(General(msg).into())
    }

}