//! The write path: every server function that changes a row.
//!
//! One function per user action, each in its own short transaction, and each
//! taking the values the form collects. Nothing keeps a transaction open across
//! a round trip: the pool is shared with the CLI, so a held write lock would
//! stop a `gym food` in another terminal.

use crate::db::Db;
use crate::util::{day_bounds, t2s, Res};
use crate::web::dto::Totals;
use chrono::{DateTime, Duration, Local, NaiveDate};

/// A day's timestamps, resolved to now-in-that-day.
///
/// The forms ask for a date and a time; the row keeps an RFC3339 instant, as
/// every other writer does.
pub fn instant_of(date: &str, time: &str) -> Res<String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("'{date}' is not a date (expected YYYY-MM-DD) because '{e}'."))?;
    let t = if time.trim().is_empty() {
        Local::now().time()
    } else {
        chrono::NaiveTime::parse_from_str(time, "%H:%M")
            .map_err(|e| format!("'{time}' is not a time (expected HH:MM) because '{e}'."))?
    };
    let naive = d.and_time(t);
    let local = Local
        .from_local_datetime(&naive)
        .single()
        .ok_or_else(|| format!("{date} {time} does not exist in this timezone."))?;
    Ok(t2s(local))
}

use chrono::TimeZone;

pub async fn add_meal(
    db: &Db,
    date: &str,
    time: &str,
    food: i64,
    amount: f64,
    note: &str,
) -> Res<i64> {
    if amount <= 0.0 {
        Err("The amount must be greater than zero.".to_string())?;
    }
    let at = instant_of(date, time)?;
    Ok(sqlx::query_scalar!(
        r#"INSERT INTO meal (date, food, amount, note) VALUES (?1, ?2, ?3, ?4) RETURNING id AS "id!""#,
        at,
        food,
        amount,
        note
    )
    .fetch_one(&db.pool)
    .await?)
}

