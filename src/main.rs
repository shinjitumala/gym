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

use act::list_new_files;
use com::*;
use db::ExerciseHistoryItem;
use inquire::{list_option::ListOption, Confirm, CustomType, Select, Text};
use itertools::Itertools;

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
    let date = input_date2("Date")?;

    let mut t = db.start().await?;

    let session = t.new_session(place.id, date).await?;

    for r in csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(a.data.p)
        .map_err(|e| format!("Failed to read file '{}' because '{e}'", a.data.s))?
        .records()
    {
        let sets: Vec<_> = r.unwrap().iter().map(|e| e.to_owned()).collect();
        let e = t.get_exercise(&sets[0]).await?;
        for s in sets[1..].iter() {
            let m: Vec<_> = s.split("x").collect();
            let w: f64 = m[0].trim().parse().map_err(|e| {
                format!(
                    "Failed to parse as f64 '{}' because '{e}' at '{sets:?}'",
                    m[0]
                )
            })?;
            let reps = &m[1..];
            for r in reps {
                let r: f64 = r.trim().parse().map_err(|e| {
                    format!("Failed to parse as f64 '{}' because '{e}' at '{sets:?}'", r)
                })?;
                t.new_set(session, e.id, w, r, to_one_rep_max(w, r)?, format!(""))
                    .await?;
            }
        }
    }

    t.commit().await?;

    Ok(())
}

async fn input_place(db: &mut Db) -> Res<db::Place> {
    let places = db.places().await?;
    let lines = to_lines(&places.iter().map(|e| e.to_line()).collect_vec())
        .into_iter()
        .enumerate()
        .map(|(i, e)| ListOption::new(i, e))
        .collect();
    Ok(places[Select::new("Place", lines).prompt()?.index].clone())
}

#[derive(Acts)]
#[acts(desc = "")]
#[allow(dead_code)]
pub struct Main(
    Add,
    Prog,
    Weight,
    GetWeight,
    Place,
    Upd,
    Web,
    Sync,
    AdHoc,
    New,
);

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
    let date = input_date2("When did you measure?")?;
    let weight = CustomType::<f64>::new("Weight (kg)").prompt()?;
    let bodyfat = CustomType::<f64>::new("Bodyfat (%)").prompt()?;
    let desc = Text::new("Note").prompt()?;
    let mut db = c.db().await?;
    db.add_weight(date, weight, bodyfat, desc).await?;
    act::upd()?;
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

#[derive(Args)]
#[args(desc = "Automated update.")]
pub struct Sync {}
impl Run<C> for Sync {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(sync(c, a)?)
    }
}
fn sync(c: &C, _a: Sync) -> Res<()> {
    let r = list_new_files(&c.cfg.repo)?;
    let r: Vec<_> = r.trim().split("\n").collect();
    for r in r.iter() {
        Add::run(
            c,
            Add {
                data: FileExist::parse(&format!("{}/{}", &c.cfg.repo, r))
                    .map_err(|e| format!("{e}"))?,
            },
        )?;
    }
    act::upd()?;
    act::commit(&c.cfg.repo)?;
    Ok(())
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

#[derive(Args)]
#[args(desc = "Execute scripts as needed for altering table, etc.")]
pub struct AdHoc {}
impl Run<C> for AdHoc {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(adhoc(c, a)?)
    }
}
#[tokio::main]
async fn adhoc(c: &C, _a: AdHoc) -> Res<()> {
    let mut db = c.db().await?;
    db.adhoc().await?;
    Ok(())
}

#[derive(Args)]
#[args(desc = "New session at the gym to the buffer.")]
pub struct New {}
impl Run<C> for New {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(new_session(c, a)?)
    }
}
#[tokio::main]
async fn new_session(c: &C, _a: New) -> Res<()> {
    let mut db = c.db().await?;
    let p = input_place(&mut db).await?;
    let d = input_date2("Training time")?;
    let ecomp = TextWithAutocomplete::new(db.exercises(p.id).await?, |e| {
        [e.name.to_owned(), e.desc.to_owned()]
    });

    let mut t = db.start().await?;
    let s = t.new_session(p.id, d).await?;
    t.commit().await?;

    loop {
        let mut t = db.start().await?;
        let e = Text::new("Exercise")
            .with_autocomplete(ecomp.clone())
            .prompt()?
            .trim()
            .to_owned();
        let e = t.get_exercise(&e).await?;
        t.commit().await?;

        let h = db.get_exercise_history(p.id, e.id).await?;
        let mut l = 0i64;
        let mut b = Vec::<ExerciseHistoryItem>::new();
        for h in h {
            if h.date != l {
                if l != 0 {
                    let d = Date::from_timestamp(l);
                    println!("{d}:");
                    println!(
                        "{}",
                        to_table(
                            &b.iter()
                                .map(|e| {
                                    [
                                        format!("{}", e.load),
                                        format!("x"),
                                        format!("{}", e.rep),
                                        e.desc.to_owned(),
                                    ]
                                })
                                .collect_vec(),
                        )
                    );
                }
                l = h.date;
                b.clear();
            }
            b.push(h.to_owned());
        }
        if l != 0 {
            let d = Date::from_timestamp(l);
            println!("{d}:");
            println!(
                "{}",
                to_table(
                    &b.iter()
                        .map(|e| {
                            [
                                format!("{}", e.load),
                                format!("x"),
                                format!("{}", e.rep),
                                e.desc.to_owned(),
                            ]
                        })
                        .collect_vec(),
                )
            );
        }

        loop {
            let load = CustomType::<f64>::new("load").prompt()?;
            loop {
                let mut t = db.start().await?;
                let rep = CustomType::<f64>::new("rep").prompt()?;
                let desc = Text::new("Notes").prompt()?;
                t.new_set(s, e.id, load, rep, to_one_rep_max(load, rep)?, desc)
                    .await?;
                t.commit().await?;

                if Confirm::new("Done with current load?")
                    .with_default(false)
                    .prompt()?
                {
                    break;
                }
            }

            if Confirm::new("Done with exercise?")
                .with_default(false)
                .prompt()?
            {
                break;
            }
        }

        if Confirm::new("Done with session?")
            .with_default(false)
            .prompt()?
        {
            break;
        }
    }
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
