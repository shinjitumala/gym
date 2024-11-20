use std::{collections::HashMap, str::FromStr};

use crate::com::*;

use chrono::Local;
use serde::Serialize;
use sqlx::{
    query,
    query::{Map, Query},
    query_as,
    sqlite::*,
    Connection, Error,
};

pub struct Db {
    c: SqliteConnection,
}

type Txx<'a> = sqlx::Transaction<'a, Sqlite>;

pub struct Tx<'a> {
    t: Txx<'a>,
}

impl<'a> Tx<'a> {
    fn new(t: Txx<'a>) -> Self {
        Self { t }
    }
    pub async fn commit(self) -> Res<()> {
        Ok(self.t.commit().await?)
    }

    async fn exec<'b>(
        &mut self,
        q: Query<'b, Sqlite, SqliteArguments<'b>>,
    ) -> Res<SqliteQueryResult> {
        Ok(q.execute(&mut *self.t).await?)
    }

    pub async fn new_session(&mut self, place: i64, date: Date) -> Res<i64> {
        let b = date.as_timestamp();
        let a = self
            .exec(query!(
                "INSERT INTO session (place,date) VALUES (?,?)",
                place,
                b
            ))
            .await?;
        Ok(a.last_insert_rowid())
    }

    pub async fn new_set(
        &mut self,
        session: i64,
        exercise: i64,
        load: f64,
        rep: f64,
        tmax: f64,
        desc: String,
    ) -> Res<()> {
        let e = self
            .exec(query!(
                "INSERT INTO _set (exercise, load, rep, desc, tmax) VALUES (?, ?, ?, ?, ?)",
                exercise,
                load,
                rep,
                desc,
                tmax
            ))
            .await?;
        let id = e.last_insert_rowid();
        self.exec(query!(
            "INSERT INTO session2set (session, _set) VALUES (?, ?)",
            session,
            id
        ))
        .await?;
        Ok(())
    }

    pub async fn get_exercise(&mut self, name: &str) -> Res<Exercise> {
        let a = query_as!(Exercise, "SELECT * FROM exercise WHERE name = ?", name)
            .fetch_all(&mut *self.t)
            .await?;
        Ok(if !a.is_empty() {
            a[0].to_owned()
        } else {
            let e = self
                .exec(query!(
                    "INSERT INTO exercise (name,desc) VALUES (?,'')",
                    name
                ))
                .await?;
            Exercise {
                id: e.last_insert_rowid(),
                name: name.to_owned(),
                desc: format!(""),
            }
        })
    }
}

impl Db {
    pub async fn new(c: &C) -> Res<Self> {
        let o = SqliteConnectOptions::from_str(&c.cfg.db)?;
        let mut c = SqliteConnection::connect_with(&o).await?;
        query!("PRAGMA FOREIGN_KEYS = ON").execute(&mut c).await?;
        Ok(Self { c })
    }

    pub async fn exercises(&mut self, place: i64) -> Res<Vec<Exercise>> {
        Ok(query_as!(
            Exercise,
            r"
                SELECT exercise.id, exercise.name, exercise.desc
                FROM session
                INNER JOIN place ON place.id = session.place
                INNER JOIN session2set ON session2set.session = session.id
                INNER JOIN _set ON session2set._set = _set.id
                INNER JOIN exercise ON _set.exercise = exercise.id
                WHERE place.id = ?
                GROUP BY exercise.id
        ",
            place
        )
        .fetch_all(&mut self.c)
        .await?)
    }

