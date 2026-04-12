mod util;

use crate::com::*;
use chrono::{DateTime, Duration, Local, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    path::PathBuf,
};
use util::*;
pub use util::{s2t, t2s};

pub struct Db {
    pub exercises: FileDb<Exercise>,
    pub muscle_groups: FileDb<MuscleGroup>,
    pub place: FileDb<Place>,
    pub set: DirDb<Set>,
    pub session: DirDb<Session>,
    pub weight: DirDb<Weight>,
    pub food: FileDb<Food>,
    pub meal: DirDb<Meal>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Exercise {
    pub name: String,
    pub desc: String,
    pub muscle_groups: Vec<Exercise2MuscleGroup>,
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Exercise2MuscleGroup {
    group: usize,
    amount: f64,
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct MuscleGroup {
    pub name: String,
    pub desc: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Place {
    name: String,
    desc: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Set {
    date: String,
    session: usize,
    exercise: usize,
    load: f64,
    rep: f64,
    desc: String,
}
impl Timed for Set {
    fn time(&self) -> &str {
        &self.date
    }
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Session {
    date: String,
    place: usize,
    sets: Vec<usize>,
    desc: String,
}
impl Timed for Session {
    fn time(&self) -> &str {
        &self.date
    }
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Weight {
    pub date: String,
    pub kg: f64,
    pub bodyfat: f64,
    pub desc: String,
}
impl Timed for Weight {
    fn time(&self) -> &str {
        &self.date
    }
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Food {
    pub name: String,
    pub calories: f64,
    pub protein: f64,
    pub fat: f64,
    pub carbohydrate: f64,
    pub desc: String,
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Meal {
    date: String,
    food: usize,
    amount: f64,
    desc: String,
}
impl Timed for Meal {
    fn time(&self) -> &str {
        &self.date
    }
}

impl Db {
    const F_EXERCISE: &str = "exercise.toml";
    const F_MUSCLE_GROUP: &str = "muscle_group.toml";
    const F_EXERCISE2MUSCLE_GROUP: &str = "exercise2muscle_group.toml";
    const F_PLACE: &str = "place.toml";
    const D_SET: &str = "set";
    const D_SESSION: &str = "session";
    const D_WEIGHT: &str = "weight";
    const F_FOOD: &str = "food.toml";
    const D_MEAL: &str = "meal";

    pub fn new(c: &C) -> Res<Self> {
        let base = PathBuf::from(&c.cfg.db);
        let e = base
            .try_exists()
            .map_err(|e| format!("Failed to read db directory '{}' because '{e}'.", &c.cfg.db))?;
        if !e {
            return Err(format!(
                "Database directory '{}' does not exist.",
                &c.cfg.db
            ))?;
        }

        Ok(Self {
            exercises: FileDb::load(base.join(Self::F_EXERCISE)).unwrap_or_default(),
            muscle_groups: FileDb::load(base.join(Self::F_MUSCLE_GROUP)).unwrap_or_default(),
            place: FileDb::load(base.join(Self::F_PLACE)).unwrap_or_default(),
            set: DirDb::load(base.join(Self::D_SET), DirDbType::Day)?,
            session: DirDb::load(base.join(Self::D_SESSION), DirDbType::Month)?,
            weight: DirDb::load(base.join(Self::D_WEIGHT), DirDbType::Month)?,
            food: FileDb::load(base.join(Self::F_FOOD)).unwrap_or_default(),
            meal: DirDb::load(base.join(Self::D_MEAL), DirDbType::Day)?,
        })
    }
    pub fn load_full(&mut self) -> Res<()> {
        self.set.load_full()?;
        self.session.load_full()?;
        self.weight.load_full()?;
        self.meal.load_full()?;
        Ok(())
    }
    pub fn save(&self) -> Res<()> {
        self.exercises.save()?;
        self.muscle_groups.save()?;
        self.place.save()?;
        self.set.save()?;
        self.session.save()?;
        self.weight.save()?;
        self.food.save()?;
        self.meal.save()?;
        Ok(())
    }

    pub fn exercises(&self, place: usize) -> Res<Vec<(&usize, &Exercise)>> {
        let sids: HashSet<_> = self
            .session
            .find(|(_, e)| e.place == place)?
            .into_iter()
            .map(|(_, e)| e.sets)
            .flatten()
            .collect();
        let eids: HashSet<_> = self
            .set
            .find(|(id, _)| sids.contains(id))?
            .into_iter()
            .map(|(_, e)| e.exercise)
            .collect();

        Ok(self
            .exercises
            .e
            .e
            .iter()
            .filter(|(id, _)| eids.contains(*id))
            .collect())
    }

    pub fn new_session(&mut self, place: usize, d: Date, desc: String) -> Res<usize> {
        let id = self.session.add(Session {
            date: d.to_string(),
            place,
            sets: Vec::new(),
            desc,
        });
        Ok(id)
    }
    pub fn get_exercise(&mut self, exercise: &str) -> Res<(usize, Exercise)> {
        if let Some((id, e)) = self.exercises.e.e.iter().find(|(_, e)| e.name == exercise) {
            return Ok((*id, e.clone()));
        }

        let e = Exercise {
            name: exercise.to_string(),
            ..Exercise::default()
        };
        let id = self.exercises.e.add(e.clone());
        Ok((id, e))
    }
    pub fn get_exercise_history(
        &self,
        place: usize,
        exercise: usize,
        limit: usize,
    ) -> Res<Vec<ExerciseHistoryItem>> {
        let mut s = HashSet::new();
        Ok(self
            .set
            .find(|(_, e)| e.exercise == exercise)?
            .into_iter()
            .sorted_by_key(|(_, e)| e.date.to_owned())
            .rev()
            .map(|(id, e)| -> Res<_> {
                let x = self
                    .session
                    .find(|(_, e)| e.sets.contains(&id) && e.place == place)?;
                Ok((e, x))
            })
            .filter_map_ok(|(s, e)| e.into_iter().next().map(|(sid, e)| (s, sid, e)))
            .take_while(|e| {
                if let Ok((_, sid, _)) = e {
                    s.insert(*sid);
                };
                s.len() <= limit
            })
            .map_ok(|(e, _, session)| ExerciseHistoryItem {
                date: session.date,
                load: e.load,
                rep: e.rep,
                desc: e.desc,
            })
            .process_results(|i| i.collect())?)
    }

    pub fn places(&self) -> Vec<(&usize, &Place)> {
        self.place.e.e.iter().collect()
    }

    pub fn new_set<T: TimeZone>(
        &mut self,
        date: DateTime<T>,
        session: usize,
        exercise: usize,
        load: f64,
        rep: f64,
        one_rep_max: f64,
        desc: String,
    ) -> Res<()> {
        self.set.add(Set {
            date: t2s(date),
            session,
            exercise,
            load,
            rep,
            desc,
        });
        Ok(())
    }

    pub fn add_weight(&mut self, date: Date, kg: f64, bodyfat: f64, note: String) -> Res<()> {
        self.weight.add(Weight {
            date: date.to_string(),
            kg,
            bodyfat,
            desc: note,
        });
        Ok(())
    }

    pub async fn major_exercise_maps(&mut self) -> Res<MajorExerciseMaps> {
        todo!()
    }

    pub async fn get_prog(&mut self) -> Res<Prog> {
        todo!()
    }

    pub async fn new_place(&mut self, name: &str, desc: &str) -> Res<()> {
        todo!()
    }

    pub async fn get_weight(&mut self) -> Res<Vec<Weight>> {
        todo!()
    }

    pub fn calories_today(&mut self) -> Res<MealsDaily> {
        let n = Local::now();
        let o = Duration::seconds((n.hour() * 60 * 60 + n.minute() * 60 + n.second()) as i64);
        let s = n - o;
        let e = s + Duration::days(1);
        println!("{} {}", s.to_rfc3339(), e.to_rfc3339());

        let e = self
            .meal
            .find(|(_, m)| s2t(&m.date).map(|t| s <= t && t < e).unwrap_or(false))?
            .iter()
            .map(|(_, m)| -> Res<_> {
                let f = self
                    .food
                    .e
                    .e
                    .get(&m.food)
                    .ok_or(Err::Str(format!("No food with id '{}'.", m.food)))?;
                Ok((m, f))
            })
            .process_results(|i| {
                i.fold(MealsDaily::default(), |mut c, (m, f)| {
                    c.calories += m.amount * f.calories;
                    c.protein += m.amount * f.protein;
                    c.fat += m.amount * f.fat;
                    c.carbohydrate += m.amount * f.carbohydrate;
                    c
                })
            })?;
        Ok(e)
    }

    // pub async fn get_meals(&mut self) -> Res<Meals> {
    //     todo!()
    // }

    pub fn foods(&mut self) -> Res<Vec<(&usize, &Food)>> {
        Ok(self.food.e.e.iter().collect())
    }
    pub fn new_food(
        &mut self,
        name: &str,
        calories: f64,
        protein: Option<f64>,
        fat: Option<f64>,
        carbohydrate: Option<f64>,
        desc: &str,
    ) -> Res<usize> {
        let id = self.food.e.add(Food {
            name: name.to_owned(),
            calories,
            protein: protein.unwrap_or_default(),
            fat: fat.unwrap_or_default(),
            carbohydrate: carbohydrate.unwrap_or_default(),
            desc: desc.to_owned(),
        });
        Ok(id)
    }
    pub fn new_meal<T: TimeZone>(
        &mut self,
        date: DateTime<T>,
        food: usize,
        amount: f64,
        desc: &str,
    ) -> Res<()> {
        self.meal.add(Meal {
            date: t2s(date),
            food,
            amount,
            desc: desc.to_owned(),
        });
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct MuscleMapIn {
    pub id: i64,
    pub amount: f64,
}

// #[derive(Clone, Debug, Serialize)]
// pub struct MuscleGroup {
//     pub id: i64,
//     pub name: String,
//     pub desc: String,
// }

#[derive(Clone, Debug)]
pub struct MuscleMapOut {
    pub id: i64,
    pub name: String,
    pub amount: f64,
}
impl MuscleMapOut {
    pub fn to_line(&self) -> [String; 2] {
        [self.name.to_owned(), format!("{:.2}", self.amount)]
    }
}

#[derive(Clone, Debug)]
pub struct ExerciseHistoryItem {
    pub date: String,
    pub load: f64,
    pub rep: f64,
    pub desc: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct BestSet {
    pub date: Date,
    pub load: f64,
    pub rep: f64,
    pub max: f64,
    pub desc: String,
}

// #[derive(Clone)]
// pub struct Place {
//     pub id: i64,
//     pub name: String,
//     pub desc: String,
// }
impl Place {
    pub fn to_line(&self) -> [&str; 2] {
        [&self.name, &self.desc]
    }
}

// #[derive(Serialize, Clone)]
// pub struct Weight {
//     pub date: Date,
//     pub kg: f64,
//     pub bodyfat: f64,
//     pub desc: String,
// }
pub type Prog = HashMap<String, Vec<BestSet>>;

// #[derive(Clone, Serialize, Deserialize, Debug)]
// pub struct Exercise {
//     pub id: i64,
//     pub name: String,
//     pub desc: String,
// }
impl Exercise {
    pub fn to_line(&self) -> [&str; 2] {
        [&self.name, &self.desc]
    }
}

// #[derive(Serialize, Clone)]
// pub struct Food {
//     pub id: i64,
//     pub name: String,
//     pub calories: f64,
//     pub protein: Option<f64>,
//     pub fat: Option<f64>,
//     pub carbohydrate: Option<f64>,
//     pub desc: String,
// }
impl Food {
    pub fn to_line(&self) -> [String; 6] {
        [
            self.name.to_owned(),
            format!("{:.1}", self.calories),
            format!("{:.1}", self.protein),
            // self.protein
            //     .map(|e| format!("{:.1}", e))
            //     .unwrap_or(String::new()),
            format!("{:.1}", self.fat),
            // self.fat
            //     .map(|e| format!("{:.1}", e))
            //     .unwrap_or(String::new()),
            format!("{:.1}", self.carbohydrate),
            // self.carbohydrate
            //     .map(|e| format!("{:.1}", e))
            //     .unwrap_or(String::new()),
            format!("{}", self.desc),
        ]
    }
    pub fn head() -> [String; 6] {
        [
            format!("name"),
            format!("calorie"),
            format!("protein"),
            format!("fat"),
            format!("carbohydrate"),
            format!("desc"),
        ]
    }

    pub fn to_line2(&self) -> [String; 3] {
        [
            self.name.to_owned(),
            format!("{:.1}", self.calories),
            format!("{:.1}", self.desc),
        ]
    }
    pub fn head2() -> [String; 3] {
        [format!("name"), format!("calorie"), format!("desc")]
    }
    pub fn print(&self) -> String {
        let x = [Food::head(), self.to_line()];
        format!("{}", to_table(&x))
    }
}

// #[derive(Serialize, Clone)]
// pub struct Meals {
//     pub daily: Vec<MealsDaily>,
//     pub breakdown: Vec<Meal>,
// }

#[derive(Serialize, Clone, Default)]
pub struct MealsDaily {
    pub calories: f64,
    pub protein: f64,
    pub fat: f64,
    pub carbohydrate: f64,
}
impl MealsDaily {
    pub fn to_lines_today(&self) -> [[String; 4]; 2] {
        [
            [
                format!("calories kcal"),
                format!("protein g"),
                format!("fat g"),
                format!("carbohydrate g"),
            ],
            [
                format!("{:.2}", self.calories),
                format!("{:.2}", self.protein),
                format!("{:.2}", self.fat),
                format!("{:.2}", self.carbohydrate),
            ],
        ]
    }
}

// #[derive(Serialize, Clone)]
// pub struct Meal {
//     pub date: Option<String>,
//     pub name: String,
//     pub calories: f64,
//     pub fat: Option<f64>,
//     pub protein: Option<f64>,
//     pub carbohydrate: Option<f64>,
//     pub amount: Option<f64>,
//     pub desc: String,
// }

pub type MajorExerciseMaps = HashMap<String, Vec<String>>;

// #[derive(Serialize)]
// pub struct Sets {
//     pub date: Date,
//     pub place: String,
//     pub exercise: String,
//     pub count: f64,
//     pub mg: String,
//     pub desc: f64,
// }
