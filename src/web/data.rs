//! Queries behind the pages, and the DTOs they fill.
//!
//! Server only: these run in the binary, never in the browser. Row timestamps
//! are stored as RFC3339 with whatever offset they were written with, so every
//! day boundary is computed here, in the machine's local zone, rather than in
//! SQL — one place for the rule, and the same rule the CLI's `gym food` uses.

use crate::db::Db;
use crate::stats::{self, DayIntake, Goal, Point};
use crate::util::{to_one_rep_max, Res};
use crate::web::dto::*;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate};
use std::collections::{BTreeMap, HashMap};

pub struct Settings {
    pub deload_weeks: i64,
    pub trend_window_days: i64,
    pub trend_min_weighins: usize,
    pub maintenance_min_days: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            deload_weeks: 6,
            trend_window_days: 21,
            trend_min_weighins: 14,
            maintenance_min_days: 10,
        }
    }
}

fn local_date(s: &str) -> Option<NaiveDate> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Local).date_naive())
}

fn local_time(s: &str) -> String {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Local).format("%H:%M").to_string())
        .unwrap_or_default()
}

fn iso(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

fn days(d: NaiveDate) -> i64 {
    d.num_days_from_ce() as i64
}

/// Midnight at the start of `d`, **in the machine's zone**, as an RFC3339
/// instant.
///
/// The window comparisons run in SQL against `julianday`, which reads a bare
/// `YYYY-MM-DD` as UTC midnight. Passing one would silently shift the day by
/// the zone's offset — nine hours here — and drop every row written after local
/// midnight but before UTC midnight. An instant has no such ambiguity.
fn midnight(d: NaiveDate) -> String {
    use chrono::TimeZone;
    Local
        .from_local_datetime(&d.and_hms_opt(0, 0, 0).expect("midnight exists"))
        .earliest()
        .map(|t| t.to_rfc3339())
        .unwrap_or_else(|| d.to_string())
}

pub async fn settings(db: &Db) -> Res<Settings> {
    let rows = sqlx::query!(r#"SELECT key AS "key!", value FROM setting"#)
        .fetch_all(&db.pool)
        .await?;
    let mut s = Settings::default();
    for r in rows {
        let v = r.value.parse::<i64>().unwrap_or(0);
        match r.key.as_str() {
            "deload_weeks" => s.deload_weeks = v,
            "trend_window_days" => s.trend_window_days = v.max(3),
            "trend_min_weighins" => s.trend_min_weighins = v.max(3) as usize,
            "maintenance_min_days" => s.maintenance_min_days = v.max(1) as usize,
            _ => {}
        }
    }
    Ok(s)
}

pub async fn info(db: &Db, remote: &str) -> Res<ServerInfo> {
    Ok(ServerInfo {
        db: db.path.clone(),
        remote: remote.to_owned(),
        sessions: sqlx::query_scalar!("SELECT count(*) FROM session")
            .fetch_one(&db.pool)
            .await?,
        sets: sqlx::query_scalar!("SELECT count(*) FROM training_set")
            .fetch_one(&db.pool)
            .await?,
        meals: sqlx::query_scalar!("SELECT count(*) FROM meal")
            .fetch_one(&db.pool)
            .await?,
        weights: sqlx::query_scalar!("SELECT count(*) FROM weight")
            .fetch_one(&db.pool)
            .await?,
    })
}

pub async fn phase(db: &Db) -> Res<Option<PhaseRow>> {
    let now = Local::now().to_rfc3339();
    let r = sqlx::query!(
        r#"SELECT kind AS "kind!", start AS "start!", target_rate AS "target_rate!",
                  target_rep AS "target_rep!", target_weight
           FROM phase WHERE julianday(start) <= julianday(?1)
           ORDER BY julianday(start) DESC LIMIT 1"#,
        now
    )
    .fetch_optional(&db.pool)
    .await?;
    Ok(r.map(|r| PhaseRow {
        kind: r.kind,
        start: local_date(&r.start).map(iso).unwrap_or(r.start),
        target_rate: r.target_rate,
        target_rep: r.target_rep,
        target_weight: r.target_weight,
    }))
}

fn goal_of(p: &Option<PhaseRow>) -> Option<Goal> {
    p.as_ref().map(|p| match p.kind.as_str() {
        "cut" => Goal::Cut,
        _ => Goal::Bulk(p.target_rate),
    })
}

fn goal_label(p: &Option<PhaseRow>) -> Option<String> {
    p.as_ref().map(|p| match p.kind.as_str() {
        "cut" => "cut: at or below 0 kg/week".to_string(),
        _ => format!("bulk: {:.2} kg/week", p.target_rate / 30.4375 * 7.0),
    })
}

fn trend_view(
    weights: &[(NaiveDate, f64)],
    s: &Settings,
    phase: &Option<PhaseRow>,
    today: NaiveDate,
) -> TrendView {
    let from = today - Duration::days(s.trend_window_days);
    let mut window: Vec<Point> = weights.iter().filter(|(d, _)| *d > from).copied().collect();
    window.sort_by_key(|(d, _)| *d);
    let t = stats::trend(&window, s.trend_min_weighins);
    let goal = goal_of(phase);
    let fit = t.shown();
    let verdict = match (fit, goal) {
        (Some(f), Some(g)) => Some(match stats::verdict(f, g) {
            stats::Verdict::On => Verdict::On,
            stats::Verdict::Off => Verdict::Off,
            stats::Verdict::Uncertain => Verdict::Uncertain,
        }),
        _ => None,
    };
    let (low, high) = match goal {
        Some(g) => {
            let (l, h) = stats::goal_range(g);
            (l.is_finite().then_some(l), h)
        }
        None => (None, None),
    };
    TrendView {
        points: t
            .points
            .iter()
            .map(|(d, kg)| PointRow {
                day: days(*d),
                kg: *kg,
            })
            .collect(),
        per_week: fit.map(|f| f.per_week()),
        band: fit.map(|f| f.band_per_week()),
        have: t.have,
        need: t.need,
        verdict,
        goal: goal_label(phase),
        goal_low: low,
        goal_high: high,
    }
}

/// Weigh-ins in a window, oldest first, with the row the edit tab shows.
async fn weights_between(
    db: &Db,
    from: NaiveDate,
    to: NaiveDate,
) -> Res<Vec<(NaiveDate, f64, WeightRow)>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", date, kg AS "kg!", bodyfat, note
           FROM weight
           WHERE julianday(date) >= julianday(?1) AND julianday(date) < julianday(?2)
           ORDER BY julianday(date)"#,
        midnight(from),
        midnight(to + Duration::days(1))
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| {
            let d = local_date(&r.date)?;
            Some((
                d,
                r.kg,
                WeightRow {
                    id: r.id,
                    date: iso(d),
                    kg: r.kg,
                    bodyfat: r.bodyfat,
                    note: r.note,
                },
            ))
        })
        .collect())
}

