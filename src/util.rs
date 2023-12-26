
use thiserror::Error;
use crate::svn::{LogPath, FromPath};
use colored::*;
use chrono::{DateTime, Local};

#[derive(Error, Debug)]
pub enum SvError {
    #[error("{0}")]
    General(String),
    #[error("{}", String::from_utf8_lossy(&.0.stderr))]
    SvnError(std::process::Output),
}

pub fn formatted_log_path(log_path: &LogPath) -> String {
    let color = match log_path.action.as_str() {
        "D"  => "red",
        "A"  => "green",
        "M"  => "blue",
        rest => "white"
    };

    let base = format!("  {} {}", log_path.action.color(color), log_path.path.color(color));

    match &log_path.from_path {
        Some(FromPath { path, revision }) => format!("{} (from {} {})", base, path.magenta(), revision.yellow()),
        None                              => base
    }
}

pub fn parse_svn_date(date_str: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(date_str)
    .unwrap()  // We assume all svn dates are well formed!
    .with_timezone(&Local)
}

pub fn display_svn_date(date: &DateTime<Local>) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn display_svn_time(date: &DateTime<Local>) -> String {
    date.format("%H:%M:%S").to_string()
}

pub fn display_svn_datetime(date: &DateTime<Local>) -> String {
    format!("{} {}", display_svn_date(date), display_svn_time(date))
}
