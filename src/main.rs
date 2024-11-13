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
use db::ExerciseHistoryItem;
use inquire::{list_option::ListOption, Confirm, CustomType, Select, Text};
use itertools::Itertools;

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
pub struct Main(Weight, Place, Web, Sync, New, Food, RegFood);

async fn prog(c: &C) -> Res<db::Prog> {
    let mut db = c.db().await?;
    let r = db.get_prog().await?;
    Ok(r)
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
    Ok(())
}

async fn get_weight(c: &C) -> Res<Vec<db::Weight>> {
    let mut db = c.db().await?;
    let d = db.get_weight().await?;
    Ok(d)
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
#[args(desc = "Sync with remote.")]
pub struct Sync {}
impl Run<C> for Sync {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(sync(c, a)?)
    }
}
fn sync(c: &C, _a: Sync) -> Res<()> {
    let repo = &c.cfg.repo;
    act::pull(&repo)?;
    act::commit(&repo)?;
    Ok(())
}

#[tokio::main]
async fn web(c: &C, a: Web) -> Res<()> {
    use warp::{
        fs::{dir, file},
        path,
        reply::json,
        serve, Filter,
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("s");

    let rprog = prog(c).await?;
    let rweight = get_weight(c).await?;

    let mut db = c.db().await?;
    let rfood = db.get_meals().await?;

    let x = path!()
        .and(file(root.join("index.html")))
        .or(dir(root))
        .or(path("prog").map(move || json(&rprog)))
        .or(path("weight").map(move || json(&rweight)))
        .or(path("food").map(move || json(&rfood)));
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

#[derive(Args)]
#[args(desc = "Add food data.")]
pub struct Food {}
impl Run<C> for Food {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(food(c, a)?)
    }
}
#[tokio::main]
async fn food(c: &C, _a: Food) -> Res<()> {
    let mut db = c.db().await?;

    let foods = db.foods().await?;
    let fcmp = TextWithAutocomplete::new(foods.clone(), |f| [f.name.to_owned()]);
    let f = loop {
        let f = Text::new("food")
            .with_autocomplete(fcmp.clone())
            .prompt()?
            .trim()
            .to_owned();
        if let Some(e) = foods.iter().find(|e| e.name == f) {
            println!("{}", e.print());
            break e.id;
        }

        println!("Registering new food...");
        let f = reg_food(&mut db, &f).await?;
        break f;
    };
    let date = input_date2("When did you eat?")?;
    let desc = Text::new("desc").prompt()?;

    db.new_meal(date.as_timestamp(), f, &desc).await?;
    Ok(())
}

#[derive(Args)]
#[args(desc = "Register new food.")]
pub struct RegFood {}
impl Run<C> for RegFood {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(reg_food_main(c, a)?)
    }
}
#[tokio::main]
async fn reg_food_main(c: &C, _a: RegFood) -> Res<()> {
    let mut db = c.db().await?;
    let f = Text::new("Name").prompt()?;
    reg_food(&mut db, &f).await?;
    Ok(())
}
async fn reg_food(db: &mut Db, name: &str) -> Res<i64> {
    let calories = CustomType::<f64>::new("calories").prompt()?;
    let protein = CustomType::<f64>::new("protein")
        .with_help_message("You can press ESC if unknown")
        .prompt_skippable()?;
    let fat = CustomType::<f64>::new("fat")
        .with_help_message("You can press ESC if unknown")
        .prompt_skippable()?;
    let carbohydrate = CustomType::<f64>::new("carbohydrate")
        .with_help_message("You can press ESC if unknown")
        .prompt_skippable()?;
    let desc = Text::new("desc").prompt()?;
    Ok(db
        .new_food(&name, calories, protein, fat, carbohydrate, &desc)
        .await?)
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