/// Record a batch of meals at one instant: the form's rows, in one
/// transaction, so a half-entered meal is never written.
pub async fn add_meals(
    db: &Db,
    date: &str,
    time: &str,
    rows: Vec<(i64, f64, String)>,
) -> Res<usize> {
    if rows.is_empty() {
        Err("Add at least one food.".to_string())?;
    }
    for (_, amount, _) in &rows {
        if *amount <= 0.0 {
            Err("Every amount must be greater than zero.".to_string())?;
        }
    }
    let at = instant_of(date, time)?;
    let mut tx = db.pool.begin().await?;
    for (food, amount, note) in &rows {
        sqlx::query!(
            "INSERT INTO meal (date, food, amount, note) VALUES (?1, ?2, ?3, ?4)",
            at,
            food,
            amount,
            note
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(rows.len())
}

/// Start a phase. The old one stays: a session records the phase that was
/// active, so history does not shift when a new phase starts.
pub async fn set_phase(
    db: &Db,
    kind: &str,
    start: &str,
    target_rate: f64,
    target_rep: i64,
    target_weight: Option<f64>,
) -> Res<i64> {
    if kind != "bulk" && kind != "cut" {
        Err(format!("'{kind}' is not a phase: it is 'bulk' or 'cut'."))?;
    }
    if target_rep <= 0 {
        Err("The rep target must be at least one.".to_string())?;
    }
    if let Some(w) = target_weight {
        if !(0.0..500.0).contains(&w) {
            Err(format!("'{w}' is not a plausible target weight."))?;
        }
    }
    let at = instant_of(start, "")?;
    Ok(sqlx::query_scalar!(
        r#"INSERT INTO phase (kind, start, target_rate, target_rep, target_weight)
           VALUES (?1, ?2, ?3, ?4, ?5) RETURNING id AS "id!""#,
        kind,
        at,
        target_rate,
        target_rep,
        target_weight
    )
    .fetch_one(&db.pool)
    .await?)
}

pub async fn delete_meal(db: &Db, id: i64) -> Res<()> {
    let n = sqlx::query!("DELETE FROM meal WHERE id = ?1", id)
        .execute(&db.pool)
        .await?
        .rows_affected();
    if n == 0 {
        Err(format!("No meal with id '{id}'."))?;
    }
    Ok(())
}

pub async fn edit_meal(
    db: &Db,
    id: i64,
    date: &str,
    time: &str,
    food: i64,
    amount: f64,
    note: &str,
) -> Res<()> {
    if amount <= 0.0 {
        Err("The amount must be greater than zero.".to_string())?;
    }
    let at = instant_of(date, time)?;
    let n = sqlx::query!(
        "UPDATE meal SET date = ?1, food = ?2, amount = ?3, note = ?4 WHERE id = ?5",
        at,
        food,
        amount,
        note,
        id
    )
    .execute(&db.pool)
    .await?
    .rows_affected();
    if n == 0 {
        Err(format!("No meal with id '{id}'."))?;
    }
    Ok(())
}

pub async fn add_weight(
    db: &Db,
    date: &str,
    time: &str,
    kg: f64,
    bodyfat: Option<f64>,
    note: &str,
) -> Res<i64> {
    if !(0.0..500.0).contains(&kg) {
        Err(format!("'{kg}' is not a plausible weight in kilograms."))?;
    }
    if let Some(b) = bodyfat {
        if !(0.0..100.0).contains(&b) {
            Err(format!("'{b}' is not a plausible body fat percentage."))?;
        }
    }
    let at = instant_of(date, time)?;
    Ok(sqlx::query_scalar!(
        r#"INSERT INTO weight (date, kg, bodyfat, note) VALUES (?1, ?2, ?3, ?4) RETURNING id AS "id!""#,
        at,
        kg,
        bodyfat,
        note
    )
    .fetch_one(&db.pool)
    .await?)
}

pub async fn edit_weight(
    db: &Db,
    id: i64,
    date: &str,
    time: &str,
    kg: f64,
    bodyfat: Option<f64>,
    note: &str,
) -> Res<()> {
    let at = instant_of(date, time)?;
    let n = sqlx::query!(
        "UPDATE weight SET date = ?1, kg = ?2, bodyfat = ?3, note = ?4 WHERE id = ?5",
        at,
        kg,
        bodyfat,
        note,
        id
    )
    .execute(&db.pool)
    .await?
    .rows_affected();
    if n == 0 {
        Err(format!("No weigh-in with id '{id}'."))?;
    }
    Ok(())
}

pub async fn delete_weight(db: &Db, id: i64) -> Res<()> {
    sqlx::query!("DELETE FROM weight WHERE id = ?1", id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

/// Edit a food's numbers. A `None` leaves that number unknown, which is not the
/// same row as a measured zero.
pub async fn edit_food(
    db: &Db,
    id: i64,
    name: &str,
    calories: f64,
    protein: Option<f64>,
    fat: Option<f64>,
    carbohydrate: Option<f64>,
    note: &str,
) -> Res<()> {
    if name.trim().is_empty() {
        Err("A food needs a name.".to_string())?;
    }
    if calories < 0.0 {
        Err("Calories cannot be negative.".to_string())?;
    }
    let n = sqlx::query!(
        "UPDATE food SET name = ?1, calories = ?2, protein = ?3, fat = ?4, carbohydrate = ?5, note = ?6 WHERE id = ?7",
        name.trim(),
        calories,
        protein,
        fat,
        carbohydrate,
        note,
        id
    )
    .execute(&db.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            crate::util::Err::Str(format!("A food called '{name}' already exists."))
        } else {
            crate::util::Err::from(e)
        }
    })?
    .rows_affected();
    if n == 0 {
        Err(format!("No food with id '{id}'."))?;
    }
    Ok(())
}

pub async fn add_food(
    db: &Db,
    name: &str,
    calories: f64,
    protein: Option<f64>,
    fat: Option<f64>,
    carbohydrate: Option<f64>,
    note: &str,
) -> Res<i64> {
    if name.trim().is_empty() {
        Err("A food needs a name.".to_string())?;
    }
    Ok(sqlx::query_scalar!(
        r#"INSERT INTO food (name, calories, protein, fat, carbohydrate, note)
           VALUES (?1, ?2, ?3, ?4, ?5, ?6) RETURNING id AS "id!""#,
        name.trim(),
        calories,
        protein,
        fat,
        carbohydrate,
        note
    )
    .fetch_one(&db.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            crate::util::Err::Str(format!("A food called '{name}' already exists."))
        } else {
            crate::util::Err::from(e)
        }
    })?)
}