/// Meals in a window, with the food they point at and the amount applied.
async fn meals_between(db: &Db, from: NaiveDate, to: NaiveDate) -> Res<Vec<MealRow>> {
    let rows = sqlx::query!(
        r#"
SELECT m.id AS "id!", m.date, f.name AS "food!", m.amount AS "amount!",
       m.note AS "note!", f.calories AS "kcal!",
       COALESCE(f.protein, 0.0) AS "protein!: f64",
       COALESCE(f.fat, 0.0) AS "fat!: f64",
       COALESCE(f.carbohydrate, 0.0) AS "carbohydrate!: f64"
FROM meal m JOIN food f ON f.id = m.food
WHERE julianday(m.date) >= julianday(?1) AND julianday(m.date) < julianday(?2)
ORDER BY julianday(m.date)
"#,
        midnight(from),
        midnight(to + Duration::days(1))
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| MealRow {
            id: r.id,
            date: local_date(&r.date).map(iso).unwrap_or_default(),
            time: local_time(&r.date),
            food: r.food,
            amount: r.amount,
            kcal: r.amount * r.kcal,
            protein: r.amount * r.protein,
            fat: r.amount * r.fat,
            carbohydrate: r.amount * r.carbohydrate,
            note: r.note,
        })
        .collect())
}

async fn foods(db: &Db) -> Res<Vec<FoodRow>> {
    let rows = sqlx::query!(
        r#"
SELECT f.id AS "id!", f.name AS "name!", f.calories AS "kcal!",
       COALESCE(f.protein, 0.0) AS "protein!: f64",
       COALESCE(f.fat, 0.0) AS "fat!: f64",
       COALESCE(f.carbohydrate, 0.0) AS "carbohydrate!: f64",
       f.note AS "note!",
       (SELECT count(*) FROM meal m WHERE m.food = f.id) AS "eaten!"
FROM food f ORDER BY f.id
"#
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| FoodRow {
            id: r.id,
            name: r.name,
            per_unit: Totals {
                kcal: r.kcal,
                protein: r.protein,
                fat: r.fat,
                carbohydrate: r.carbohydrate,
            },
            note: r.note,
            eaten: r.eaten,
        })
        .collect())
}

