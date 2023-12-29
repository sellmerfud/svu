
use anyhow::Result;
use clap::{Command, Arg, ArgMatches};
use crate::svn;
use crate::util;
use super::SvCommand;

pub struct Show;
#[derive(Debug)]
struct Options {
    revision:    Option<String>,
    paths:       Vec<String>,
    show_diff:   bool,
    show_paths:  bool,
    show_msg:    bool,
}

impl Options {
    fn build_options(matches: &ArgMatches) -> Options {
        let revision = matches.get_one::<String>("revision").map(|s| s.clone());
        let paths = match matches.get_many::<String>("paths") {
            Some(paths) => paths.map(|s| s.to_owned()).collect(),
            None => vec![]
        };

        Options {
            revision,
            paths,
            show_diff:   matches.get_flag("show-diff"),
            show_paths:  matches.get_flag("show-paths"),
            show_msg:    !matches.get_flag("no-message"),
        }
    }
}

impl SvCommand for Show {
    fn name(&self) -> &'static str { "show" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Show the details of a commit")
            .after_help("The revision defaults to the current working copy revision.\n\
            If no revision is given and the first path looks like a revision it will be treated as one.\n\
            If no path is given it defaults to the current working copy directory.")
            .arg(
                Arg::new("revision")
                .short('r')
                .long("revision")
                .value_name("rev")
                .action(clap::ArgAction::Append)
                .help("The commit revision")
            )
            .arg(
                Arg::new("show-diff")
                    .short('d')
                    .long("diff")
                    .action(clap::ArgAction::SetTrue)
                    .help("Show the diff output of the commit")
            )
            .arg(
                Arg::new("show-paths")
                    .short('p')
                    .long("paths")
                    .action(clap::ArgAction::SetTrue)
                    .help("Show the path affected by the commit")
            )
            .arg(
                Arg::new("no-message")
                    .short('n')
                    .long("no-message")
                    .action(clap::ArgAction::SetTrue)
                    .help("Do not display the commit message")
            )
            .arg(
                Arg::new("paths")
                    .value_name("PATH")
                    .action(clap::ArgAction::Append)
                    .help("Limit commit to a given path or repositiory url")
            )
    }

    fn run(&self, matches: &ArgMatches) -> Result<()> {
        //Show::show_results(&Options::build_options(matches))
        let options = Options::build_options(matches);
        let mut revision = options.revision.clone();
        let mut paths = options.paths.clone();
        
        //  If no revisions are specified and the first 'path' looks like a revision
        //  then treat it as one, appending :0 if it does not have a range.
        if revision.is_none() &&
           !paths.is_empty() &&
           svn::looks_like_revision(paths[0].as_str()) {
            revision = Some(paths.remove(0));
        };
        if paths.is_empty() {
            paths.push(".".to_owned());
        }

        //  Resolve the revision if necessary and coerce it into
        //  a vector
        //  In some cases when the revision is PREV, it may not produce a log entry 
        //  even though 'svn info' would succeed.  To work around this oddity
        //  we append :0 to the revision and limit the log to 1 entry.
        // let rr = if r.contains(':') { r } else { r.clone() + ":0"};
        let rev_vector: Vec<String> = revision
        .into_iter()
        .flat_map(|r| svn::resolve_revision_string(r.as_str(), paths[0].as_str()))
        .map(|r| if r.contains(':') { r } else { r.clone() + ":0"})
        .collect();

        let log_entry = &svn::log(&paths, &rev_vector, true, Some(1), false, true)?[0];
        util::show_commit(&log_entry, options.show_msg, options.show_paths);
        if options.show_diff {            
            println!();
            let lines = svn::change_diff(&paths[0], &log_entry.revision)?;
            for line in &lines {
                util::print_diff_line(line);
            }
        }
        Ok(())
    }
}