/// Refuse to delete a food the log still refers to, and say how much.
pub async fn delete_food(db: &Db, id: i64) -> Res<()> {
    let used: i64 = sqlx::query_scalar!("SELECT count(*) FROM meal WHERE food = ?1", id)
        .fetch_one(&db.pool)
        .await?;
    if used > 0 {
        Err(format!(
            "{used} meal(s) still use this food; change them first, or keep it."
        ))?;
    }
    let preset: i64 =
        sqlx::query_scalar!("SELECT count(*) FROM meal_preset_item WHERE food = ?1", id)
            .fetch_one(&db.pool)
            .await?;
    if preset > 0 {
        Err(format!(
            "{preset} meal preset(s) still use this food; remove it from them first."
        ))?;
    }
    sqlx::query!("DELETE FROM food WHERE id = ?1", id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

/// The effort row a name refers to, or `None` for "not recorded".
async fn effort_id(
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite>,
    name: &str,
) -> Res<Option<i64>> {
    if name.trim().is_empty() {
        return Ok(None);
    }
    Ok(
        sqlx::query_scalar!(r#"SELECT id FROM effort WHERE name = ?1"#, name.trim())
            .fetch_optional(exec)
            .await?,
    )
}

/// Start a session: one row, open until it is ended.
///
/// The phase is the one in force at the start, recorded on the session so the
/// analysis keeps the phase the session was trained in.
pub async fn start_session(db: &Db, place: i64, date: &str, time: &str) -> Res<i64> {
    let open: i64 = sqlx::query_scalar!("SELECT count(*) FROM session WHERE ended IS NULL")
        .fetch_one(&db.pool)
        .await?;
    if open > 0 {
        Err("A session is already open; end it before starting another.".to_string())?;
    }
    let at = instant_of(date, time)?;
    Ok(sqlx::query_scalar!(
        r#"INSERT INTO session (date, place, phase) VALUES (?1, ?2, ?3) RETURNING id AS "id!""#,
        at,
        place,
        sqlx::query_scalar!(
            r#"SELECT id FROM phase WHERE julianday(start) <= julianday(?1) ORDER BY julianday(start) DESC LIMIT 1"#,
            at
        )
        .fetch_optional(&db.pool)
        .await?
    )
    .fetch_one(&db.pool)
    .await?)
}

/// Put an exercise on the session's checklist, once.
pub async fn add_exercise(db: &Db, session: i64, exercise: i64) -> Res<()> {
    sqlx::query!(
        r#"INSERT INTO session_list_item (session, exercise, position)
           SELECT ?1, ?2, COALESCE(MAX(position), -1) + 1 FROM session_list_item WHERE session = ?1
           ON CONFLICT (session, exercise) DO NOTHING"#,
        session,
        exercise
    )
    .execute(&db.pool)
    .await?;
    Ok(())
}

/// Apply a preset to a session's checklist: every item it does not have yet,
/// in the preset's order.
pub async fn apply_exercise_preset(db: &Db, session: i64, preset: i64) -> Res<usize> {
    let items = sqlx::query!(
        r#"SELECT exercise AS "exercise!" FROM exercise_list_item WHERE list = ?1 ORDER BY position"#,
        preset
    )
    .fetch_all(&db.pool)
    .await?;
    if items.is_empty() {
        Err("That preset has no exercises yet.".to_string())?;
    }
    let mut tx = db.pool.begin().await?;
    let mut added = 0;
    for i in items {
        let n = sqlx::query!(
            r#"INSERT INTO session_list_item (session, exercise, position)
               SELECT ?1, ?2, COALESCE(MAX(position), -1) + 1 FROM session_list_item WHERE session = ?1
               ON CONFLICT (session, exercise) DO NOTHING"#,
            session,
            i.exercise
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        added += n as usize;
    }
    tx.commit().await?;
    Ok(added)
}

/// Refuse to write to a session that has ended, or that does not exist.
///
/// Every set and every superset group goes through here, so a page left open
/// on a finished session fails with a sentence rather than tucking a row into
/// yesterday's log.
async fn open_session(db: &Db, session: i64) -> Res<()> {
    let ended: Option<Option<String>> =
        sqlx::query_scalar!(r#"SELECT ended FROM session WHERE id = ?1"#, session)
            .fetch_optional(&db.pool)
            .await?;
    match ended {
        None => Err(crate::util::Err::Str(format!(
            "No session with id '{session}'."
        ))),
        Some(Some(_)) => Err(crate::util::Err::Str(
            "That session has ended; start a new one.".to_string(),
        )),
        Some(None) => Ok(()),
    }
}

/// Record one set: the row is stamped with the moment it was saved.
///
/// The effort is stored as a row, so the ranking lives in the database rather
/// than in a match on a string.
pub async fn log_set(
    db: &Db,
    session: i64,
    exercise: i64,
    load: f64,
    rep: f64,
    effort: &str,
    note: &str,
) -> Res<i64> {
    if load < 0.0 || rep <= 0.0 {
        Err("A set needs a load of at least zero and at least one rep.".to_string())?;
    }
    open_session(db, session).await?;
    let at = Local::now().to_rfc3339();
    let mut tx = db.pool.begin().await?;
    sqlx::query!(
        r#"INSERT INTO session_list_item (session, exercise, position)
           SELECT ?1, ?2, COALESCE(MAX(position), -1) + 1 FROM session_list_item WHERE session = ?1
           ON CONFLICT (session, exercise) DO NOTHING"#,
        session,
        exercise
    )
    .execute(&mut *tx)
    .await?;
    let id = sqlx::query_scalar!(
        r#"INSERT INTO training_set (session, exercise, position, date, load, rep, note, effort)
           SELECT ?1, ?2, COALESCE(MAX(position), -1) + 1, ?3, ?4, ?5, ?6, ?7
           FROM training_set WHERE session = ?1
           RETURNING id AS "id!""#,
        session,
        exercise,
        at,
        load,
        rep,
        note,
        effort_id(&mut *tx, effort).await?
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

/// Record two or more exercises as one superset: one group row and one set per
/// exercise, sharing the load and each with its own reps.
///
/// The group is what makes a superset more than two adjacent rows: the sets
/// carry its id, so the log can say they were done back to back with no rest
/// between them.
pub async fn log_superset(
    db: &Db,
    session: i64,
    load: f64,
    reps: &[(i64, f64)],
    effort: &str,
    note: &str,
) -> Res<i64> {
    if load < 0.0 {
        Err("A superset needs a load of at least zero.".to_string())?;
    }
    if reps.len() < 2 {
        Err(
            "A superset is two or more exercises: give a rep count for at least two of them."
                .to_string(),
        )?;
    }
    if reps.iter().any(|(_, rep)| *rep <= 0.0) {
        Err("Every set of a superset needs at least one rep.".to_string())?;
    }
    open_session(db, session).await?;
    let at = Local::now().to_rfc3339();
    let mut tx = db.pool.begin().await?;
    let group = sqlx::query_scalar!(
        r#"INSERT INTO superset (session) VALUES (?1) RETURNING id AS "id!""#,
        session
    )
    .fetch_one(&mut *tx)
    .await?;
    let effort = effort_id(&mut *tx, effort).await?;
    let mut position: i64 = sqlx::query_scalar!(
        "SELECT COALESCE(MAX(position), -1) FROM training_set WHERE session = ?1",
        session
    )
    .fetch_one(&mut *tx)
    .await?;
    for (exercise, rep) in reps {
        position += 1;
        sqlx::query!(
            r#"INSERT INTO session_list_item (session, exercise, position)
               SELECT ?1, ?2, COALESCE(MAX(position), -1) + 1 FROM session_list_item WHERE session = ?1
               ON CONFLICT (session, exercise) DO NOTHING"#,
            session,
            exercise
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            r#"INSERT INTO training_set (session, exercise, position, date, load, rep, note, effort, superset)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            session,
            exercise,
            position,
            at,
            load,
            rep,
            note,
            effort,
            group
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(group)
}

/// End the open session.
///
/// A session with no sets recorded nothing, so ending it removes the row: there
/// is no half-session in the log, and no way to be trapped by one started by
/// mistake.
pub async fn end_session(db: &Db, session: i64) -> Res<String> {
    let sets: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM training_set WHERE session = ?1",
        session
    )
    .fetch_one(&db.pool)
    .await?;
    if sets == 0 {
        sqlx::query!("DELETE FROM session WHERE id = ?1", session)
            .execute(&db.pool)
            .await?;
        return Ok("No sets were recorded, so the session was discarded.".to_string());
    }
    let n = sqlx::query!(
        "UPDATE session SET ended = ?1 WHERE id = ?2 AND ended IS NULL",
        Local::now().to_rfc3339(),
        session
    )
    .execute(&db.pool)
    .await?
    .rows_affected();
    if n == 0 {
        Err(format!("No open session with id '{session}'."))?;
    }
    Ok(format!("Session ended with {sets} set(s)."))
}

/// Flag or unflag a session as a deload. The reminder is advice; this is the
/// only thing that ever sets the flag.
pub async fn set_deload(db: &Db, id: i64, deload: bool) -> Res<()> {
    sqlx::query!("UPDATE session SET deload = ?1 WHERE id = ?2", deload, id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn edit_set(db: &Db, id: i64, load: f64, rep: f64, effort: &str, note: &str) -> Res<()> {
    if load < 0.0 || rep <= 0.0 {
        Err("A set needs a load of at least zero and at least one rep.".to_string())?;
    }
    let n = sqlx::query!(
        "UPDATE training_set SET load = ?1, rep = ?2, note = ?3, effort = ?4 WHERE id = ?5",
        load,
        rep,
        note,
        effort_id(&db.pool, effort).await?,
        id
    )
    .execute(&db.pool)
    .await?
    .rows_affected();
    if n == 0 {
        Err(format!("No set with id '{id}'."))?;
    }
    Ok(())
}

/// Delete a set and close the gap in its session's positions.
pub async fn delete_set(db: &Db, id: i64) -> Res<()> {
    let mut tx = db.pool.begin().await?;
    let row = sqlx::query!(
        r#"SELECT session AS "session!", position AS "position!" FROM training_set WHERE id = ?1"#,
        id
    )
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        Err(format!("No set with id '{id}'."))?;
        return Ok(());
    };
    sqlx::query!("DELETE FROM training_set WHERE id = ?1", id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!(
        "UPDATE training_set SET position = position - 1 WHERE session = ?1 AND position > ?2",
        row.session,
        row.position
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn edit_exercise(
    db: &Db,
    id: i64,
    name: &str,
    note: &str,
    video_url: Option<&str>,
) -> Res<()> {
    if name.trim().is_empty() {
        Err("An exercise needs a name.".to_string())?;
    }
    let url = video_url.map(str::trim).filter(|u| !u.is_empty());
    if let Some(u) = url {
        if !(u.starts_with("https://") || u.starts_with("http://")) {
            Err(format!(
                "'{u}' is not a link: it must start with http:// or https://."
            ))?;
        }
    }
    sqlx::query!(
        "UPDATE exercise SET name = ?1, note = ?2, video_url = ?3 WHERE id = ?4",
        name.trim(),
        note,
        url,
        id
    )
    .execute(&db.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            crate::util::Err::Str(format!("An exercise called '{name}' already exists."))
        } else {
            crate::util::Err::from(e)
        }
    })?;
    Ok(())
}

/// Set one exercise's links, replacing what was there.
pub async fn set_groups(db: &Db, exercise: i64, groups: Vec<(i64, f64)>) -> Res<()> {
    let mut tx = db.pool.begin().await?;
    sqlx::query!(
        "DELETE FROM exercise_muscle_group WHERE exercise = ?1",
        exercise
    )
    .execute(&mut *tx)
    .await?;
    for (g, amount) in groups {
        if amount <= 0.0 {
            Err("A muscle group's amount must be greater than zero.".to_string())?;
        }
        sqlx::query!(
            "INSERT INTO exercise_muscle_group (exercise, muscle_group, amount) VALUES (?1, ?2, ?3)",
            exercise,
            g,
            amount
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            if e.to_string().contains("FOREIGN KEY") {
                crate::util::Err::Str(format!("No muscle group with id '{g}'."))
            } else {
                crate::util::Err::from(e)
            }
        })?;
    }
    tx.commit().await?;
    Ok(())
}

/// Rename an exercise.
pub async fn rename_exercise(db: &Db, id: i64, name: &str) -> Res<()> {
    if name.trim().is_empty() {
        Err("An exercise needs a name.".to_string())?;
    }
    sqlx::query!(
        "UPDATE exercise SET name = ?1 WHERE id = ?2",
        name.trim(),
        id
    )
    .execute(&db.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            crate::util::Err::Str(format!("An exercise called '{name}' already exists."))
        } else {
            crate::util::Err::from(e)
        }
    })?;
    Ok(())
}

/// Merge `from` into `to`: every set and list item is re-pointed, the links are
/// unioned with the survivor's amount winning, and the merged-away row is
/// deleted. All of it in one transaction, so a half-merged exercise cannot
/// exist.
pub async fn merge_exercises(db: &Db, from: i64, to: i64) -> Res<()> {
    if from == to {
        Err("An exercise cannot be merged into itself.".to_string())?;
    }
    let mut tx = db.pool.begin().await?;
    sqlx::query!(
        "UPDATE training_set SET exercise = ?1 WHERE exercise = ?2",
        to,
        from
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE OR REPLACE session_list_item SET exercise = ?1 WHERE exercise = ?2",
        to,
        from
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "UPDATE OR REPLACE exercise_list_item SET exercise = ?1 WHERE exercise = ?2",
        to,
        from
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        r#"INSERT OR IGNORE INTO exercise_muscle_group (exercise, muscle_group, amount)
           SELECT ?1, muscle_group, amount FROM exercise_muscle_group WHERE exercise = ?2"#,
        to,
        from
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM exercise_muscle_group WHERE exercise = ?1",
        from
    )
    .execute(&mut *tx)
    .await?;
    let n = sqlx::query!("DELETE FROM exercise WHERE id = ?1", from)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    if n == 0 {
        Err(format!("No exercise with id '{from}'."))?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn add_place(db: &Db, name: &str, note: &str) -> Res<i64> {
    if name.trim().is_empty() {
        Err("A place needs a name.".to_string())?;
    }
    Ok(sqlx::query_scalar!(
        r#"INSERT INTO place (name, note) VALUES (?1, ?2) RETURNING id AS "id!""#,
        name.trim(),
        note
    )
    .fetch_one(&db.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            crate::util::Err::Str(format!("A place called '{name}' already exists."))
        } else {
            crate::util::Err::from(e)
        }
    })?)
}

pub async fn rename_place(db: &Db, id: i64, name: &str, note: &str) -> Res<()> {
    if name.trim().is_empty() {
        Err("A place needs a name.".to_string())?;
    }
    sqlx::query!(
        "UPDATE place SET name = ?1, note = ?2 WHERE id = ?3",
        name.trim(),
        note,
        id
    )
    .execute(&db.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            crate::util::Err::Str(format!("A place called '{name}' already exists."))
        } else {
            crate::util::Err::from(e)
        }
    })?;
    Ok(())
}

pub async fn delete_place(db: &Db, id: i64) -> Res<()> {
    let used: i64 = sqlx::query_scalar!("SELECT count(*) FROM session WHERE place = ?1", id)
        .fetch_one(&db.pool)
        .await?;
    if used > 0 {
        Err(format!("{used} session(s) still use this place."))?;
    }
    sqlx::query!("DELETE FROM place WHERE id = ?1", id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn add_group(db: &Db, name: &str, note: &str) -> Res<i64> {
    if name.trim().is_empty() {
        Err("A muscle group needs a name.".to_string())?;
    }
    Ok(sqlx::query_scalar!(
        r#"INSERT INTO muscle_group (name, note) VALUES (?1, ?2) RETURNING id AS "id!""#,
        name.trim(),
        note
    )
    .fetch_one(&db.pool)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            crate::util::Err::Str(format!("A muscle group called '{name}' already exists."))
        } else {
            crate::util::Err::from(e)
        }
    })?)
}

pub async fn delete_group(db: &Db, id: i64) -> Res<()> {
    let used: i64 = sqlx::query_scalar!(
        "SELECT count(*) FROM exercise_muscle_group WHERE muscle_group = ?1",
        id
    )
    .fetch_one(&db.pool)
    .await?;
    if used > 0 {
        Err(format!(
            "{used} exercise link(s) still use this muscle group; remove them first."
        ))?;
    }
    sqlx::query!("DELETE FROM muscle_group WHERE id = ?1", id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn add_preset(db: &Db, name: &str, items: Vec<(i64, f64)>) -> Res<i64> {
    if name.trim().is_empty() {
        Err("A preset needs a name.".to_string())?;
    }
    if items.is_empty() {
        Err("A preset needs at least one food.".to_string())?;
    }
    let mut tx = db.pool.begin().await?;
    let id = sqlx::query_scalar!(
        r#"INSERT INTO meal_preset (name) VALUES (?1) RETURNING id AS "id!""#,
        name.trim()
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            crate::util::Err::Str(format!("A preset called '{name}' already exists."))
        } else {
            crate::util::Err::from(e)
        }
    })?;
    for (food, amount) in items {
        sqlx::query!(
            "INSERT INTO meal_preset_item (preset, food, amount) VALUES (?1, ?2, ?3)",
            id,
            food,
            amount
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(id)
}

/// Mark a preset as used.
///
/// A preset no longer records its foods by itself: it loads them into the meal
/// being entered, and nothing is written until that is saved. Loading is still
/// what "used" means, so the list keeps sorting by the preset the reader
/// reaches for first; the stamp writes no meal row.
pub async fn touch_preset(db: &Db, preset: i64) -> Res<()> {
    let at = Local::now().to_rfc3339();
    sqlx::query!(
        "UPDATE meal_preset SET last_used = ?1 WHERE id = ?2",
        at,
        preset
    )
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn delete_preset(db: &Db, id: i64) -> Res<()> {
    sqlx::query!("DELETE FROM meal_preset WHERE id = ?1", id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

/// A gym preset: a named, ordered list of exercises to work through.
pub async fn add_exercise_preset(db: &Db, name: &str, items: Vec<i64>) -> Res<i64> {
    let name = name.trim();
    if name.is_empty() {
        Err("A preset needs a name.".to_string())?;
    }
    if items.is_empty() {
        Err("A preset needs at least one exercise.".to_string())?;
    }
    let mut tx = db.pool.begin().await?;
    let id = sqlx::query_scalar!(
        r#"INSERT INTO exercise_list (name) VALUES (?1) RETURNING id AS "id!""#,
        name
    )
    .fetch_one(&mut *tx)
    .await?;
    for (position, exercise) in items.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO exercise_list_item (list, exercise, position) VALUES (?1, ?2, ?3)",
            id,
            exercise,
            position as i64
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(id)
}

/// Rewrite a preset: its name and its list, in one transaction.
///
/// Finished sessions keep the checklist they recorded — they store the
/// exercises, not the preset — so rewriting a preset never rewrites history.
pub async fn edit_exercise_preset(db: &Db, id: i64, name: &str, items: Vec<i64>) -> Res<()> {
    let name = name.trim();
    if name.is_empty() {
        Err("A preset needs a name.".to_string())?;
    }
    if items.is_empty() {
        Err("A preset needs at least one exercise.".to_string())?;
    }
    let mut tx = db.pool.begin().await?;
    sqlx::query!("UPDATE exercise_list SET name = ?1 WHERE id = ?2", name, id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM exercise_list_item WHERE list = ?1", id)
        .execute(&mut *tx)
        .await?;
    for (position, exercise) in items.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO exercise_list_item (list, exercise, position) VALUES (?1, ?2, ?3)",
            id,
            exercise,
            position as i64
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn delete_exercise_preset(db: &Db, id: i64) -> Res<()> {
    sqlx::query!("DELETE FROM exercise_list WHERE id = ?1", id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn set_setting(db: &Db, key: &str, value: &str) -> Res<()> {
    let known = [
        "deload_weeks",
        "trend_window_days",
        "trend_min_weighins",
        "maintenance_min_days",
    ];
    if !known.contains(&key) {
        Err(format!("'{key}' is not a setting."))?;
    }
    value
        .parse::<i64>()
        .map_err(|_| format!("'{value}' is not a whole number."))?;
    sqlx::query!(
        "INSERT INTO setting (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        key,
        value
    )
    .execute(&db.pool)
    .await?;
    Ok(())
}

/// Every preset with its parts and totals, newest-used first.
pub async fn presets(db: &Db) -> Res<Vec<crate::web::dto::PresetRow>> {
    use crate::web::dto::{PresetPart, PresetRow};
    let rows = sqlx::query!(
        r#"
SELECT p.id AS "id!", p.name AS "name!", p.last_used,
       COALESCE(SUM(i.amount * f.calories), 0.0) AS "kcal!: f64",
       COALESCE(SUM(i.amount * COALESCE(f.protein, 0.0)), 0.0) AS "protein!: f64",
       COALESCE(SUM(i.amount * COALESCE(f.fat, 0.0)), 0.0) AS "fat!: f64",
       COALESCE(SUM(i.amount * COALESCE(f.carbohydrate, 0.0)), 0.0) AS "carbohydrate!: f64",
       (SELECT count(*) FROM meal_preset_item w WHERE w.preset = p.id) AS "items!: i64"
FROM meal_preset p
JOIN meal_preset_item i ON i.preset = p.id
JOIN food f ON f.id = i.food
GROUP BY p.id
ORDER BY COALESCE(julianday(p.last_used), 0.0) DESC, p.name
"#
    )
    .fetch_all(&db.pool)
    .await?;
    let parts = sqlx::query!(
        r#"SELECT i.preset AS "preset!", i.food AS "food!", i.amount AS "amount!",
                  f.name AS "name!"
           FROM meal_preset_item i JOIN food f ON f.id = i.food
           ORDER BY f.name"#
    )
    .fetch_all(&db.pool)
    .await?;
    let mut by_preset: std::collections::HashMap<i64, Vec<PresetPart>> = Default::default();
    for p in parts {
        by_preset.entry(p.preset).or_default().push(PresetPart {
            food: p.food,
            name: p.name,
            amount: p.amount,
        });
    }
    Ok(rows
        .into_iter()
        .map(|r| PresetRow {
            parts: by_preset.remove(&r.id).unwrap_or_default(),
            id: r.id,
            name: r.name,
            items: r.items as usize,
            per_unit: Totals {
                kcal: r.kcal,
                protein: r.protein,
                fat: r.fat,
                carbohydrate: r.carbohydrate,
            },
        })
        .collect())
}

/// The local day a `date` string falls in, for the "today" default.
pub fn today() -> String {
    let (s, _) = day_bounds(Local::now());
    s.date_naive().to_string()
}

/// The day after `date`, to show a sibling row in tests and debugging.
pub fn next_day(date: &str) -> Res<String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|e| format!("'{date}' is not a date because '{e}'."))?;
    Ok((d + Duration::days(1)).to_string())
}

/// Parse an optional number a form left blank.
pub fn opt_num(s: &str) -> Res<Option<f64>> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    s.parse::<f64>()
        .map(Some)
        .map_err(|_| crate::util::Err::Str(format!("'{s}' is not a number.")))
}

pub fn num(s: &str, field: &str) -> Res<f64> {
    let t = s.trim();
    t.parse::<f64>()
        .map_err(|_| crate::util::Err::Str(format!("'{field}' is '{t}', which is not a number.")))
}

pub type At = DateTime<Local>;
