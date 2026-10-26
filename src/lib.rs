//! `gym`: a local-first fitness tracker.
//!
//! The command line lives in `cmd`, the store in `db`, and the web UI in
//! `web`. Only `web`, `stats` and the plain-data modules compile for the WASM
//! half of the web build: `sqlx`, `tokio` and `inquire` are for the host, so a
//! browser bundle never carries a database driver.

pub mod stats;
pub mod train;
pub mod web;

#[cfg(not(target_arch = "wasm32"))]
pub mod cmd;
#[cfg(not(target_arch = "wasm32"))]
pub mod ctx;
#[cfg(not(target_arch = "wasm32"))]
pub mod db;
#[cfg(not(target_arch = "wasm32"))]
pub mod food;
#[cfg(not(target_arch = "wasm32"))]
pub mod import;
#[cfg(not(target_arch = "wasm32"))]
pub mod model;
#[cfg(not(target_arch = "wasm32"))]
pub mod util;

#[cfg(not(target_arch = "wasm32"))]
pub mod com {
    pub use crate::ctx::C;
    pub use crate::db::Db;
    pub use crate::model::*;
    pub use crate::util::*;
    pub use chrono::{DateTime, Local, TimeZone, Utc};
    pub use fpr_cli::*;
    pub use fpr_cli_derives::*;
    pub use inquire::{
        list_option::ListOption,
        validator::{StringValidator, Validation},
        Confirm, CustomType, InquireError, Select, Text,
    };
    pub use itertools::*;
}

/// The browser entry point `cargo leptos` looks for in the WASM half.
///
/// Without it the generated JS has nothing to call and the page never
/// hydrates: the first paint is there, but every form falls back to a full page
/// load and every `<Action>` does nothing.
#[cfg(feature = "hydrate")]
#[leptos::wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount::hydrate_body(crate::web::app::App);
}
