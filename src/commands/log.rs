
use regex::Regex;
use anyhow::Result;
use clap::{Command, Arg, ArgMatches};
use crate::svn::{self, LogEntry};
use crate::util::{self, StringWrapper};
use colored::*;
use chrono::{DateTime, Local};
use super::SvCommand;

pub struct Log;
struct Options {
    limit:        Option<u16>,
    author:       bool,
    date:         bool,
    time:         bool,
    full:         bool,
    show_paths:   bool,
    stop_on_copy: bool,
    reverse:      bool,
    revisions:    Vec<String>,
    regexes:      Vec<Regex>,
    paths:        Vec<String>,
}

impl Options {
    fn build_options(matches: &ArgMatches) -> Options {
        let revisions = if matches.get_flag("incoming") {
            vec!["HEAD:BASE".to_owned()]
        } else {
            match matches.get_many::<String>("revision") {
                Some(revs) => revs.map(|s| s.to_owned()).collect(),
                None => vec![]
            }
        };

        let regexes = match matches.get_many::<Regex>("match") {
            Some(regexes) => regexes.map(|r| r.to_owned()).collect(),
            None => vec![]
        };

        let paths = match matches.get_many::<String>("paths") {
            Some(paths) => paths.map(|s| s.to_owned()).collect(),
            None => vec![]
        };

        let verbose = matches.get_flag("verbose");

        Options {
            author:       matches.get_flag("author") || verbose,
            time:         matches.get_flag("time")   || verbose,
            full:         matches.get_flag("full")   || verbose,
            date:         matches.get_flag("date"),
            reverse:      matches.get_flag("reverse"),
            show_paths:   matches.get_flag("show-paths"),
            stop_on_copy: matches.get_flag("stop-on-copy"),
            limit:        matches.get_one::<u16>("limit").copied(),
            revisions,
            regexes,
            paths
        }
    }
}

impl SvCommand for Log {
    fn name(&self) -> &'static str { "log" }

    fn clap_command(&self) -> Command {
        Command::new(self.name())
            .about("Display formatted log entries")
            .after_help("By default shows only the first line of each commit message (see --full)\n\
                        If only 1 revision is given and it is not a range then :0 is appended to make it a range.\n\
                        If no revision is given and the first path looks like a revision it will be treated as one."
            )
            .arg(
                Arg::new("limit")
                    .short('l')
                    .long("limit")
                    .value_name("num")
                    .value_parser(clap::value_parser!(u16).range(1..))
                    .help("Limit the number of commits displayed")
            )
            .arg(
                Arg::new("author")
                    .short('a')
                    .long("author")
                    .action(clap::ArgAction::SetTrue)
                    .help("Display the author of each commit")
            )
            .arg(
                Arg::new("date")
                    .short('d')
                    .long("date")
                    .action(clap::ArgAction::SetTrue)
                    .conflicts_with_all(&["time", "verbose"])
                    .help("Display the date of each commit")
            )
            .arg(
                Arg::new("time")
                    .short('t')
                    .long("time")
                    .action(clap::ArgAction::SetTrue)
                    .help("Display the date and time of each commit")
            )
            .arg(
                Arg::new("full")
                    .short('f')
                    .long("full")
                    .action(clap::ArgAction::SetTrue)
                    .help("Display full commit messages")
            )
            .arg(
                Arg::new("show-paths")
                    .short('p')
                    .long("paths")
                    .action(clap::ArgAction::SetTrue)
                    .help("Display the paths affected by each commit")
            )
            .arg(
                Arg::new("verbose")
                    .short('v')
                    .long("verbose")
                    .action(clap::ArgAction::SetTrue)
                    .help("Shorthand for --author --time --full")
            )
            .arg(
                Arg::new("revision")
                .short('r')
                .long("revision")
                .value_name("rev")
                .action(clap::ArgAction::Append)
                .help("Specify a revision or a range of revisions")
            )
            .arg(
                Arg::new("reverse")
                .long("reverse")
                .action(clap::ArgAction::SetTrue)
                .help("Output the commits in the reverse order")
            )
            .arg(
                Arg::new("incoming")
                .short('i')
                .long("incoming")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("revision")
                .help("Display commits incoming with next update")
            )
            .arg(
                Arg::new("stop-on-copy")
                .long("stop-on-copy")
                .action(clap::ArgAction::SetTrue)
                .help("Do not cross copies while traversing history")
            )
            .arg(
                Arg::new("match")
                .short('m')
                .long("match")
                .value_name("regex")
                .value_parser(Regex::new)
                .action(clap::ArgAction::Append)
                .help("Limits commits to those with a message matching a regular expression")
            )
            .arg(
                Arg::new("paths")
                .value_name("PATH")
                .action(clap::ArgAction::Append)
                .help("Limit commits to given paths (default is '.')")
            )
    }
        
