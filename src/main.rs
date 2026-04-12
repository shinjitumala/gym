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

use std::{net::SocketAddr, process::exit};

use chrono::Utc;
use com::*;
use serde::Serialize;

fn input_place(db: &Db) -> Res<(usize, db::Place)> {
    let places = db.places();
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
pub struct Main(Weight, Web, Sync, New, Food, Test);

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
    addr: String,
}
impl Run<C> for Web {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(web(c, a)?)
    }
}

#[derive(Args)]
#[args(desc = "Test website.")]
pub struct Test {
    #[arg(desc = "Socket address.", s = ("0.0.0.0:8080"))]
    addr: String,
}
impl Run<C> for Test {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(test(c, a)?)
    }
}
#[tokio::main]
async fn test(c: &C, a: Test) -> Res<()> {
    use web_api::*;

    let r = env!("CARGO_MANIFEST_DIR");

    let x = dir(format!("{r}/s/"))
        .or(path("mgs").and(with_db(c.clone())).and_then(mgs))
        .or(path("map").and(with_db(c.clone())).and_then(map))
        .or(path("sets").and(with_db(c.clone())).and_then(hsets))
        .or(path("prog").and(with_db(c.clone())).and_then(hprog))
        .or(path("weight").and(with_db(c.clone())).and_then(hweight))
        .or(path("food").and(with_db(c.clone())).and_then(hfood));

    let x = get().and(x.or(file(format!("{r}/s/index.html"))));

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

mod web_api {
    use super::*;
    use std::{collections::BTreeMap, convert::Infallible};
    pub use warp::{
        any,
        filters::fs::{dir, file},
        get, path,
        path::end,
        reply::{html, json, with_header},
        serve, Filter, Reply,
    };

    #[derive(Serialize)]
    struct DataProg {
        date: Vec<Date>,
        max: Vec<f64>,
        desc: Vec<String>,
    }
    impl DataProg {
        pub fn new() -> Self {
            Self {
                date: Vec::new(),
                max: Vec::new(),
                desc: Vec::new(),
            }
        }
    }
    async fn prog(c: &C) -> Res<BTreeMap<String, DataProg>> {
        let mut db = c.db()?;
        let r = db.get_prog().await?;
        let mut m = BTreeMap::new();
        for (k, v) in r {
            let e = match m.get_mut(&k) {
                Some(e) => e,
                None => {
                    m.insert(k.to_owned(), DataProg::new());
                    m.get_mut(&k).unwrap()
                }
            };
            for v in v {
                e.date.push(v.date);
                e.max.push(v.max);
                e.desc.push(format!("{} x {}\n{}", v.load, v.rep, v.desc));
            }
        }
        Ok(m)
    }

    #[derive(Serialize)]
    struct DataSets {
        date: Vec<Date>,
        place: Vec<String>,
        count: Vec<f64>,
        desc: Vec<f64>,
    }
    impl DataSets {
        fn new() -> Self {
            Self {
                date: Vec::new(),
                place: Vec::new(),
                count: Vec::new(),
                desc: Vec::new(),
            }
        }
    }

    type Sets = BTreeMap<String, BTreeMap<String, DataSets>>;
    async fn sets(c: &C) -> Res<Sets> {
        let mut db = c.db()?;
        let b = todo!(); //db.sets().await?;
                         // let mut m = BTreeMap::new();
                         // for b in b {
                         //     let k1 = b.mg;
                         //     let v1 = match m.get_mut(&k1) {
                         //         Some(e) => e,
                         //         None => {
                         //             m.insert(k1.to_owned(), BTreeMap::new());
                         //             m.get_mut(&k1).unwrap()
                         //         }
                         //     };
                         //
                         //     let k2 = b.exercise;
                         //     let v2 = match v1.get_mut(&k2) {
                         //         Some(e) => e,
                         //         None => {
                         //             v1.insert(k2.to_owned(), DataSets::new());
                         //             v1.get_mut(&k2).unwrap()
                         //         }
                         //     };
                         //
                         //     v2.date.push(b.date);
                         //     v2.place.push(b.place);
                         //     v2.count.push(b.count);
                         //     v2.desc.push(b.desc);
                         // }
                         // Ok(m)
    }

    #[derive(Serialize)]
    struct DataWeight {
        date: Vec<Date>,
        kg: Vec<f64>,
        bodyfat: Vec<f64>,
        desc: Vec<String>,
    }
    async fn get_weight(c: &C) -> Res<DataWeight> {
        let mut db = c.db()?;
        let d = db.get_weight().await?;

        let mut r = DataWeight {
            date: Vec::new(),
            kg: Vec::new(),
            bodyfat: Vec::new(),
            desc: Vec::new(),
        };
        for a in d {
            r.date.push(Date::from_timestamp(s2t(&a.date)?.timestamp()));
            r.kg.push(a.kg);
            r.bodyfat.push(a.bodyfat);
            r.desc.push(a.desc);
        }

        Ok(r)
    }

    #[derive(Serialize)]
    pub struct DataFood {
        date: Vec<String>,
        calories: Vec<f64>,
        protein: Vec<f64>,
        desc: Vec<String>,
    }
    impl DataFood {
        fn new() -> Self {
            Self {
                date: Vec::new(),
                calories: Vec::new(),
                protein: Vec::new(),
                desc: Vec::new(),
            }
        }
    }

    pub async fn food(c: &C) -> Res<BTreeMap<String, DataFood>> {
        let mut db = c.db()?;
        // let mut r = BTreeMap::new();
        todo!()
        // let m = db.get_meals().await?;
        // for m in m.breakdown {
        //     let v = match r.get_mut(&m.name) {
        //         Some(e) => e,
        //         None => {
        //             r.insert(m.name.to_owned(), DataFood::new());
        //             r.get_mut(&m.name).unwrap()
        //         }
        //     };
        //
        //     v.date.push(m.date.unwrap_or(String::new()));
        //     v.calories.push(m.calories);
        //     v.protein.push(m.protein.unwrap_or(0f64));
        //     v.desc.push(format!(
        //         "{} x {}\n{}",
        //         m.name,
        //         m.amount.unwrap_or(1.),
        //         m.desc
        //     ))
        // }
        //
        // Ok(r)
    }

    #[derive(Serialize)]
    pub struct JsonErr {
        message: String,
    }
    impl JsonErr {
        fn new(e: Err) -> Self {
            Self {
                message: String::from(e),
            }
        }
    }

    pub fn with_db(c: C) -> impl Filter<Extract = (C,), Error = Infallible> + Clone {
        any().map(move || c.clone())
    }
    pub async fn hprog(c: C) -> Result<impl Reply, Infallible> {
        let r = prog(&c).await;
        match r {
            Err(e) => Ok(json(&JsonErr::new(e))),
            Ok(e) => Ok(json(&e)),
        }
    }
    pub async fn hweight(c: C) -> Result<impl Reply, Infallible> {
        let r = get_weight(&c).await;
        match r {
            Err(e) => Ok(json(&JsonErr::new(e))),
            Ok(e) => Ok(json(&e)),
        }
    }
    pub async fn hfood(c: C) -> Result<impl Reply, Infallible> {
        let r = food(&c).await;
        match r {
            Err(e) => Ok(json(&JsonErr::new(e))),
            Ok(e) => Ok(json(&e)),
        }
    }
    pub async fn mgs(c: C) -> Result<impl Reply, Infallible> {
        async fn a(c: C) -> Res<Vec<MuscleGroup>> {
            todo!()
            // let mut db = c.db()?;
            // Ok(db.muscle_groups().await?)
        }
        match a(c).await {
            Err(e) => Ok(json(&JsonErr::new(e))),
            Ok(e) => Ok(json(&e)),
        }
    }
    pub async fn hsets(c: C) -> Result<impl Reply, Infallible> {
        async fn a(c: C) -> Res<Sets> {
            Ok(sets(&c).await?)
        }
        match a(c).await {
            Err(e) => Ok(json(&JsonErr::new(e))),
            Ok(e) => Ok(json(&e)),
        }
    }
    pub async fn map(c: C) -> Result<impl Reply, Infallible> {
        async fn a(c: C) -> Res<db::MajorExerciseMaps> {
            let mut db = c.db()?;
            Ok(db.major_exercise_maps().await?)
        }
        match a(c).await {
            Err(e) => Ok(json(&JsonErr::new(e))),
            Ok(e) => Ok(json(&e)),
        }
    }

    pub const INDEX: &str = include_str!("../s/index.html");
    pub const CSS: &str = include_str!("../s/main.css");
    pub const JS: &str = include_str!("../s/main.js");
}

#[tokio::main]
async fn web(c: &C, a: Web) -> Res<()> {
    use web_api::*;

    let index = end().map(|| html(INDEX));

    let x = path("index.html")
        .map(|| html(INDEX))
        .or(path("main.css").map(|| with_header(CSS, "content-type", "text/css")))
        .or(path("main.js").map(|| with_header(JS, "content-type", "text/javascript")))
        .or(path("mgs").and(with_db(c.clone())).and_then(mgs))
        .or(path("map").and(with_db(c.clone())).and_then(map))
        .or(path("sets").and(with_db(c.clone())).and_then(hsets))
        .or(path("prog").and(with_db(c.clone())).and_then(hprog))
        .or(path("weight").and(with_db(c.clone())).and_then(hweight))
        .or(path("food").and(with_db(c.clone())).and_then(hfood));

    let x = get().and(x.or(index));
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
                                to_one_rep_max(load, rep)?,
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
    db.load_full()?;
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
