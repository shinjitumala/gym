use chrono::{DateTime, Local, SecondsFormat, Utc};
use serde::Serialize;
use std::fmt::Display;

use crate::com::*;

pub type Res<T> = Result<T, Err>;
type DbErr = sqlx::Error;

pub enum Err {
    Str(String),
}
impl From<String> for Err {
    fn from(v: String) -> Self {
        Err::Str(v)
    }
}
impl From<DbErr> for Err {
    fn from(v: DbErr) -> Self {
        Err::Str(v.to_string())
    }
}
impl From<Err> for String {
    fn from(v: Err) -> String {
        use Err::*;
        match v {
            Str(s) => s,
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    v: DateTime<Local>,
}
impl Date {
    pub fn as_timestamp(&self) -> i64 {
        self.v.timestamp()
    }
    pub fn as_datetime<'a>(&'a self) -> &'a DateTime<Local> {
        &self.v
    }
    pub fn from_timestamp(t: i64) -> Self {
        Self {
            v: DateTime::from_timestamp(t, 0).unwrap().into(),
        }
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
    Ok(Date {
        v: input_date(prompt)
            .with_starting_input(
                &Date {
                    v: Utc::now().into(),
                }
                .to_string(),
            )
            .prompt()?
            .into(),
    })
}

pub fn to_one_rep_max(load: f64, rep: f64) -> Res<f64> {
    // https://strengthlevel.com/one-rep-max-calculator
    let rep = rep.round() as i64;
    let f = match rep {
        1 => 100.0_f64,
        2 => 97.0_f64,
        3 => 94.0_f64,
        4 => 92.0_f64,
        5 => 89.0_f64,
        6 => 86.0_f64,
        7 => 83.0_f64,
        8 => 81.0_f64,
        9 => 78.0_f64,
        10 => 75.0_f64,
        11 => 73.0_f64,
        12 => 71.0_f64,
        13 => 70.0_f64,
        14 => 68.0_f64,
        15 => 67.0_f64,
        16 => 65.0_f64,
        17 => 64.0_f64,
        18 => 63.0_f64,
        19 => 61.0_f64,
        20 => 60.0_f64,
        21 => 59.0_f64,
        22 => 58.0_f64,
        23 => 57.0_f64,
        24 => 56.0_f64,
        25 => 55.0_f64,
        26 => 54.0_f64,
        27 => 53.0_f64,
        28 => 52.0_f64,
        29 => 51.0_f64,
        30 => 50.0_f64,
        e => Err(format!("'{e}' is out of bounds."))?,
    };
    let r = load * 100.0_f64 / f;
    Ok((r * 10.0).round() / 10.0)
}