fn sum<'a>(meals: impl Iterator<Item = &'a MealRow>) -> Totals {
    meals.fold(Totals::default(), |mut t, m| {
        t.kcal += m.kcal;
        t.protein += m.protein;
        t.fat += m.fat;
        t.carbohydrate += m.carbohydrate;
        t
    })
}

fn day_map(meals: &[MealRow]) -> BTreeMap<NaiveDate, Vec<&MealRow>> {
    let mut by_day: BTreeMap<NaiveDate, Vec<&MealRow>> = BTreeMap::new();
    for m in meals {
        if let Ok(d) = NaiveDate::parse_from_str(&m.date, "%Y-%m-%d") {
            by_day.entry(d).or_default().push(m);
        }
    }
    by_day
}

pub async fn food_daily(db: &Db) -> Res<FoodDaily> {
    let today = Local::now().date_naive();
    let s = settings(db).await?;
    let p = phase(db).await?;
    let meals = meals_between(db, today, today).await?;
    let span = Duration::days(s.trend_window_days.max(30));
    let window_meals = meals_between(db, today - span, today).await?;
    let weights = weights_between(db, today - Duration::days(365), today).await?;
    let flat: Vec<(NaiveDate, f64)> = weights.iter().map(|(d, kg, _)| (*d, *kg)).collect();
    let trend = trend_view(&flat, &s, &p, today);

    let by_day = day_map(&window_meals);
    let intake: Vec<DayIntake> = by_day
        .iter()
        .map(|(d, ms)| DayIntake::new(*d, sum(ms.iter().copied()).kcal))
        .collect();
    let logged_days = intake.iter().filter(|d| d.logged).count();

    let mut w: Vec<Point> = flat
        .iter()
        .filter(|(d, _)| *d > today - span)
        .copied()
        .collect();
    w.sort_by_key(|(d, _)| *d);
    let fit = stats::trend(&w, s.trend_min_weighins).shown();
    let m = fit.and_then(|f| stats::maintenance(&intake, f, s.maintenance_min_days));
    let outliers = m
        .map(|m| {
            stats::outliers(&intake, m)
                .into_iter()
                .map(|o| OutlierRow {
                    day: days(o.date),
                    kcal: o.kcal,
                    expected: o.expected,
                    delta: o.delta,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(FoodDaily {
        today: iso(today),
        totals: sum(meals.iter()),
        meals,
        foods: foods(db).await?,
        last_weight: weights.last().map(|(_, _, w)| w.clone()),
        trend,
        maintenance: m.map(|m| MaintenanceView {
            kcal: m.kcal,
            days: m.days,
            intake: m.intake,
        }),
        logged_days,
        need_days: s.maintenance_min_days,
        outliers,
        phase: p,
        presets: crate::web::actions::presets(db)
            .await?
            .into_iter()
            .map(|(id, name, per_unit, items)| PresetRow {
                id,
                name,
                items,
                per_unit,
            })
            .collect(),
    })
}

pub async fn food_edit(db: &Db) -> Res<FoodEdit> {
    let today = Local::now().date_naive();
    let mut meals = meals_between(db, today - Duration::days(120), today).await?;
    meals.reverse();
    let mut weights: Vec<WeightRow> = weights_between(db, today - Duration::days(120), today)
        .await?
        .into_iter()
        .map(|(_, _, w)| w)
        .collect();
    weights.reverse();
    Ok(FoodEdit {
        meals,
        weights,
        foods: foods(db).await?,
    })
}

pub async fn food_analysis(db: &Db) -> Res<FoodAnalysis> {
    let today = Local::now().date_naive();
    let span = 90i64;
    let s = settings(db).await?;
    let p = phase(db).await?;
    let meals = meals_between(db, today - Duration::days(span), today).await?;
    let weights = weights_between(db, today - Duration::days(365), today).await?;
    let flat: Vec<(NaiveDate, f64)> = weights.iter().map(|(d, kg, _)| (*d, *kg)).collect();

    let mut out = Vec::new();
    for (d, ms) in day_map(&meals) {
        let t = sum(ms.iter().copied());
        let mut per_food: HashMap<&str, f64> = HashMap::new();
        for m in &ms {
            *per_food.entry(m.food.as_str()).or_default() += m.kcal;
        }
        let mut split: Vec<SplitRow> = per_food
            .into_iter()
            .map(|(food, kcal)| SplitRow {
                food: food.to_owned(),
                kcal,
            })
            .collect();
        split.sort_by(|a, b| {
            b.kcal
                .partial_cmp(&a.kcal)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        split.truncate(6);
        out.push(DayRow {
            date: iso(d),
            logged: t.kcal >= stats::LOGGED_DAY_KCAL,
            totals: t,
            split,
        });
    }

    let all = foods(db).await?;
    let mut favourite = all.clone();
    favourite.sort_by(|a, b| b.eaten.cmp(&a.eaten).then(a.name.cmp(&b.name)));
    favourite.retain(|f| f.eaten > 0);
    let mut by_calories = all;
    by_calories.sort_by(|a, b| {
        (b.per_unit.kcal * b.eaten as f64)
            .partial_cmp(&(a.per_unit.kcal * a.eaten as f64))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    by_calories.retain(|f| f.eaten > 0);

    Ok(FoodAnalysis {
        logged_days: out.iter().filter(|d| d.logged).count(),
        days: out,
        trend: trend_view(&flat, &s, &p, today),
        favourite,
        by_calories,
        weights: weights.into_iter().map(|(_, _, w)| w).collect(),
        span: span as usize,
    })
}

pub struct RawSet {
    pub id: i64,
    pub session: i64,
    pub exercise: i64,
    pub position: i64,
    pub load: f64,
    pub rep: f64,
    pub note: String,
    pub superset: Option<i64>,
    pub inferred: bool,
}

pub async fn all_sets(db: &Db) -> Res<Vec<RawSet>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", session AS "session!", exercise AS "exercise!",
                  position AS "position!", load AS "load!", rep AS "rep!",
                  note AS "note!", superset, inferred AS "inferred!"
           FROM training_set ORDER BY session, position"#
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| RawSet {
            id: r.id,
            session: r.session,
            exercise: r.exercise,
            position: r.position,
            load: r.load,
            rep: r.rep,
            note: r.note,
            superset: r.superset,
            inferred: r.inferred != 0,
        })
        .collect())
}

pub async fn exercise_index(db: &Db) -> Res<(HashMap<i64, String>, Vec<ExerciseRow>)> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", name AS "name!", note AS "note!", video_url FROM exercise ORDER BY name"#
    )
    .fetch_all(&db.pool)
    .await?;
    let groups = sqlx::query!(
        r#"SELECT emg.exercise AS "exercise!", emg.muscle_group AS "group!",
                  emg.amount AS "amount!", mg.name AS "name!"
           FROM exercise_muscle_group emg JOIN muscle_group mg ON mg.id = emg.muscle_group"#
    )
    .fetch_all(&db.pool)
    .await?;
    let mut by_ex: HashMap<i64, Vec<GroupRow>> = HashMap::new();
    for g in groups {
        by_ex.entry(g.exercise).or_default().push(GroupRow {
            id: g.group,
            name: g.name,
            amount: g.amount,
        });
    }
    let mut index = HashMap::new();
    let mut out = Vec::new();
    for r in rows {
        index.insert(r.id, r.name.clone());
        out.push(ExerciseRow {
            id: r.id,
            name: r.name,
            note: r.note,
            video_url: r.video_url,
            groups: by_ex.remove(&r.id).unwrap_or_default(),
        });
    }
    Ok((index, out))
}

pub async fn places(db: &Db) -> Res<Vec<PlaceRow>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", name AS "name!", note AS "note!" FROM place ORDER BY name"#
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| PlaceRow {
            id: r.id,
            name: r.name,
            note: r.note,
        })
        .collect())
}

