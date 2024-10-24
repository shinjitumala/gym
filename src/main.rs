mod ctx;
mod db;
mod util;

pub mod com {
    pub use crate::ctx::C;
    pub use crate::db::Db;
    pub use crate::util::*;
    pub use fpr_cli::*;
    pub use fpr_cli_derives::*;
    pub use inquire::InquireError;
}

use std::{net::SocketAddr, path::PathBuf, process::exit};

use com::*;
use inquire::{list_option::ListOption, CustomType, Select, Text};

#[derive(Args)]
#[args(desc = "Add JSON data.")]
pub struct Add {
    #[arg(desc = "Path to the CSV data.")]
    data: FileExist,
}
impl Run<C> for Add {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(add(c, a)?)
    }
}
#[tokio::main]
async fn add(c: &C, a: Add) -> Res<()> {
    let mut db = c.db().await?;

    let place = input_place(&mut db).await?;
    let date = input_date("Date")?;

    let session = db.new_session(place.id, date).await?;

    for r in csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(a.data.p)
        .map_err(|e| format!("Failed to read file '{}' because '{e}'", a.data.s))?
        .records()
    {
        let sets: Vec<_> = r.unwrap().iter().map(|e| e.to_owned()).collect();
        let e = db.get_exercise(&sets[0]).await?;
        for s in sets[1..].iter() {
            let m: Vec<_> = s.split("x").collect();
            let w: f64 = m[0]
                .trim()
                .parse()
                .map_err(|e| format!("Failed to parse as f64 '{}' because '{e}'", m[0]))?;
            let reps = &m[1..];
            for r in reps {
                let r: f64 = r
                    .trim()
                    .parse()
                    .map_err(|e| format!("Failed to parse as f64 '{}' because '{e}'", r))?;
                db.new_set(session, e.id, w, r, format!("")).await?;
            }
        }
    }

    act::upd()?;

    Ok(())
}

async fn input_place(db: &mut Db) -> Res<db::Place> {
    let places = db.places().await?;
    let lines = to_lines(&places.iter().map(|e| e.to_line()).collect())
        .into_iter()
        .enumerate()
        .map(|(i, e)| ListOption::new(i, e))
        .collect();
    Ok(places[Select::new("Place", lines).prompt()?.index].clone())
}

#[derive(Acts)]
#[acts(desc = "")]
#[allow(dead_code)]
pub struct Main(Add, Prog, Weight, GetWeight, Place, Upd, Web);

#[derive(Args)]
#[args(desc = "Get the progress data.")]
pub struct Prog {}
impl Run<C> for Prog {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(prog(c, a)?)
    }
}
#[tokio::main]
async fn prog(c: &C, _a: Prog) -> Res<()> {
    let mut db = c.db().await?;
    let r = db.get_prog().await?;
    println!("{r}");
    Ok(())
}

#[derive(Args)]
#[args(desc = "Add weight data.")]
pub struct Weight {}
impl Run<C> for Weight {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(weight(c, a)?)
    }
}
#[tokio::main]
async fn weight(c: &C, _a: Weight) -> Res<()> {
    let date = input_date("When did you measure?")?;
    let weight = CustomType::<f64>::new("Weight (kg)").prompt()?;
    let bodyfat = CustomType::<f64>::new("Bodyfat (%)").prompt()?;
    let desc = Text::new("Note").prompt()?;
    let mut db = c.db().await?;
    db.add_weight(date, weight, bodyfat, desc).await?;
    Ok(())
}

#[derive(Args)]
#[args(desc = "Get weight data.")]
pub struct GetWeight {}
impl Run<C> for GetWeight {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(get_weight(c, a)?)
    }
}
#[tokio::main]
async fn get_weight(c: &C, _a: GetWeight) -> Res<()> {
    let mut db = c.db().await?;
    let d = db.get_weight().await?;
    println!("{d}");
    Ok(())
}

#[derive(Args)]
#[args(desc = "Add a place.")]
pub struct Place {
    #[arg(desc = "Name of a place you train.")]
    name: String,
    #[arg(desc = "Description of the place.")]
    desc: String,
}
impl Run<C> for Place {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(place(c, a)?)
    }
}
#[tokio::main]
async fn place(c: &C, a: Place) -> Res<()> {
    let mut db = c.db().await?;
    db.new_place(&a.name, &a.desc).await?;
    Ok(())
}

#[derive(Args)]
#[args(desc = "Updates the server data to the latest.")]
pub struct Upd {}
impl Run<C> for Upd {
    type R = ();
    fn run(_c: &C, _a: Self) -> Result<Self::R, String> {
        act::upd()?;
        Ok(())
    }
}

#[derive(Args)]
#[args(desc = "Runs a local web server.")]
pub struct Web {
    #[arg(desc = "Socket address.", s = ("0.0.0.0:8080"))]
    addr: String,
}
impl Run<C> for Web {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(web(c, a)?)
    }
}
#[tokio::main]
async fn web(_c: &C, a: Web) -> Res<()> {
    use warp::{
        fs::{dir, file},
        path, serve, Filter,
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("s");
    let x = path!().and(file(root.join("index.html"))).or(dir(root));
    println!("Starting web server at '{}'...", a.addr);
    serve(x)
        .run(
            a.addr
                .parse::<SocketAddr>()
                .map_err(|e| format!("Failed to parse addr '{}' because '{e}'", a.addr))?,
        )
        .await;
    Ok(())
}

fn main2() -> Result<(), String> {
    Ok(Main::run(&C::new()?)?)
}
fn main() -> Result<(), ()> {
    match main2() {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("{}\nAborting.", e);
            exit(1);
        }
    }
}
