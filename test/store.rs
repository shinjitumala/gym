mod support;

use support::fresh;

#[tokio::test]
async fn an_unknown_bodyfat_is_null_and_a_measured_zero_is_zero() {
    let (_s, db) = fresh("weight").await;
    db.add_weight("2026-01-01T00:00:00+00:00", 70.0, None, "unknown")
        .await
        .unwrap();
    db.add_weight("2026-01-02T00:00:00+00:00", 70.0, Some(0.0), "zero")
        .await
        .unwrap();
    let rows = sqlx::query!("SELECT bodyfat FROM weight ORDER BY id")
        .fetch_all(&db.pool)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].bodyfat, None);
    assert_eq!(rows[1].bodyfat, Some(0.0));
}

#[tokio::test]
async fn an_unknown_food_number_is_not_stored_as_zero() {
    let (_s, db) = fresh("foodnull").await;
    let id = db
        .new_food("mystery", 200.0, None, None, None, "")
        .await
        .unwrap();
    let row = sqlx::query!(
        "SELECT calories, protein, fat, carbohydrate FROM food WHERE id = ?1",
        id
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(row.calories, 200.0);
    assert_eq!(row.protein, None);
    assert_eq!(row.fat, None);
    assert_eq!(row.carbohydrate, None);
}

#[tokio::test]
async fn the_day_window_is_half_open_and_bound_by_the_arguments() {
    let (_s, db) = fresh("window").await;
    let f = db
        .new_food("unit", 100.0, Some(0.0), Some(0.0), Some(0.0), "")
        .await
        .unwrap();
    db.new_meal("2026-03-01T00:00:00+00:00", f, 1.0, "start")
        .await
        .unwrap();
    db.new_meal("2026-03-01T23:59:59+00:00", f, 1.0, "end-1")
        .await
        .unwrap();
    db.new_meal("2026-03-02T00:00:00+00:00", f, 1.0, "next")
        .await
        .unwrap();
    let t = db
        .day_totals("2026-03-01T00:00:00+00:00", "2026-03-02T00:00:00+00:00")
        .await
        .unwrap();
    assert_eq!(t.calories, 200.0, "the start is included, the end is not");
}

#[tokio::test]
async fn the_window_follows_the_instant_not_the_offsets_written() {
    let (_s, db) = fresh("offsets").await;
    let f = db
        .new_food("unit", 100.0, Some(0.0), Some(0.0), Some(0.0), "")
        .await
        .unwrap();
    db.new_meal(
        "2026-03-01T09:00:00+09:00",
        f,
        1.0,
        "same instant as 00:00Z",
    )
    .await
    .unwrap();
    let t = db
        .day_totals("2026-03-01T00:00:00+00:00", "2026-03-02T00:00:00+00:00")
        .await
        .unwrap();
    assert_eq!(t.calories, 100.0);
}

#[tokio::test]
async fn a_meal_pointing_at_a_missing_food_is_refused_by_the_schema() {
    let (_s, db) = fresh("fk").await;
    let e = db.new_meal("2026-03-01T00:00:00+00:00", 404, 1.0, "").await;
    assert!(e.is_err(), "a foreign key must be enforced");
}

#[tokio::test]
async fn history_is_the_last_three_sessions_newest_first() {
    let (_s, db) = fresh("history").await;
    let place =
        sqlx::query_scalar!(r#"INSERT INTO place (name) VALUES ('p') RETURNING id AS "id!""#)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let ex = db.get_or_create_exercise("bench press").await.unwrap();

    for (i, day) in ["2026-01-01", "2026-02-01", "2026-03-01", "2026-04-01"]
        .iter()
        .enumerate()
    {
        let sid = db
            .new_session(place, &format!("{day}T10:00:00+00:00"), "")
            .await
            .unwrap();
        for rep in 1..=2 {
            db.new_set(
                sid,
                ex.id,
                &format!("{day}T10:0{rep}:00+00:00"),
                60.0,
                rep as f64,
                &format!("set {i}-{rep}"),
            )
            .await
            .unwrap();
        }
    }

    let h = db.history(place, ex.id, 3).await.unwrap();
    assert_eq!(h.len(), 6, "three sessions of two sets");
    assert!(h[0].date.starts_with("2026-04-01"), "{}", h[0].date);
    assert!(h[5].date.starts_with("2026-02-01"), "{}", h[5].date);
    assert_eq!(h[0].rep, 2.0, "within a session, newest set first");

    let other =
        sqlx::query_scalar!(r#"INSERT INTO place (name) VALUES ('q') RETURNING id AS "id!""#)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert!(
        db.history(other, ex.id, 3).await.unwrap().is_empty(),
        "history is per place"
    );
}

#[tokio::test]
async fn a_set_position_is_unique_per_session() {
    let (_s, db) = fresh("position").await;
    let place =
        sqlx::query_scalar!(r#"INSERT INTO place (name) VALUES ('p') RETURNING id AS "id!""#)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let ex = db.get_or_create_exercise("squat").await.unwrap();
    let sid = db
        .new_session(place, "2026-01-01T00:00:00+00:00", "")
        .await
        .unwrap();
    for _ in 0..3 {
        db.new_set(sid, ex.id, "2026-01-01T00:00:00+00:00", 100.0, 5.0, "")
            .await
            .unwrap();
    }
    let positions = sqlx::query_scalar!(
        r#"SELECT position AS "position!" FROM training_set WHERE session = ?1 ORDER BY id"#,
        sid
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    assert_eq!(positions, vec![0, 1, 2]);
}

#[tokio::test]
async fn get_or_create_exercise_is_idempotent() {
    let (_s, db) = fresh("exercise").await;
    let a = db.get_or_create_exercise("bench press").await.unwrap();
    let b = db.get_or_create_exercise("bench press").await.unwrap();
    assert_eq!(a.id, b.id);
    let n = sqlx::query_scalar!("SELECT count(*) FROM exercise")
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn the_settings_are_seeded_with_the_documented_defaults() {
    let (_s, db) = fresh("settings").await;
    let rows = sqlx::query!(r#"SELECT key AS "key!", value FROM setting ORDER BY key"#)
        .fetch_all(&db.pool)
        .await
        .unwrap();
    let pairs: Vec<(String, String)> = rows.into_iter().map(|r| (r.key, r.value)).collect();
    assert_eq!(
        pairs,
        vec![
            ("deload_weeks".to_string(), "6".to_string()),
            ("maintenance_min_days".to_string(), "10".to_string()),
            ("trend_min_weighins".to_string(), "14".to_string()),
            ("trend_window_days".to_string(), "21".to_string()),
        ]
    );
}

#[tokio::test]
async fn the_food_command_window_is_this_local_day() {
    use chrono::{Duration, Local};
    let (_s, db) = fresh("localday").await;
    let f = db
        .new_food("unit", 100.0, Some(10.0), Some(5.0), Some(2.0), "")
        .await
        .unwrap();
    let now = Local::now();
    db.new_meal(&now.to_rfc3339(), f, 2.0, "today")
        .await
        .unwrap();
    db.new_meal(
        &(now - Duration::days(1)).to_rfc3339(),
        f,
        10.0,
        "yesterday",
    )
    .await
    .unwrap();

    let (s, e) = gym::util::day_bounds(now);
    let t = db
        .day_totals(&s.to_rfc3339(), &e.to_rfc3339())
        .await
        .unwrap();
    assert_eq!(t.calories, 200.0, "only the meal eaten today");
    assert_eq!(t.protein, 20.0);
    assert_eq!(t.fat, 10.0);
    assert_eq!(t.carbohydrate, 4.0);
}

#[test]
fn the_local_day_starts_at_midnight_and_lasts_a_day() {
    use chrono::{Local, TimeZone, Timelike};
    let now = Local.with_ymd_and_hms(2026, 5, 13, 21, 33, 0).unwrap();
    let (s, e) = gym::util::day_bounds(now);
    assert_eq!((s.hour(), s.minute(), s.second()), (0, 0, 0));
    assert_eq!(s.date_naive(), now.date_naive());
    assert_eq!(e - s, chrono::Duration::days(1));
}

/// The web pages' window must use local midnight, not UTC midnight: a meal
/// eaten at 06:00 JST is 21:00 UTC the day before, and a bare `YYYY-MM-DD`
/// bound would drop it.
#[cfg(feature = "ssr")]
#[tokio::test]
async fn the_web_day_window_is_local_midnight_to_local_midnight() {
    use chrono::{Duration, Local, TimeZone};
    let (_s, db) = fresh("webday").await;
    let f = db
        .new_food("unit", 100.0, Some(1.0), Some(1.0), Some(1.0), "")
        .await
        .unwrap();
    let today = Local::now().date_naive();
    let early = Local
        .from_local_datetime(&today.and_hms_opt(6, 0, 0).unwrap())
        .earliest()
        .unwrap();
    db.new_meal(&early.to_rfc3339(), f, 1.0, "early local morning")
        .await
        .unwrap();
    let yesterday = today - Duration::days(1);
    let late_yesterday = Local
        .from_local_datetime(&yesterday.and_hms_opt(23, 0, 0).unwrap())
        .earliest()
        .unwrap();
    db.new_meal(&late_yesterday.to_rfc3339(), f, 10.0, "last night")
        .await
        .unwrap();

    let d = gym::web::data::food_daily(&db).await.unwrap();
    assert_eq!(
        d.totals.kcal, 100.0,
        "only the meal eaten after local midnight counts: {:?}",
        d.meals
    );
    assert_eq!(d.meals.len(), 1);
}

/// A session the CLI writes is finished the moment it is written; only the web's
/// live session stays open. If the two were the same, a CLI session would block
/// the next one started from the browser.
#[tokio::test]
async fn a_cli_session_is_finished_when_it_is_written() {
    let (_s, db) = fresh("ended").await;
    let place =
        sqlx::query_scalar!(r#"INSERT INTO place (name) VALUES ('home') RETURNING id AS "id!""#)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    let id = db
        .new_session(place, "2026-03-01T10:00:00+00:00", "cli")
        .await
        .unwrap();
    let ended = sqlx::query_scalar!(r#"SELECT ended FROM session WHERE id = ?1"#, id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(
        ended.as_deref(),
        Some("2026-03-01T10:00:00+00:00"),
        "a CLI session must not look like one still being recorded"
    );
}

/// The recommendation compares effort before load, and a smaller rank means
/// closer to failure. That ordering is the migration's, so it is asserted here
/// rather than assumed: `fail` must outrank `form fail`, which outranks `hard`.
#[tokio::test]
async fn the_effort_levels_rank_from_none_left_to_easy() {
    let (_s, db) = fresh("effort").await;
    let ranks = sqlx::query!(r#"SELECT name AS "name!", sort AS "sort!" FROM effort"#)
        .fetch_all(&db.pool)
        .await
        .unwrap();
    let rank = |name: &str| {
        ranks
            .iter()
            .find(|r| r.name == name)
            .map(|r| r.sort)
            .unwrap_or_else(|| panic!("no effort called {name}"))
    };
    let (fail, form_fail, hard) = (rank("fail"), rank("form fail"), rank("hard"));
    assert!(
        fail < form_fail && form_fail < hard,
        "effort ranks must increase as the set gets easier: fail {fail}, form fail {form_fail}, hard {hard}"
    );
}
