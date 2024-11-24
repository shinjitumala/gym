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

use std::{net::SocketAddr, process::exit};

use com::*;
use db::{ExerciseHistoryItem, MuscleGroup};
use inquire::{list_option::ListOption, Confirm, CustomType, Select, Text};
use itertools::Itertools;
use serde::Serialize;

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
    Weight,
    Place,
    Web,
    Sync,
    New,
    Food,
    RegFood,
    MapExercise,
    Test,
);

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
        let mut db = c.db().await?;
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
    struct DataWeight {
        date: Vec<Date>,
        kg: Vec<f64>,
        bodyfat: Vec<f64>,
        desc: Vec<String>,
    }
    async fn get_weight(c: &C) -> Res<DataWeight> {
        let mut db = c.db().await?;
        let d = db.get_weight().await?;

        let mut r = DataWeight {
            date: Vec::new(),
            kg: Vec::new(),
            bodyfat: Vec::new(),
            desc: Vec::new(),
        };
        for a in d {
            r.date.push(a.date);
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
        let mut db = c.db().await?;
        let mut r = BTreeMap::new();
        let m = db.get_meals().await?;
        for m in m.breakdown {
            let v = match r.get_mut(&m.name) {
                Some(e) => e,
                None => {
                    r.insert(m.name.to_owned(), DataFood::new());
                    r.get_mut(&m.name).unwrap()
                }
            };

            v.date.push(m.date.unwrap_or(String::new()));
            v.calories.push(m.calories);
            v.protein.push(m.protein.unwrap_or(0f64));
            v.desc.push(format!(
                "{} x {}\n{}",
                m.name,
                m.amount.unwrap_or(1.),
                m.desc
            ))
        }

        Ok(r)
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
            let mut db = c.db().await?;
            Ok(db.muscle_groups().await?)
        }
        match a(c).await {
            Err(e) => Ok(json(&JsonErr::new(e))),
            Ok(e) => Ok(json(&e)),
        }
    }
    pub async fn map(c: C) -> Result<impl Reply, Infallible> {
        async fn a(c: C) -> Res<db::MajorExerciseMaps> {
            let mut db = c.db().await?;
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
    let amount = CustomType::<f64>::new("Amount")
        .with_help_message("Multiplier")
        .with_default(1.0f64)
        .prompt()?;
    let desc = Text::new("desc").prompt()?;

    db.new_meal(date.as_timestamp(), f, amount, &desc).await?;
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

#[derive(Args)]
#[args(desc = "Map exercise to muscle group.")]
pub struct MapExercise {}
impl Run<C> for MapExercise {
    type R = ();
    fn run(c: &C, a: Self) -> Result<Self::R, String> {
        Ok(map_exercise(c, a)?)
    }
}
#[tokio::main]
async fn map_exercise(c: &C, _a: MapExercise) -> Res<()> {
    let mut db = c.db().await?;
    let places = db.places().await?;
    let place = &places[select_line("Filter exercise by place", &places, |e| {
        e.to_line().map(|e| e.to_owned())
    })
    .prompt()?
    .index];

    let exercises = db.exercises(place.id).await?;
    let mgs = db.muscle_groups().await?;

    loop {
        let exercise = &exercises[select_line("Exercise to be mapped", &exercises, |e| {
            e.to_line().map(|e| e.to_owned())
        })
        .prompt()?
        .index];

        let maps = db.get_exercise_maps(exercise.id).await?;

        let mut maps_new = maps.clone();

        #[derive(Actions, Clone)]
        enum A {
            Add,
            Edit,
            Delete,
            Done,
        }

        loop {
            println!(
                "Current:\n{}",
                to_table(&maps_new.iter().map(|e| e.to_line()).collect_vec())
            );
            match A::get("Select action", None)? {
                A::Add => {
                    let mgs = mgs
                        .iter()
                        .filter(|e| maps_new.iter().find(|e2| e2.id == e.id).is_none())
                        .collect_vec();
                    let mg = mgs[select_line("Target muscle group", &mgs, |e| {
                        [e.name.to_owned(), e.desc.to_owned()]
                    })
                    .prompt()?
                    .index];
                    let amount = CustomType::<f64>::new("Set for muscle group per set of exercise")
                        .with_default(1f64)
                        .prompt()?;

                    maps_new.push(db::MuscleMapOut {
                        id: mg.id,
                        name: mg.name.to_owned(),
                        amount,
                    });
                }
                A::Edit => {
                    let map = select_line("which one", &maps_new, |e| e.to_line())
                        .prompt()?
                        .index;
                    maps_new[map].amount =
                        CustomType::<f64>::new("Set for muscle group per set of exercise")
                            .with_default(maps_new[map].amount)
                            .prompt()?;
                }
                A::Delete => {
                    let map = select_line("which one", &maps_new, |e| e.to_line())
                        .prompt()?
                        .index;
                    maps_new.remove(map);
                }
                A::Done => break,
            }
        }

        let a = maps_new
            .into_iter()
            .map(|e| db::MuscleMapIn {
                id: e.id,
                amount: e.amount,
            })
            .collect_vec();
        db.map_exercise(exercise.id, &a).await?;

        if Confirm::new("done").with_default(true).prompt()? {
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