    fn run(&self, matches: &ArgMatches) -> Result<()> {
        show_results(&Options::build_options(matches))
    }
}

fn get_log_entries(options: &Options) -> Result<Vec<LogEntry>> {
    let mut revisions = options.revisions.clone();
    let mut paths = options.paths.clone();
    
    //  If no revisions are specified and the first 'path' looks like a revision
    //  then treat it as one, appending :0 if it does not have a range.
    if revisions.is_empty() && 
        !paths.is_empty() &&
        svn::looks_like_revision(paths[0].as_str()) {
        revisions = vec![paths.remove(0)];
    };

    //  Resolve any revisions that contains names such as HEAD or
    // that contain rev-3 type expressions.
    let resolve_path = paths.first().map(|p| p.as_str()).unwrap_or(".");
    revisions = revisions.into_iter().flat_map(|r| {
        svn::resolve_revision_string(r.as_str(), resolve_path)
    }).collect();

    if revisions.len() == 1 && !revisions[0].contains(':') {
        revisions[0] = format!("{}:0", revisions[0]);
    }

    let entries = svn::log(
        &paths,
        &revisions,
        true,  // include_msg
        options.limit,
        options.stop_on_copy,
        options.show_paths)?;

    //  Check any regular expressions entered by the user.
    //  Include the entry if it matches at least one of them.
    if options.regexes.is_empty() {
        Ok(entries)
    } else {
        let matching = |entry: &LogEntry| -> bool {
            let msg = entry.msg.join("\n");
            options.regexes.iter().any(|r| r.is_match(msg.as_str()))
        };
        let new_entries = entries.into_iter().filter(matching).collect();
        Ok(new_entries)
    }
}

fn show_results(options: &Options) -> Result<()> {

    fn parent_dir(path: &str) -> String {
        let re = Regex::new(r"^(.*)/[^/]+").expect("Error parsing parent_dir regex");
        let mut local_path = path.to_owned();
        local_path = local_path.chomp('/').to_owned();

        if let Some(caps) = re.captures(&local_path) {
            caps[1].to_owned()
        }
        else {
            ".".to_owned()
        }
    }

    let mut entries = get_log_entries(options)?;

    //  In the case where we are showing `incoming` commits
    //  we will have a single revision of "HEAD:BASE".
    //  It it possible that the "BASE" revision already exists
    //  in the working copy and thus will not be `incoming` so
    //  we do not want to display it.
    let omit_rev = if !entries.is_empty() &&
                        options.revisions.len() == 1 &&
                        options.revisions[0] == "HEAD:BASE" {
        let wc_path = options.paths.first().map(|p| p.as_str()).unwrap_or(".");
        let path_info = svn::info(wc_path, None)?;
        if path_info.kind == "dir" {
            Some(path_info.commit_rev)
        } else {
            let parent_info = svn::info(&parent_dir(&wc_path), None)?;
            Some(parent_info.commit_rev)
        }
    } else {
        None
    };
    
    //  Get the length of the longest revision string and author name
    let (max_rev_len, max_author_len) = entries.iter().fold((0, 0), |(max_r, max_a), e| {
        (max_r.max(e.revision.len()), max_a.max(e.author.len()))
    });

    let build_prefix = |revision: &String, author: &String, date: &DateTime<Local>| -> String {

        let rev_str    = format!("{:width$}", revision.yellow(), width=max_rev_len);
        let author_str = format!("{:width$}", author.cyan(), width=max_author_len);
        let date_str   = if options.time {
            util::display_svn_datetime(date).magenta()
        } else {
            util::display_svn_date(date).magenta()
        };


        match (options.author, options.date||options.time) {
            (true, true) => format!("{} {} {}", rev_str, author_str, date_str),
            (true, false) => format!("{} {}", rev_str, author_str),
            (false, true) => format!("{} {}", rev_str, date_str),
            (false, false)=> rev_str
        }
    };

    if options.reverse {
        entries.reverse();
    }
    
    for LogEntry { revision, author, date, msg, paths } in &entries {
        if Some(revision) != omit_rev.as_ref() {
            let msg_1st = msg.first().map(|s| s.as_str()).unwrap_or("");
            let prefix  = build_prefix(revision, author, date);

            if options.full {
                println!("\n{}", prefix);
                for line in msg {
                    println!("{}", line);
                }
            }
            else {
                println!("{} {}", prefix, msg_1st);
            }

            if options.show_paths {
                for path in paths {
                    println!("{}", util::formatted_log_path(path))
                }
            }
        }
    }

    Ok(())
}
