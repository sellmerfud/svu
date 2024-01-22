
use clap::Parser;
use std::process;
use crate::app::{Commands, Run};

mod app;
mod util;
mod commands;
mod svn;
mod auth;


fn main() {
    match Commands::parse().run() {
        Ok(_) => {
            process::exit(0);
        }
        Err(e) => {
            eprintln!("{:?}", e);
            process::exit(1);
        }
    }
}
