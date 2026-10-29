use chrono::{DateTime, Local, SecondsFormat, Utc};
use serde::Serialize;
use std::fmt::Display;

use crate::com::*;

pub type Res<T> = Result<T, Err>;

#[derive(Debug)]
pub enum Err {
    Str(String),
}
impl From<String> for Err {
    fn from(v: String) -> Self {
        Err::Str(v)
    }
}
impl From<Err> for String {
    fn from(v: Err) -> String {
        match v {
            Err::Str(s) => s,
        }
    }
}
impl From<InquireError> for Err {
    fn from(v: InquireError) -> Self {
        Self::Str(v.to_string())
    }
}
impl From<MyErr> for Err {
    fn from(v: MyErr) -> Self {
        Self::Str(v.into())
    }
}
impl From<sqlx::Error> for Err {
    fn from(v: sqlx::Error) -> Self {
        Self::Str(v.to_string())
    }
}
impl From<sqlx::migrate::MigrateError> for Err {
    fn from(v: sqlx::migrate::MigrateError) -> Self {
        Self::Str(v.to_string())
    }
}
impl From<std::io::Error> for Err {
    fn from(v: std::io::Error) -> Self {
        Self::Str(v.to_string())
    }
}
impl Display for Err {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Err::Str(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    v: DateTime<Local>,
}
impl Date {
    pub fn as_datetime<'a>(&'a self) -> &'a DateTime<Local> {
        &self.v
    }
}
impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.v.to_rfc3339_opts(SecondsFormat::Secs, false))
    }
}

impl Serialize for Date {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str = &self.to_string();
        serializer.serialize_str(str)
    }
}

pub fn input_date2(prompt: &str) -> Res<Date> {
    let now = Date {
        v: Utc::now().into(),
    }
    .to_string();
    let v: DateTime<Local> = input_date::<Local>(prompt)
        .with_starting_input(&now)
        .prompt()?
        .into();
    Ok(Date { v })
}

pub fn s2t(s: &str) -> Res<DateTime<Local>> {
    Ok(DateTime::<Local>::from(
        DateTime::parse_from_rfc3339(s).map_err(|e| e.to_string())?,
    ))
}
pub fn t2s<T: TimeZone>(t: DateTime<T>) -> String {
    t.to_utc().to_rfc3339()
}

/// The local day containing `now`, as a half-open pair: midnight to midnight.
pub fn day_bounds(now: DateTime<Local>) -> (DateTime<Local>, DateTime<Local>) {
    use chrono::Timelike;
    let o =
        chrono::Duration::seconds((now.hour() * 60 * 60 + now.minute() * 60 + now.second()) as i64);
    let s = now - o;
    (s, s + chrono::Duration::days(1))
}
