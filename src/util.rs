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
