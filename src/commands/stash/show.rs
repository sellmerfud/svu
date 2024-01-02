
use clap::{Command, Arg, ArgMatches};
use super::*;
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

        println!("{:<11}| {}", stash_id_display(options.stash_id), stash.summary_display());
        println!("{:<11}| {}", "created", display_svn_datetime(&stash.date).magenta());
        println!("{:<11}| {}", "patch file", rel_patch.to_string_lossy().blue());
        println!("{:->70}", "-");
        for item in &stash.items {
            let mut pathname = item.path.clone();
            //  append '/' for directories
            if item.is_dir {
                pathname.push('/');
            }
            let path = Path::new(pathname.as_str());
            // First create the full path to the item relative to the working copy root.
            // Then make that relative to the current working directory.
            let rel_path  = diff_paths(&wc_root.join(path), &cwd).unwrap();
            let revision  = match item.status.as_str() {
                UNVERSIONED => "unversioned",
                ADDED       => "new",
                _           => item.revision.as_str()
            };
            let color = item.status_color();
            println!("{} {} [{}]", item.status_letter().color(color), rel_path.to_string_lossy().color(color), revision.yellow())
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
        let msg = format!("{} does not exist in the stash", stash_id_display(options.stash_id));
        Err(General(msg).into())
    }

}