pub struct Sessions {
    pub rows: Vec<SessionRow>,
    pub dates: HashMap<i64, NaiveDate>,
    pub index: HashMap<i64, String>,
}

/// The last `limit` sessions with their sets, and the PRs each one set.
///
/// A PR is the best estimated 1RM of an exercise over every earlier session of
/// that exercise, wherever it was trained, so the comparison is against the
/// whole log and not against the window this page happens to show.
pub async fn sessions_with_sets(db: &Db, limit: Option<i64>) -> Res<Sessions> {
    let rows = sqlx::query!(
        r#"SELECT s.id AS "id!", s.date, p.name AS "place!", s.note AS "note!",
                  s.deload AS "deload!"
           FROM session s JOIN place p ON p.id = s.place
           ORDER BY julianday(s.date) DESC, s.id DESC LIMIT ?1"#,
        limit.unwrap_or(-1)
    )
    .fetch_all(&db.pool)
    .await?;
    let (index, _) = exercise_index(db).await?;
    let sets = all_sets(db).await?;

    let mut dates = HashMap::new();
    let mut out = Vec::new();
    for r in rows {
        let Some(d) = local_date(&r.date) else {
            continue;
        };
        dates.insert(r.id, d);
        out.push(SessionRow {
            id: r.id,
            date: iso(d),
            place: r.place,
            note: r.note,
            deload: r.deload != 0,
            sets: Vec::new(),
            prs: Vec::new(),
        });
    }

    let mut by_session: HashMap<i64, Vec<&RawSet>> = HashMap::new();
    for s in &sets {
        by_session.entry(s.session).or_default().push(s);
    }

    // Best e1RM per exercise per session, over the whole log, then a running
    // maximum by date: a session outside the window still counts as "earlier".
    let all_dates = session_dates(db).await?;
    let mut per_ex: HashMap<i64, BTreeMap<NaiveDate, f64>> = HashMap::new();
    for s in &sets {
        let Some(d) = all_dates.get(&s.session).copied() else {
            continue;
        };
        if let Ok(e) = to_one_rep_max(s.load, s.rep) {
            let v = per_ex
                .entry(s.exercise)
                .or_default()
                .entry(d)
                .or_insert(0.0);
            if e > *v {
                *v = e;
            }
        }
    }
    let mut best_before: HashMap<i64, Vec<(NaiveDate, f64)>> = HashMap::new();
    for (ex, series) in &per_ex {
        let mut running = f64::NEG_INFINITY;
        let mut out = Vec::new();
        for (d, e) in series {
            out.push((*d, running));
            if *e > running {
                running = *e;
            }
        }
        best_before.insert(*ex, out);
    }
    let previous = |ex: i64, d: NaiveDate| -> f64 {
        best_before
            .get(&ex)
            .and_then(|v| {
                v.iter()
                    .filter(|(when, _)| *when < d)
                    .map(|(_, best)| *best)
                    .fold(None, |acc: Option<f64>, v| {
                        Some(acc.map(|a| a.max(v)).unwrap_or(v))
                    })
            })
            .unwrap_or(f64::NEG_INFINITY)
    };

    for row in out.iter_mut() {
        let date = dates[&row.id];
        if let Some(ms) = by_session.get(&row.id) {
            for s in ms {
                row.sets.push(SetRow {
                    id: s.id,
                    position: s.position,
                    exercise: index.get(&s.exercise).cloned().unwrap_or_default(),
                    load: s.load,
                    rep: s.rep,
                    note: s.note.clone(),
                    superset: s.superset,
                    inferred: s.inferred,
                });
            }
        }
        let mut prs: Vec<PrRow> = Vec::new();
        for s in by_session.get(&row.id).cloned().unwrap_or_default() {
            let Ok(e) = to_one_rep_max(s.load, s.rep) else {
                continue;
            };
            let before = previous(s.exercise, date);
            if e > before {
                prs.push(PrRow {
                    exercise: index.get(&s.exercise).cloned().unwrap_or_default(),
                    load: s.load,
                    rep: s.rep,
                    e1rm: e,
                });
            }
        }
        prs.sort_by(|a, b| {
            b.e1rm
                .partial_cmp(&a.e1rm)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        prs.dedup_by(|a, b| a.exercise == b.exercise);
        prs.truncate(3);
        row.prs = prs;
    }
    Ok(Sessions {
        rows: out,
        dates,
        index,
    })
}

async fn session_dates(db: &Db) -> Res<HashMap<i64, NaiveDate>> {
    let rows = sqlx::query!(r#"SELECT id AS "id!", date FROM session"#)
        .fetch_all(&db.pool)
        .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| local_date(&r.date).map(|d| (r.id, d)))
        .collect())
}

pub async fn gym_daily(db: &Db) -> Res<GymDaily> {
    let s = settings(db).await?;
    let p = phase(db).await?;
    let sessions = sessions_with_sets(db, Some(8)).await?;
    let (_, exercises) = exercise_index(db).await?;
    let last_deload = sessions
        .rows
        .iter()
        .find(|s| s.deload)
        .map(|s| s.date.clone());
    let today = Local::now().date_naive();
    let weeks_since = last_deload
        .as_ref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .map(|d| (today - d).num_days() / 7);
    Ok(GymDaily {
        places: places(db).await?,
        exercises,
        sessions: sessions.rows,
        last_deload,
        deload_weeks: s.deload_weeks,
        weeks_since_deload: weeks_since,
        phase: p,
        place: None,
    })
}

pub async fn gym_edit(db: &Db) -> Res<GymEdit> {
    let (_, exercises) = exercise_index(db).await?;
    let groups =
        sqlx::query!(r#"SELECT id AS "id!", name AS "name!" FROM muscle_group ORDER BY name"#)
            .fetch_all(&db.pool)
            .await?;
    let sessions = sessions_with_sets(db, Some(20)).await?;
    Ok(GymEdit {
        exercises,
        places: places(db).await?,
        groups: groups
            .into_iter()
            .map(|g| MuscleRow {
                id: g.id,
                name: g.name,
            })
            .collect(),
        sessions: sessions.rows,
    })
}

pub async fn gym_analysis(db: &Db) -> Res<GymAnalysis> {
    let (index, exercises) = exercise_index(db).await?;
    let sets = all_sets(db).await?;
    let dates = session_dates(db).await?;
    let deload_sessions = sqlx::query!(r#"SELECT id AS "id!" FROM session WHERE deload = 1"#)
        .fetch_all(&db.pool)
        .await?;
    let deloads: Vec<i64> = deload_sessions
        .iter()
        .filter_map(|r| dates.get(&r.id))
        .map(|d| days(*d))
        .collect();

    let primary: HashMap<i64, String> = exercises
        .iter()
        .map(|e| {
            (
                e.id,
                e.groups
                    .first()
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|| "ungrouped".to_string()),
            )
        })
        .collect();
    let ungrouped: Vec<String> = exercises
        .iter()
        .filter(|e| e.groups.is_empty())
        .map(|e| e.name.clone())
        .collect();

    let mut best: BTreeMap<(i64, NaiveDate), f64> = BTreeMap::new();
    for s in &sets {
        let Some(d) = dates.get(&s.session) else {
            continue;
        };
        if let Ok(e) = to_one_rep_max(s.load, s.rep) {
            let v = best.entry((s.exercise, *d)).or_insert(0.0);
            if e > *v {
                *v = e;
            }
        }
    }
    let mut per_ex: HashMap<i64, Vec<StrengthPoint>> = HashMap::new();
    for ((ex, d), e1rm) in best {
        per_ex
            .entry(ex)
            .or_default()
            .push(StrengthPoint { day: days(d), e1rm });
    }
    let mut strength: Vec<StrengthSeries> = per_ex
        .into_iter()
        .filter_map(|(ex, mut points)| {
            points.sort_by_key(|p| p.day);
            (points.len() >= 2).then(|| StrengthSeries {
                exercise: index.get(&ex).cloned().unwrap_or_default(),
                group: primary
                    .get(&ex)
                    .cloned()
                    .unwrap_or_else(|| "ungrouped".to_string()),
                points,
            })
        })
        .collect();
    strength.sort_by(|a, b| a.group.cmp(&b.group).then(a.exercise.cmp(&b.exercise)));

    let shares: HashMap<i64, Vec<(String, f64)>> = exercises
        .iter()
        .map(|e| {
            (
                e.id,
                e.groups
                    .iter()
                    .map(|g| (g.name.clone(), g.amount))
                    .collect(),
            )
        })
        .collect();
    let mut vol: HashMap<String, BTreeMap<NaiveDate, f64>> = HashMap::new();
    for s in &sets {
        let Some(d) = dates.get(&s.session) else {
            continue;
        };
        let week = *d - Duration::days(d.weekday().num_days_from_monday() as i64);
        for (name, amount) in shares.get(&s.exercise).cloned().unwrap_or_default() {
            *vol.entry(name).or_default().entry(week).or_insert(0.0) += amount;
        }
    }
    let mut volume: Vec<VolumeSeries> = vol
        .into_iter()
        .map(|(group, points)| VolumeSeries {
            group,
            points: points
                .into_iter()
                .map(|(d, sets)| VolumePoint { day: days(d), sets })
                .collect(),
        })
        .collect();
    volume.sort_by(|a, b| a.group.cmp(&b.group));

    Ok(GymAnalysis {
        strength,
        volume,
        ungrouped,
        deloads,
        span: 180,
    })
}
