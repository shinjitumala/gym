mod ctx;
mod db;
mod food;
mod util;

pub mod com {
    pub use crate::ctx::C;
    pub use crate::db::Db;
    pub use crate::db::*;
    pub use crate::util::*;
    pub use fpr_cli::*;
    pub use fpr_cli_derives::*;
    pub use inquire::{
        list_option::ListOption,
        validator::{StringValidator, Validation},
        Confirm, CustomType, InquireError, Select, Text,
    };
    pub use itertools::*;
}
use chrono::Utc;
use com::*;
use std::process::exit;

fn input_place(db: &Db) -> Res<(usize, db::Place)> {
    let places = db.places()?;
    let lines = to_lines(&places.iter().map(|(_, e)| e.to_line()).collect_vec())
        .into_iter()
        .enumerate()
        .map(|(i, e)| ListOption::new(i, e))
        .collect();
    let (id, place) = places[Select::new("Place", lines).prompt()?.index];
    Ok((*id, place.to_owned()))
}

#[derive(Acts)]
#[acts(desc = "")]
#[allow(dead_code)]
pub struct Main(Weight, Web, Sync, New, Food);

#[derive(Args)]
#[args(desc = "Add weight data.")]
pub struct Weight {}
impl Run<C> for Weight {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(weight(c, a)?)
    }
}
fn weight(c: &C, _a: Weight) -> Res<()> {
    let date = input_date2("When did you measure?")?;
    let weight = CustomType::<f64>::new("Weight (kg)").prompt()?;
    let bodyfat = CustomType::<f64>::new("Bodyfat (%)").prompt()?;
    let desc = Text::new("Note").prompt()?;
    let mut db = c.db()?;
    db.add_weight(date, weight, bodyfat, desc)?;
    Ok(())
}

#[derive(Args)]
#[args(desc = "Runs a local web server.")]
pub struct Web {
    #[arg(desc = "Socket address.", s = ("0.0.0.0:8080"))]
    _addr: String,
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
async fn web(_c: &C, _a: Web) -> Res<()> {
    todo!()
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
fn new_session(c: &C, _a: New) -> Res<()> {
    let mut db = c.db()?;
    let (pid, _) = input_place(&db)?;
    let d = input_date2("Training time")?;
    let desc = Text::new("Description")
        .prompt_skippable()?
        .unwrap_or_default();
    let s = db.new_session(pid, d, desc)?;
    db.save()?;

    loop {
        let ecomp = TextWithAutocomplete::new(
            db.exercises(pid)?
                .into_iter()
                .map(|(id, e)| (*id, e.to_owned()))
                .collect(),
            |(_, e)| [e.name.to_owned(), e.desc.to_owned()],
        );

        let e = Text::new("Exercise")
            .with_autocomplete(ecomp.clone())
            .with_help_message("Press ESC if done")
            .prompt_skippable()?
            .map(|e| e.trim().to_owned());

        if let Some(e) = e {
            let (eid, _) = db.get_exercise(&e)?;

            let h = db.get_exercise_history(pid, eid, 3)?;
            let mut l = String::new();
            let mut b = Vec::<ExerciseHistoryItem>::new();
            for h in h {
                if h.date != l {
                    if !l.is_empty() {
                        let d = s2t(&l)?.to_rfc3339();
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
                    l = h.date.to_owned();
                    b.clear();
                }
                b.push(h.to_owned());
            }
            if !l.is_empty() {
                let d = s2t(&l)?.to_rfc3339();
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
                let load = CustomType::<f64>::new("load")
                    .with_help_message("Press ESC if done with exercise")
                    .prompt_skippable()?;
                if let Some(load) = load {
                    loop {
                        let rep = CustomType::<f64>::new("rep")
                            .with_help_message("Press ESC if done with load")
                            .prompt_skippable()?;
                        if let Some(rep) = rep {
                            let desc = Text::new("Notes").prompt()?;
                            db.new_set(
                                Utc::now(),
                                s,
                                eid,
                                load,
                                rep,
                                desc,
                            )?;
                            db.save()?;
                        } else {
                            break;
                        }
                    }
                } else {
                    break;
                }
            }
        } else {
            break;
        }
    }
    db.save()?;
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
fn food(c: &C, _a: Food) -> Res<()> {
    let mut db = c.db()?;

    let t = db.calories_today()?;
    println!("Today's total:");
    println!("{}", to_table(&t.to_lines_today()));

    let date = input_date2("When did you eat?")?;
    let foods = db
        .foods()?
        .into_iter()
        .map(|(k, v)| (*k, v.to_owned()))
        .collect_vec();
    loop {
        let f = match select_line(
            "What did you eat? (calories kcal, protein g, fat g, carbohydrates g)",
            &foods,
            |(_, f)| f.to_line(),
        )
        .with_help_message("Press ESC to register unknown food")
        .prompt_skippable()?
        {
            Some(f) => foods[f.index].0,
            None => {
                println!("Registering new food...");
                food::reg(&mut db)?
            }
        };

        let amount = CustomType::<f64>::new("Amount")
            .with_help_message("Multiplier")
            .with_default(1.0f64)
            .prompt()?;
        let desc = Text::new("desc").prompt()?;

        db.new_meal(date.as_datetime().to_owned(), f, amount, &desc)?;
        db.save()?;

        if !Confirm::new("Add more food?")
            .with_default(false)
            .prompt()?
        {
            break;
        }
    }

    let t = db.calories_today()?;
    println!("Today's total:");
    println!("{}", to_table(&t.to_lines_today()));

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
