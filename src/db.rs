use crate::{com::*, model::*};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::{path::Path, time::Duration};

pub const MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub struct Db {
    pub pool: Pool<Sqlite>,
    /// The file this pool is connected to, for the pages that show it.
    pub path: String,
}

fn options(p: &Path, create: bool) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(p)
        .create_if_missing(create)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5))
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
}

async fn pool(o: SqliteConnectOptions) -> Res<Pool<Sqlite>> {
    Ok(SqlitePoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(o)
        .await?)
}

async fn connected(p: &Path, create: bool) -> Res<Db> {
    let pool = pool(options(p, create)).await?;
    MIGRATOR.run(&pool).await?;
    Ok(Db {
        pool,
        path: p.to_string_lossy().to_string(),
    })
}

fn existing_file(p: &str) -> Res<&Path> {
    let p = Path::new(p);
    if p.is_dir() {
        Err(format!(
            "'{}' is a directory; the database is a file (see the configuration page)",
            p.display()
        ))?;
    }
    if !p.exists() {
        Err(format!(
            "No database at '{}'. Create one with 'gym db init', or import the TOML archive with 'gym db import <dir>'.",
            p.display()
        ))?;
    }
    Ok(p)
}

async fn new_file(p: &str) -> Res<&Path> {
    let p = Path::new(p);
    if p.is_dir() {
        Err(format!(
            "'{}' is a directory; the database is a file (see the configuration page)",
            p.display()
        ))?;
    }
    if let Some(d) = p.parent() {
        if !d.as_os_str().is_empty() && !d.exists() {
            Err(format!("'{}' does not exist.", d.display()))?;
        }
    }
    if p.exists() {
        refuse_used(p).await?;
    }
    Ok(p)
}

async fn refuse_used(p: &Path) -> Res<()> {
    let existing = pool(options(p, false)).await?;
    let tables: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name <> '_sqlx_migrations'"
    )
    .fetch_one(&existing)
    .await?;
    if tables > 0 {
        Err(format!(
            "'{}' is already a database with {tables} table(s); refusing to initialise it.",
            p.display()
        ))?;
    }
    existing.close().await;
    Ok(())
}

impl Db {
    pub async fn open(p: &str) -> Res<Self> {
        connected(existing_file(p)?, false).await
    }

    pub async fn create(p: &str) -> Res<Self> {
        connected(new_file(p).await?, true).await
    }
}

pub async fn places(db: &Db) -> Res<Vec<Place>> {
    Ok(sqlx::query_as!(
        Place,
        r#"
SELECT p.id AS "id!", p.name, p.note
FROM place p
ORDER BY COALESCE((SELECT MAX(julianday(s.date)) FROM session s WHERE s.place = p.id), 0.0) DESC, p.id
"#
    )
    .fetch_all(&db.pool)
    .await?)
}

pub async fn exercises(db: &Db, place: i64) -> Res<Vec<Exercise>> {
    Ok(sqlx::query_as!(
        Exercise,
        r#"
SELECT e.id AS "id!", e.name, e.note
FROM exercise e
JOIN (
    SELECT ts.exercise AS ex, MAX(julianday(ts.date)) AS last
    FROM training_set ts
    JOIN session s ON s.id = ts.session
    WHERE s.place = ?1
    GROUP BY ts.exercise
) x ON x.ex = e.id
ORDER BY x.last DESC, e.id
"#,
        place
    )
    .fetch_all(&db.pool)
    .await?)
}

pub async fn get_or_create_exercise(db: &Db, name: &str) -> Res<Exercise> {
    Ok(sqlx::query_as!(
        Exercise,
        r#"
INSERT INTO exercise (name) VALUES (?1)
ON CONFLICT (name) DO UPDATE SET name = excluded.name
RETURNING id AS "id!", name, note
"#,
        name
    )
    .fetch_one(&db.pool)
    .await?)
}

/// A session the CLI records: finished as soon as it is written, because the
/// prompts have already asked for every set. The web's live session is the
/// other kind — it stays open until it is ended — so a row written here must
/// not look like one that is still being filled in.
pub async fn new_session(db: &Db, place: i64, date: &str, note: &str) -> Res<i64> {
    Ok(sqlx::query_scalar!(
        r#"INSERT INTO session (date, place, note, ended) VALUES (?1, ?2, ?3, ?1) RETURNING id AS "id!""#,
        date,
        place,
        note
    )
    .fetch_one(&db.pool)
    .await?)
}

