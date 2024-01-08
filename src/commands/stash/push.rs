
use clap::{Command, Arg, ArgMatches};
use crate::svn;
use std::path::Path;
use super::*;
use anyhow::Result;
use uuid::Uuid;

pub struct Push;


struct Options {
    unversioned:        bool,
    revert_workingcopy: bool,
    description:        Option<String>,
}

impl Options {
    fn build_options(matches: &ArgMatches) -> Options {

        let description = matches.get_one::<String>("message").map(|m| m.to_owned());

        Options {
            unversioned:        matches.get_flag("unversioned"),
            revert_workingcopy: !matches.get_flag("no-revert"),
            description,
        }
    }
}

impl Push {
    //  Return vector of clap arguments for the push command.
    pub fn push_args() -> Vec<Arg> {
        vec![
            Arg::new("unversioned")
                .short('u')
                .long("unversioned")
                .action(clap::ArgAction::SetTrue)
                .help("Include unversioned files in the stash"),
            Arg::new("message")
                .short('m')
                .long("message")
                .help("A short description of the stash"),
            Arg::new("no-revert")
                .short('n')
                .long("no-revert")
                .action(clap::ArgAction::SetTrue)
                .help("Do not revert the working copy"),
        ]
    }
}

impl StashCommand for Push {
    fn name(&self) -> &'static str { "push" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Push the working copy to the stash and revert the working copy")
            .args(Push::push_args())
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_command(&Options::build_options(matches))?;
        Ok(())
        // show_results(&Options::build_options(matches))
    }
}

fn get_log_message_1st(wc_root: &Path) -> Result<String> {
    let log = svn::log(&None, &[wc_root.to_string_lossy()], &["BASE:0".into()], true, Some(1), false, false)?;
    Ok(log[0].msg_1st())
  }

fn create_patch_name() -> String {
    format!("{}.patch", Uuid::new_v4())
}

fn do_command<'a>(options: &Options) -> Result<()> {
    svn::workingcopy_info()?;  // Make sure we are in a working copy.
    let wc_root = svn::workingcopy_root(Path::new(".")).unwrap();
    let items = get_stash_items(&wc_root, options.unversioned)?;

    if items.is_empty() {
        println!("No local changes to save");
    }
    else {
        let (branch, revision) = svn::current_branch(&wc_root)?;
        let description = options.description
            .clone()
            .unwrap_or(get_log_message_1st(&wc_root)?);

        let stash_path = stash_path()?;
        let patch_name = create_patch_name();

        svn::create_patch(&stash_path.join(patch_name.as_str()), &wc_root)?;
        
        let stash = StashFileEntry {
            branch,
            revision,
            description,
            date: Local::now(),
            patch_name,
            items: items.clone(),
        };
        add_stash_entry(&stash)?;

        if options.revert_workingcopy {
            // Lastly we revert the working copy.
            // We will explicitly revert all entries to ensure that the --remove-added flag is honored.
            // For added/unversioned directories we do not need to revert any entries below them
            // as these entries will be reverted recursively with their respective  directories.
            let added_unversioned: Vec<StashItem> = items
                .iter()
                .filter(|i| i.is_dir && (i.status == ADDED || i.status == UNVERSIONED))
                .map(|i| i.clone())
                .collect();
            let can_skip = |i: &StashItem| -> bool {
                added_unversioned.iter().any(|p| i.path.starts_with(&p.path) && i.path != p.path)
            };
            let revert_paths: Vec<String> = items.iter().filter(|i| !can_skip(i)).map(|i| i.path.clone()).collect();
            svn::revert(&revert_paths, "infinity", true, Some(&wc_root))?;
        }

        println!("Saved working copy state - {}", stash.summary_display());
    }
    Ok(())
}


