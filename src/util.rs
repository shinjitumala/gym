use std::{fmt::Display, str::FromStr};

use chrono::{DateTime, Local, SecondsFormat, Utc};
use inquire::{
    validator::{CustomTypeValidator, ErrorMessage},
    CustomType,
};

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

#[derive(Clone)]
pub struct Date {
    v: DateTime<Local>,
}
impl CustomTypeValidator<String> for Date {
    fn validate(
        &self,
        i: &String,
    ) -> Result<inquire::validator::Validation, inquire::CustomUserError> {
        use inquire::validator::Validation::*;
        match DateTime::parse_from_rfc3339(i) {
            Ok(_) => Ok(Valid),
            Err(e) => Ok(Invalid(ErrorMessage::Custom(format!("{e}")))),
        }
    }
}
impl Date {
    pub fn as_timestamp(&self) -> i64 {
        self.v.timestamp()
    }
    pub fn as_datetime<'a>(&'a self) -> &'a DateTime<Local> {
        &self.v
    }
}
impl FromStr for Date {
    type Err = Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            v: DateTime::parse_from_rfc3339(s)
                .map_err(|e| format!("{e}"))?
                .into(),
        })
    }
}
impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.v.to_rfc3339_opts(SecondsFormat::Secs, false))
    }
}

pub fn input_date(prompt: &str) -> Res<Date> {
    Ok(CustomType::<Date>::new(prompt)
        .with_starting_input(
            &Date {
                v: Utc::now().into(),
            }
            .to_string(),
        )
        .prompt()?)
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
    Ok(load * 100.0_f64 / f)
}
