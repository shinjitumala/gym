use std::{collections::HashMap, str::FromStr};

use crate::com::*;

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
        desc: String,
    ) -> Res<()> {
        let e = self
            .exec(query!(
                "INSERT INTO _set (exercise, load, rep, desc) VALUES (?, ?, ?, ?)",
                exercise,
                load,
                rep,
                desc
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
        Ok(query_as!(Place, "SELECT * FROM place")
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

    pub async fn get_prog(&mut self) -> Res<String> {
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
        let a = serde_json::to_string_pretty(&x).unwrap();
        Ok(a)
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

    pub async fn get_weight(&mut self) -> Res<String> {
        #[derive(Serialize)]
        struct W {
            date: Date,
            kg: f64,
            bodyfat: f64,
            desc: String,
        }
        let w: Vec<_> = self
            .query(query!("SELECT * FROM weight"))
            .await?
            .into_iter()
            .map(|e| W {
                date: Date::from_timestamp(e.date),
                kg: e.kg,
                bodyfat: e.bodyfat,
                desc: e.desc.to_owned(),
            })
            .collect();
        Ok(serde_json::to_string_pretty(&w).unwrap())
    }

    pub async fn adhoc(&mut self) -> Res<()> {
        todo!()
    }
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
