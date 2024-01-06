
use clap::{Command, Arg, ArgMatches};
use super::*;
use anyhow::Result;

pub struct Bad;

struct Options {
    revision: Option<String>,
}

static mut ALIAS: String = String::new();

impl BisectCommand for Bad {
    fn name(&self) -> &'static str { "bad" }

    fn clap_command(&self) -> Command {
        let cmd = Command::new(self.name())
            .about("Mark a revision as bad (It contains the bug)")
            .arg(
                Arg::new("revision")
                .value_name("REVISION")
                .help("The bad revision.\n\
                       If not specified, the current working copy revision is used.")
            );

        if let Some(data) = load_bisect_data().ok().flatten() {
            if let Some(name) = data.term_bad {
                // The clap library call requires a &st reference
                // so we must use a static to ensure that it is not dropped.
                unsafe { 
                    ALIAS = name.clone();
                    return cmd.alias(ALIAS.as_str());
                }
            }
        }
        cmd

    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        do_work(&build_options(matches))?;
        Ok(())
    }
}

fn build_options(matches: &ArgMatches) -> Options {
    Options {
        revision: matches.get_one::<String>("revision").map(|s| s.to_string())
    }
}

fn do_work(options: &Options) -> Result<()> {
    let wc_info = svn::workingcopy_info()?;  // Make sure we are in a working copy.
    let wc_root = svn::workingcopy_root(&current_dir()?).unwrap();
    let data = get_bisect_data()?;
    let revision = match &options.revision {
        Some(rev) => svn::resolve_revision(&rev, &wc_root.to_string_lossy().as_ref())?,
        None      => wc_info.commit_rev,
    };

      // The new bad revision can come after the existing maxRev
      // This allows the user to recheck a range of commits.
      // The new bad revision cannot be less than or equal to the minRev
    if data.min_rev.is_some() && to_rev_num(&revision) <= to_rev_num(&data.min_rev.as_ref().unwrap())  {
        println!("The '{}' revision must be more recent than the '{}' revision", data.bad_name(), data.good_name());
    }
    else {
        let _ = mark_bad_revision(&revision);
        log_bisect_command(&std::env::args().collect::<Vec<String>>())?;
    }
    
    let data = get_bisect_data()?; // Fresh copy of data
    if let Some(status) = get_waiting_status(&data) {
        append_to_log(format!("# {}", status))?;
        println!("{}", status);
    }
    Ok(())

}
