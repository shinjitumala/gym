use gym::cmd::{Food, Import, Init, New, Sync, Web, Weight};
use gym::com::*;
use gym::ctx;
use std::process::exit;

#[derive(Acts)]
#[acts(desc = "")]
#[allow(dead_code)]
pub struct Main(Weight, Web, Sync, New, Food, Db);

#[derive(Acts)]
#[acts(desc = "The database.")]
#[allow(dead_code)]
pub struct Db(Init, Import);

fn main2() -> Result<(), String> {
    Ok(Main::run(&ctx::C::new()?)?)
}
fn main() -> Result<(), ()> {
    match main2() {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("{}\nAborting.", e);
            exit(1)
        }
    }
}
