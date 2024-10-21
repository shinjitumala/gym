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

use com::*;
use db::Place;
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
    Ok(())
}

async fn input_place(db: &mut Db) -> Res<Place> {
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
pub struct Main(Add, Prog, Weight, GetWeight);

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
    db.get_prog().await?;
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
    fn run(_c: &C, _a: Self) -> Result<Self::R, String> {
        todo!()
    }
}

fn main2() -> Result<(), String> {
    Ok(Main::run(&C::new()?)?)
}
fn main() -> Result<(), ()> {
    match main2() {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("{}\nAborting.", e);
            Err(())
        }
    }
}
