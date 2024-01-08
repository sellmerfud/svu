
use std::process;
use anyhow::Result;
use util::SvError;

pub mod app;
pub mod util;
pub mod commands;
pub mod svn;
pub mod auth;

fn run() -> Result<()> {
    let app = app::App::new();
    let matches = app.clap.clone().get_matches();
    if let Some((name, matches)) = matches.subcommand() {
        if let Some(command) = app.commands.iter().find(|cmd| cmd.name() == name) {
            command.run(matches)
        } else {
            Err(SvError::General(format!("Fatal: sub command '{}' not found!", name)).into())
        }
    } else {
        //  If user does not supply a command name
        //  then show the help message
        //  (Need a new mutable clap ref)
        Ok(app::App::new().clap.print_help()?)
    }
}

fn main() {
    match run() {
        Err(e) => {
            eprintln!("{:?}", e);
            process::exit(1);
        }
        Ok(_) => {
            process::exit(0);
        }
    }
}