    async fn exec<'a>(
        &mut self,
        q: Query<'a, Sqlite, SqliteArguments<'a>>,
    ) -> Res<SqliteQueryResult> {
        Ok(q.execute(&mut self.c).await?)
    }
    async fn query<'a, O: Send + Unpin>(
        &mut self,
        q: Map<'a, Sqlite, impl FnMut(SqliteRow) -> Result<O, Error> + Send, SqliteArguments<'a>>,
    ) -> Res<Vec<O>> {
        Ok(q.fetch_all(&mut self.c).await?)
    }

    pub async fn start(&mut self) -> Res<Tx> {
        Ok(Tx::new(self.c.begin().await?))
    }

    pub async fn places(&mut self) -> Res<Vec<Place>> {
        Ok(query_as!(Place, "SELECT place.id, place.name, place.desc FROM place INNER JOIN session ON place.id = session.place GROUP BY place ORDER BY COUNT(session.id) DESC")
            .fetch_all(&mut self.c)
            .await?)
    }

    pub async fn add_weight(&mut self, date: Date, kg: f64, bodyfat: f64, note: String) -> Res<()> {
        let date = date.as_timestamp();
        self.exec(query!(
            "INSERT INTO weight (date, kg, bodyfat, desc) VALUES (?, ?, ?, ?)",
            date,
            kg,
            bodyfat,
            note
        ))
        .await?;
        Ok(())
    }

    pub async fn get_prog(&mut self) -> Res<Prog> {
        let x = {
            let mut x = HashMap::<String, Vec<BestSet>>::new();
            let r = self.query(query!(r"
                SELECT session.date, place.name AS place, exercise.name AS exercise, MAX(_set.tmax) AS tmax, _set.load, _set.rep, _set.desc
                FROM session
                INNER JOIN place ON place.id = session.place
                INNER JOIN session2set ON session2set.session = session.id
                INNER JOIN _set ON session2set._set = _set.id
                INNER JOIN exercise ON _set.exercise = exercise.id
                GROUP BY date, place, exercise
                ORDER BY exercise;
            ")).await?;
            for r in r {
                let exercise = format!("{}@{}", r.exercise.unwrap(), r.place.unwrap());
                let e = match x.get_mut(&exercise) {
                    Some(e) => e,
                    None => {
                        x.insert(exercise.clone(), Vec::new());
                        x.get_mut(&exercise).unwrap()
                    }
                };

                let b = BestSet {
                    date: Date::from_timestamp(r.date.unwrap()),
                    max: r.tmax,
                    load: r.load.unwrap(),
                    rep: r.rep.unwrap(),
                    desc: r.desc.to_owned().unwrap(),
                };
                e.push(b);
            }
            for (_k, v) in x.iter_mut() {
                v.sort_by(|l, r| l.date.cmp(&r.date));
            }
            x
        };
        Ok(x)
    }

    pub async fn new_place(&mut self, name: &str, desc: &str) -> Res<()> {
        self.exec(query!(
            "INSERT INTO place (name, desc) VALUES (?, ?)",
            name,
            desc
        ))
        .await?;
        Ok(())
    }

    pub async fn get_weight(&mut self) -> Res<Vec<Weight>> {
        let w: Vec<_> = self
            .query(query!("SELECT * FROM weight"))
            .await?
            .into_iter()
            .map(|e| Weight {
                date: Date::from_timestamp(e.date),
                kg: e.kg,
                bodyfat: e.bodyfat,
                desc: e.desc.to_owned(),
            })
            .collect();
        Ok(w)
    }

    pub async fn get_exercise_history(
        &mut self,
        place: i64,
        exercise: i64,
    ) -> Res<Vec<ExerciseHistoryItem>> {
        let r = query_as!(
            ExerciseHistoryItem,
            r"
                SELECT session.date, _set.load, _set.rep, _set.desc
                FROM session
                INNER JOIN place ON place.id = session.place
                INNER JOIN session2set ON session2set.session = session.id
                INNER JOIN _set ON session2set._set = _set.id
                INNER JOIN exercise ON _set.exercise = exercise.id
                WHERE session.id IN (
                    SELECT session.id FROM session
                    INNER JOIN place ON place.id = session.place
                    INNER JOIN session2set ON session2set.session = session.id
                    INNER JOIN _set ON session2set._set = _set.id
                    INNER JOIN exercise ON _set.exercise = exercise.id
                    WHERE place.id = ? AND exercise.id = ?
                    GROUP BY session.id
                    ORDER BY session.date DESC LIMIT 4
                ) AND place.id = ? AND exercise.id = ?
                ORDER BY session.date DESC, _set.load DESC, _set.rep DESC
            ",
            place,
            exercise,
            place,
            exercise
        )
        .fetch_all(&mut self.c)
        .await?;

        Ok(r)
    }

    pub async fn get_meals(&mut self) -> Res<Meals> {
        let offset_seconds = Local::now().offset().local_minus_utc();

        let daily_totals = query_as!(
            MealsDaily,
            "SELECT
            strftime('%Y-%m-%d', DATETIME(date + ?, 'unixepoch')) AS date,
            SUM(food.calories * meal.amount) AS calories,
            SUM(food.protein * meal.amount) AS protein,
            SUM(food.fat * meal.amount) AS fat,
            SUM(food.carbohydrate * meal.amount) AS carbohydrate
            FROM meal
            INNER JOIN food WHERE meal.food = food.id
            GROUP BY strftime('%Y-%m-%d', DATETIME(date + ?, 'unixepoch'));",
            offset_seconds,
            offset_seconds
        )
        .fetch_all(&mut self.c)
        .await?;
        let breakdown = query_as!(
            Meal,
            "SELECT
            strftime('%Y-%m-%d', DATETIME(date + ?, 'unixepoch')) AS date,
            food.name,
            food.calories * meal.amount AS calories,
            food.protein * meal.amount AS protein,
            food.fat * meal.amount AS fat,
            food.carbohydrate  * meal.amount AS carbohydrate, 
            meal.amount
            FROM meal
            INNER JOIN food WHERE meal.food = food.id
            ORDER BY food.calories * meal.amount DESC;",
            offset_seconds,
        )
        .fetch_all(&mut self.c)
        .await?;

        Ok(Meals {
            daily: daily_totals,
            breakdown,
        })
    }

    pub async fn foods(&mut self) -> Res<Vec<Food>> {
        Ok(query_as!(Food, "SELECT * FROM food")
            .fetch_all(&mut self.c)
            .await?)
    }
    pub async fn new_food(
        &mut self,
        name: &str,
        calories: f64,
        protein: Option<f64>,
        fat: Option<f64>,
        carbohydrate: Option<f64>,
        desc: &str,
    ) -> Res<i64> {
        let e = self.exec(query!("INSERT INTO food (name, calories, protein, fat, carbohydrate, desc) VALUES (?, ?, ?, ?, ?, ?)", name, calories, protein, fat, carbohydrate,desc)).await?;
        Ok(e.last_insert_rowid())
    }
    pub async fn new_meal(&mut self, date: i64, food: i64, amount: f64, desc: &str) -> Res<()> {
        self.exec(query!(
            "INSERT INTO meal (date, food, amount, desc) VALUES (?, ?, ?, ?)",
            date,
            food,
            amount,
            desc
        ))
        .await?;
        Ok(())
    }
    pub async fn get_exercise_maps(&mut self, exercise: i64) -> Res<Vec<MuscleMapOut>> {
        Ok(query_as!(
            MuscleMapOut,
            "SELECT musclegroup as id, musclegroup.name AS name, amount FROM exercise2musclegroup
                INNER JOIN musclegroup ON musclegroup.id = musclegroup
                WHERE exercise = ?",
            exercise
        )
        .fetch_all(&mut self.c)
        .await?)
    }
    pub async fn map_exercise(&mut self, exercise: i64, muscle_maps: &[MuscleMapIn]) -> Res<()> {
        for i in muscle_maps {
            self.exec(query!(
                "INSERT OR IGNORE INTO exercise2musclegroup (exercise, musclegroup, amount) VALUES (?, ?, ?)",
                exercise,
                i.id,
                i.amount,
            )).await?;
        }
        Ok(())
    }
    pub async fn muscle_groups(&mut self) -> Res<Vec<MuscleGroup>> {
        Ok(query_as!(MuscleGroup, "SELECT * FROM musclegroup;")
            .fetch_all(&mut self.c)
            .await?)
    }
}

#[derive(Clone, Debug)]
pub struct MuscleMapIn {
    pub id: i64,
    pub amount: f64,
}

#[derive(Clone, Debug)]
pub struct MuscleGroup {
    pub id: i64,
    pub name: String,
    pub desc: String,
}

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
    pub date: i64,
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

#[derive(Clone)]
pub struct Place {
    pub id: i64,
    pub name: String,
    pub desc: String,
}
impl Place {
    pub fn to_line(&self) -> [&str; 2] {
        [&self.name, &self.desc]
    }
}

#[derive(Serialize, Clone)]
pub struct Weight {
    date: Date,
    kg: f64,
    bodyfat: f64,
    desc: String,
}
pub type Prog = HashMap<String, Vec<BestSet>>;

#[derive(Clone)]
pub struct Exercise {
    pub id: i64,
    pub name: String,
    pub desc: String,
}
impl Exercise {
    pub fn to_line(&self) -> [&str; 2] {
        [&self.name, &self.desc]
    }
}

#[derive(Serialize, Clone)]
pub struct Food {
    pub id: i64,
    pub name: String,
    pub calories: f64,
    pub protein: Option<f64>,
    pub fat: Option<f64>,
    pub carbohydrate: Option<f64>,
    pub desc: String,
}
impl Food {
    pub fn to_line(&self) -> [String; 6] {
        [
            self.name.to_owned(),
            format!("{:.1}", self.calories),
            self.protein
                .map(|e| format!("{:.1}", e))
                .unwrap_or(String::new()),
            self.fat
                .map(|e| format!("{:.1}", e))
                .unwrap_or(String::new()),
            self.carbohydrate
                .map(|e| format!("{:.1}", e))
                .unwrap_or(String::new()),
            format!("{:.1}", self.desc),
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

#[derive(Serialize, Clone)]
pub struct Meals {
    pub daily: Vec<MealsDaily>,
    pub breakdown: Vec<Meal>,
}

#[derive(Serialize, Clone)]
pub struct MealsDaily {
    pub date: Option<String>,
    pub calories: Option<f64>,
    pub protein: Option<f64>,
    pub fat: Option<f64>,
    pub carbohydrate: Option<f64>,
}

#[derive(Serialize, Clone)]
pub struct Meal {
    pub date: Option<String>,
    pub name: String,
    pub calories: f64,
    pub fat: Option<f64>,
    pub protein: Option<f64>,
    pub carbohydrate: Option<f64>,
    pub amount: Option<f64>,
}
