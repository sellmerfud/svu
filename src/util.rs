
use thiserror::Error;
use crate::svn::{LogPath, FromPath};
use colored::*;
use chrono::{DateTime, Local, NaiveDateTime};
use std::sync::OnceLock;

#[derive(Error, Debug)]
pub enum SvError {
    #[error("{0}")]
    General(String),
    #[error("{}", String::from_utf8_lossy(&.0.stderr))]
    SvnError(std::process::Output),
}

pub trait StringWrapper {
    fn chomp<'a>(&'a self, ch: char) -> &'a str;
}

impl StringWrapper for String {
    fn chomp<'a>(&'a self, ch: char) -> &'a str {
        self.trim_end_matches(ch)
    }
}

pub fn join_paths<S, T>(base: S, leaf: T) -> String
    where S: AsRef<str>, T: AsRef<str> {
    let mut path = String::new();

    path += base.as_ref().trim_end_matches('/');
    path += "/";
    path += leaf.as_ref().trim_matches('/');
    path
}


pub fn formatted_log_path(log_path: &LogPath) -> String {
    let color = match log_path.action.as_str() {
        "D"  => "red",
        "A"  => "green",
        "M"  => "blue",
        _    => "white"
    };

    let base = format!("  {} {}", log_path.action.color(color), log_path.path.color(color));

    match &log_path.from_path {
        Some(FromPath { path, revision }) => format!("{} (from {} {})", base, path.magenta(), revision.yellow()),
        None                              => base
    }
}

//  Create a `null` date value to use when an
//  entry has no date.
pub fn null_date() -> &'static DateTime<Local> {
    static NULL_DATE: OnceLock<DateTime<Local>> = OnceLock::new();
    NULL_DATE.get_or_init(|| {
        let timestamp_millis: i64 = -2208936075000; //Mon Jan 01 1900 14:38:45 GMT+0000
        let naive_datetime = NaiveDateTime::from_timestamp_millis(timestamp_millis).unwrap();
        let offset = Local::now().offset().clone();
        DateTime::<Local>::from_naive_utc_and_offset(naive_datetime, offset)
    })
}

pub fn parse_svn_date(date_str: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(date_str)
    .unwrap()  // We assume all svn dates are well formed!
    .with_timezone(&Local)
}

pub fn parse_svn_date_opt(opt_date_str: Option<String>) -> DateTime<Local> {
    if let Some(date_str) = opt_date_str {
        parse_svn_date(date_str.as_str())
    } else {
        *null_date()
    }
}

pub fn display_svn_date(date: &DateTime<Local>) -> String {
    date.format("%Y-%m-%d").to_string()
}

pub fn display_svn_time(date: &DateTime<Local>) -> String {
    date.format("%H:%M:%S").to_string()
}

pub fn display_svn_datetime(date: &DateTime<Local>) -> String {
    if date == null_date() {
        "n/a".to_owned()
    } else {
        format!("{} {}", display_svn_date(date), display_svn_time(date))
    }
}