pub async fn new_set(
    db: &Db,
    session: i64,
    exercise: i64,
    date: &str,
    load: f64,
    rep: f64,
    note: &str,
) -> Res<i64> {
    Ok(sqlx::query_scalar!(
        r#"
INSERT INTO training_set (session, exercise, position, date, load, rep, note)
SELECT ?1, ?2, COALESCE(MAX(position), -1) + 1, ?3, ?4, ?5, ?6 FROM training_set WHERE session = ?1
RETURNING id AS "id!"
"#,
        session,
        exercise,
        date,
        load,
        rep,
        note
    )
    .fetch_one(&db.pool)
    .await?)
}

pub async fn history(db: &Db, place: i64, exercise: i64, limit: i64) -> Res<Vec<HistoryItem>> {
    Ok(sqlx::query_as!(
        HistoryItem,
        r#"
SELECT s.date AS "date!", ts.load AS "load!", ts.rep AS "rep!", ts.note
FROM training_set ts
JOIN session s ON s.id = ts.session
WHERE ts.exercise = ?1 AND s.place = ?2
  AND ts.session IN (
      SELECT ts2.session
      FROM training_set ts2
      JOIN session s2 ON s2.id = ts2.session
      WHERE ts2.exercise = ?1 AND s2.place = ?2
      GROUP BY ts2.session
      ORDER BY MAX(julianday(s2.date)) DESC
      LIMIT ?3
  )
ORDER BY julianday(ts.date) DESC, ts.id DESC
"#,
        exercise,
        place,
        limit
    )
    .fetch_all(&db.pool)
    .await?)
}

pub async fn add_weight(db: &Db, date: &str, kg: f64, bodyfat: Option<f64>, note: &str) -> Res<()> {
    sqlx::query!(
        "INSERT INTO weight (date, kg, bodyfat, note) VALUES (?1, ?2, ?3, ?4)",
        date,
        kg,
        bodyfat,
        note
    )
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn day_totals(db: &Db, from: &str, to: &str) -> Res<Totals> {
    let r = sqlx::query!(
        r#"
SELECT
    COALESCE(SUM(m.amount * f.calories), 0.0) AS "calories!: f64",
    COALESCE(SUM(m.amount * f.protein), 0.0) AS "protein!: f64",
    COALESCE(SUM(m.amount * f.fat), 0.0) AS "fat!: f64",
    COALESCE(SUM(m.amount * f.carbohydrate), 0.0) AS "carbohydrate!: f64"
FROM meal m
JOIN food f ON f.id = m.food
WHERE julianday(m.date) >= julianday(?1) AND julianday(m.date) < julianday(?2)
"#,
        from,
        to
    )
    .fetch_one(&db.pool)
    .await?;
    Ok(Totals {
        calories: r.calories,
        protein: r.protein,
        fat: r.fat,
        carbohydrate: r.carbohydrate,
    })
}

pub async fn foods(db: &Db) -> Res<Vec<Food>> {
    Ok(sqlx::query_as!(
        Food,
        r#"
SELECT id AS "id!", name, calories AS "calories!", protein, fat, carbohydrate, note
FROM food ORDER BY id
"#
    )
    .fetch_all(&db.pool)
    .await?)
}

pub async fn new_food(
    db: &Db,
    name: &str,
    calories: f64,
    protein: Option<f64>,
    fat: Option<f64>,
    carbohydrate: Option<f64>,
    note: &str,
) -> Res<i64> {
    Ok(sqlx::query_scalar!(
        r#"
INSERT INTO food (name, calories, protein, fat, carbohydrate, note)
VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id
"#,
        name,
        calories,
        protein,
        fat,
        carbohydrate,
        note
    )
    .fetch_one(&db.pool)
    .await?)
}

pub async fn new_meal(db: &Db, date: &str, food: i64, amount: f64, note: &str) -> Res<()> {
    sqlx::query!(
        "INSERT INTO meal (date, food, amount, note) VALUES (?1, ?2, ?3, ?4)",
        date,
        food,
        amount,
        note
    )
    .execute(&db.pool)
    .await?;
    Ok(())
}
