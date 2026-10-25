//! `gym web`: the same log, in a browser.
//!
//! The six tabs, the shell and the look are shared code: the same components
//! render on the server for the first paint and hydratate in the browser. The
//! data arrives through server functions (`api`), so the WASM half never holds
//! a database handle.

pub mod api;
pub mod app;
pub mod charts;
pub mod dto;
pub mod pages;
pub mod ui;

#[cfg(feature = "ssr")]
pub mod actions;
#[cfg(feature = "ssr")]
pub mod data;
#[cfg(feature = "ssr")]
pub mod server;
