//! Queries behind the pages, and the DTOs they fill.
//!
//! Server only: these run in the binary, never in the browser. Row timestamps
//! are stored as RFC3339 with whatever offset they were written with, so every
//! day boundary is computed here, in the machine's local zone, rather than in
//! SQL — one place for the rule, and the same rule the CLI's `gym food` uses.
//!
//! The pickers are ordered here: a list shown "sorted by recency" is sorted by
//! the query, not by the browser, and `last_used` is carried along so the page
//! can say how long ago the row was used.

use crate::db::Db;
use crate::stats::{self, DayIntake, Goal, Point};
use crate::train::{self, SetEffort};
use crate::util::Res;
use crate::web::dto::*;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone};
use std::collections::{BTreeMap, BTreeSet, HashMap};

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

fn clock() -> String {
    Local::now().format("%H:%M").to_string()
}

/// Midnight at the start of `d`, **in the machine's zone**, as an RFC3339
/// instant.
///
/// The window comparisons run in SQL against `julianday`, which reads a bare
/// `YYYY-MM-DD` as UTC midnight. Passing one would silently shift the day by
/// the zone's offset — nine hours here — and drop every row written after local
/// midnight but before UTC midnight. An instant has no such ambiguity.
fn midnight(d: NaiveDate) -> String {
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

/// Every phase in the log, as a closed span in day numbers: a phase ends the
/// day before the next one starts, and the newest one is open-ended.
async fn phase_spans(db: &Db, today: NaiveDate) -> Res<Vec<PhaseSpan>> {
    let rows = sqlx::query!(
        r#"SELECT kind AS "kind!", start AS "start!" FROM phase ORDER BY julianday(start)"#
    )
    .fetch_all(&db.pool)
    .await?;
    let mut out: Vec<PhaseSpan> = Vec::new();
    for r in &rows {
        let Some(from) = local_date(&r.start) else {
            continue;
        };
        if let Some(last) = out.last_mut() {
            last.to = days(from) - 1;
        }
        out.push(PhaseSpan {
            kind: r.kind.clone(),
            from: days(from),
            to: days(today),
        });
    }
    Ok(out)
}

/// The weeks a deload fell in, as closed spans of day numbers, Monday to
/// Sunday, clipped to `from` and deduplicated.
///
/// A deload is a week's plan, so shading the whole week says more than a mark
/// on the single session that happened to be logged: the trend keeps every
/// point and the reader sees why the loads dipped.
async fn deload_spans(db: &Db, from: NaiveDate) -> Res<Vec<Span>> {
    let rows = sqlx::query!(r#"SELECT date FROM session WHERE deload = 1"#)
        .fetch_all(&db.pool)
        .await?;
    let mut weeks: BTreeSet<(NaiveDate, NaiveDate)> = BTreeSet::new();
    for r in &rows {
        let Some(d) = local_date(&r.date) else {
            continue;
        };
        let monday = d - Duration::days(d.weekday().num_days_from_monday() as i64);
        let sunday = monday + Duration::days(6);
        if sunday >= from {
            weeks.insert((monday.max(from), sunday));
        }
    }
    Ok(weeks
        .into_iter()
        .map(|(start, end)| Span {
            from: days(start),
            to: days(end),
        })
        .collect())
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
    let first = window.first().map(|(d, _)| *d);
    TrendView {
        window_days: s.trend_window_days,
        points: t
            .points
            .iter()
            .map(|(d, kg)| PointRow {
                day: days(*d),
                kg: *kg,
            })
            .collect(),
        fit: fit
            .zip(first)
            .map(|(f, first)| {
                t.points
                    .iter()
                    .map(|(d, _)| PointRow {
                        day: days(*d),
                        kg: f.at(first, *d),
                    })
                    .collect()
            })
            .unwrap_or_default(),
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

/// Weigh-ins in a window, oldest first, with the row the edit page shows.
async fn weights_between(
    db: &Db,
    from: NaiveDate,
    to: NaiveDate,
) -> Res<Vec<(NaiveDate, f64, WeightRow)>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", date, kg AS "kg!", bodyfat, note
           FROM weight
           WHERE julianday(date) >= julianday(?1) AND julianday(date) < julianday(?2)
           ORDER BY julianday(date), id"#,
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
                    time: local_time(&r.date),
                    kg: r.kg,
                    bodyfat: r.bodyfat,
                    note: r.note,
                },
            ))
        })
        .collect())
}

/// The last `limit` weigh-ins, newest first.
async fn recent_weights(db: &Db, limit: i64) -> Res<Vec<WeightRow>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", date, kg AS "kg!", bodyfat, note
           FROM weight ORDER BY julianday(date) DESC, id DESC LIMIT ?1"#,
        limit
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| WeightRow {
            id: r.id,
            date: local_date(&r.date).map(iso).unwrap_or_default(),
            time: local_time(&r.date),
            kg: r.kg,
            bodyfat: r.bodyfat,
            note: r.note,
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
ORDER BY julianday(m.date), m.id
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

/// Every food, most recently eaten first: the picker's order.
async fn foods(db: &Db) -> Res<Vec<FoodRow>> {
    let rows = sqlx::query!(
        r#"
SELECT f.id AS "id!", f.name AS "name!", f.calories AS "kcal!",
       COALESCE(f.protein, 0.0) AS "protein!: f64",
       COALESCE(f.fat, 0.0) AS "fat!: f64",
       COALESCE(f.carbohydrate, 0.0) AS "carbohydrate!: f64",
       f.note AS "note!",
       (SELECT count(*) FROM meal m WHERE m.food = f.id) AS "eaten!",
       (SELECT MAX(m.date) FROM meal m WHERE m.food = f.id) AS "last"
FROM food f
ORDER BY COALESCE(julianday((SELECT MAX(m.date) FROM meal m WHERE m.food = f.id)), 0.0) DESC,
         f.name
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
            last_used: r.last.as_deref().and_then(|d| local_date(d)).map(iso),
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

/// A day of intake, split by food, with the macros each food contributed.
fn day_rows(meals: &[MealRow]) -> Vec<DayRow> {
    let mut out = Vec::new();
    for (d, ms) in day_map(meals) {
        let t = sum(ms.iter().copied());
        let mut per_food: HashMap<&str, Totals> = HashMap::new();
        for m in &ms {
            let e = per_food.entry(m.food.as_str()).or_default();
            e.kcal += m.kcal;
            e.protein += m.protein;
            e.fat += m.fat;
            e.carbohydrate += m.carbohydrate;
        }
        let mut split: Vec<SplitRow> = per_food
            .into_iter()
            .map(|(food, t)| SplitRow {
                food: food.to_owned(),
                kcal: t.kcal,
                protein: t.protein,
                fat: t.fat,
                carbohydrate: t.carbohydrate,
            })
            .collect();
        split.sort_by(|a, b| {
            b.kcal
                .partial_cmp(&a.kcal)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        split.truncate(8);
        out.push(DayRow {
            date: iso(d),
            day: days(d),
            logged: t.kcal >= stats::LOGGED_DAY_KCAL,
            totals: t,
            split,
        });
    }
    out
}

/// Every food's contribution over a stretch of meals, added up and ordered by
/// calories: the ranking's data, which a day's split cannot be, since a day
/// keeps only its top few foods.
fn split_by_food(meals: &[MealRow]) -> Vec<SplitRow> {
    let mut per_food: HashMap<&str, Totals> = HashMap::new();
    for m in meals {
        let e = per_food.entry(m.food.as_str()).or_default();
        e.kcal += m.kcal;
        e.protein += m.protein;
        e.fat += m.fat;
        e.carbohydrate += m.carbohydrate;
    }
    let mut out: Vec<SplitRow> = per_food
        .into_iter()
        .map(|(food, t)| SplitRow {
            food: food.to_owned(),
            kcal: t.kcal,
            protein: t.protein,
            fat: t.fat,
            carbohydrate: t.carbohydrate,
        })
        .collect();
    out.sort_by(|a, b| {
        b.kcal
            .partial_cmp(&a.kcal)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.food.cmp(&b.food))
    });
    out
}

/// The maintenance estimate and the days it cannot explain, from the weight
/// trend and the logged intake in the same window.
fn intake_health(
    meals: &[MealRow],
    weights: &[(NaiveDate, f64)],
    s: &Settings,
    today: NaiveDate,
) -> (
    Option<MaintenanceView>,
    usize,
    Vec<OutlierRow>,
    Option<stats::Fit>,
) {
    let span = Duration::days(s.trend_window_days.max(30));
    let mut w: Vec<Point> = weights
        .iter()
        .filter(|(d, _)| *d > today - span)
        .copied()
        .collect();
    w.sort_by_key(|(d, _)| *d);
    let fit = stats::trend(&w, s.trend_min_weighins).shown();
    let intake: Vec<DayIntake> = day_map(meals)
        .iter()
        .map(|(d, ms)| DayIntake::new(*d, sum(ms.iter().copied()).kcal))
        .collect();
    let logged = intake.iter().filter(|d| d.logged).count();
    let maintenance = fit.and_then(|f| stats::maintenance(&intake, f, s.maintenance_min_days));
    let outliers = maintenance
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
    (
        maintenance.map(|m| MaintenanceView {
            kcal: m.kcal,
            days: m.days,
            intake: m.intake,
        }),
        logged,
        outliers,
        fit,
    )
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
    let (maintenance, logged_days, outliers, _) = intake_health(&window_meals, &flat, &s, today);

    Ok(FoodDaily {
        today: iso(today),
        now: clock(),
        totals: sum(meals.iter()),
        meals,
        foods: foods(db).await?,
        last_weight: weights.last().map(|(_, _, w)| w.clone()),
        trend,
        maintenance,
        logged_days,
        need_days: s.maintenance_min_days,
        outliers,
        phase: p,
        presets: crate::web::actions::presets(db).await?,
    })
}

pub async fn food_edit(db: &Db) -> Res<FoodEdit> {
    let today = Local::now().date_naive();
    let mut meals = meals_between(db, today - Duration::days(120), today).await?;
    meals.reverse();
    Ok(FoodEdit {
        today: iso(today),
        now: clock(),
        presets: crate::web::actions::presets(db).await?,
        meals,
        foods: foods(db).await?,
    })
}

pub async fn weight_daily(db: &Db) -> Res<WeightDaily> {
    let today = Local::now().date_naive();
    let s = settings(db).await?;
    let p = phase(db).await?;
    let span = Duration::days(s.trend_window_days.max(30));
    let window_meals = meals_between(db, today - span, today).await?;
    let weights = weights_between(db, today - Duration::days(365), today).await?;
    let flat: Vec<(NaiveDate, f64)> = weights.iter().map(|(d, kg, _)| (*d, *kg)).collect();
    let trend = trend_view(&flat, &s, &p, today);
    let (_, _, outliers, _) = intake_health(&window_meals, &flat, &s, today);
    let recent = recent_weights(db, 30).await?;
    Ok(WeightDaily {
        today: iso(today),
        now: clock(),
        last_weight: recent.first().cloned(),
        weights: recent,
        trend,
        phase: p,
        outliers,
    })
}

pub async fn weight_edit(db: &Db) -> Res<WeightEdit> {
    Ok(WeightEdit {
        today: iso(Local::now().date_naive()),
        now: clock(),
        weights: recent_weights(db, 120).await?,
    })
}

/// Weight · analyse: the trend split by phase, body fat, and the intake bars
/// underneath.
pub async fn weight_analysis(db: &Db, span_days: i64) -> Res<WeightAnalysis> {
    let today = Local::now().date_naive();
    let span_days = span_days.clamp(7, 730);
    let s = settings(db).await?;
    let p = phase(db).await?;
    let from = today - Duration::days(span_days);
    let meals = meals_between(db, from, today).await?;
    let weights = weights_between(db, from, today).await?;
    let flat: Vec<(NaiveDate, f64)> = weights.iter().map(|(d, kg, _)| (*d, *kg)).collect();
    let trend = trend_view(&flat, &s, &p, today);
    let (maintenance, _, _, _) = intake_health(&meals, &flat, &s, today);
    Ok(WeightAnalysis {
        line: flat
            .iter()
            .map(|(d, kg)| PointRow {
                day: days(*d),
                kg: *kg,
            })
            .collect(),
        phases: phase_spans(db, today).await?,
        last_weight: recent_weights(db, 1).await?.into_iter().next(),
        bodyfat: weights
            .iter()
            .filter_map(|(d, _, w)| {
                w.bodyfat.map(|b| PointRow {
                    day: days(*d),
                    kg: b,
                })
            })
            .collect(),
        days: day_rows(&meals),
        logged_days: day_rows(&meals).iter().filter(|d| d.logged).count(),
        ranking: split_by_food(&meals),
        deloads: deload_spans(db, from).await?,
        trend,
        weights: weights.into_iter().map(|(_, _, w)| w).collect(),
        span: span_days as usize,
        maintenance,
    })
}

pub struct RawSet {
    pub id: i64,
    pub session: i64,
    pub exercise: i64,
    pub position: i64,
    pub date: String,
    pub load: f64,
    pub rep: f64,
    pub note: String,
    pub superset: Option<i64>,
    pub inferred: bool,
    pub effort: Option<String>,
    pub effort_rank: Option<i64>,
}

impl RawSet {
    fn row(&self, index: &HashMap<i64, String>) -> SetRow {
        SetRow {
            id: self.id,
            position: self.position,
            exercise: index.get(&self.exercise).cloned().unwrap_or_default(),
            load: self.load,
            rep: self.rep,
            note: self.note.clone(),
            superset: self.superset,
            inferred: self.inferred,
            effort: self.effort.clone(),
            effort_rank: self.effort_rank,
            time: local_time(&self.date),
        }
    }

    fn effort(&self) -> SetEffort {
        SetEffort {
            load: self.load,
            rep: self.rep,
            effort: self.effort_rank,
            warmup: !self.is_working(),
        }
    }

    /// Whether the set counts as work.
    ///
    /// A warmup is recorded, shown and corrected like any other set, but it is
    /// not a set taken near failure: the advice, the row's own pre-filled
    /// numbers, the PR badge, the strength line and the sets-per-week volume
    /// all leave it out, so a warmup can neither fake progress nor pass for the
    /// work it prepared for.
    fn is_working(&self) -> bool {
        !train::is_warmup(self.effort.as_deref())
    }
}

/// The set a session is compared by: its best working set. `None` when the
/// session holds only warmups, which is not a session of work.
fn best_of(sets: &[&RawSet]) -> Option<SetEffort> {
    train::best(&sets.iter().map(|s| s.effort()).collect::<Vec<_>>())
}

async fn all_sets(db: &Db) -> Res<Vec<RawSet>> {
    let rows = sqlx::query!(
        r#"SELECT ts.id AS "id!", ts.session AS "session!", ts.exercise AS "exercise!",
                  ts.position AS "position!", ts.date, ts.load AS "load!", ts.rep AS "rep!",
                  ts.note AS "note!", ts.superset, ts.inferred AS "inferred!",
                  e.name AS "effort", e.sort AS "effort_rank"
           FROM training_set ts LEFT JOIN effort e ON e.id = ts.effort
           ORDER BY ts.session, ts.position"#
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
            date: r.date,
            load: r.load,
            rep: r.rep,
            note: r.note,
            superset: r.superset,
            inferred: r.inferred != 0,
            effort: r.effort,
            effort_rank: r.effort_rank,
        })
        .collect())
}

pub async fn exercise_index(db: &Db) -> Res<(HashMap<i64, String>, Vec<ExerciseRow>)> {
    let rows = sqlx::query!(
        r#"SELECT e.id AS "id!", e.name AS "name!", e.note AS "note!", e.video_url AS "video_url",
                  (SELECT MAX(ts.date) FROM training_set ts WHERE ts.exercise = e.id) AS "last"
           FROM exercise e
           ORDER BY COALESCE(julianday((SELECT MAX(ts.date) FROM training_set ts WHERE ts.exercise = e.id)), 0.0) DESC,
                    e.name"#
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
            last_used: r.last.as_deref().and_then(local_date).map(iso),
        });
    }
    Ok((index, out))
}

pub async fn places(db: &Db) -> Res<Vec<PlaceRow>> {
    let rows = sqlx::query!(
        r#"SELECT p.id AS "id!", p.name AS "name!", p.note AS "note!",
                  (SELECT MAX(s.date) FROM session s WHERE s.place = p.id) AS "last"
           FROM place p
           ORDER BY COALESCE(julianday((SELECT MAX(s.date) FROM session s WHERE s.place = p.id)), 0.0) DESC,
                    p.name"#
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| PlaceRow {
            id: r.id,
            name: r.name,
            note: r.note,
            last_used: r.last.as_deref().and_then(local_date).map(iso),
        })
        .collect())
}

/// The efforts the window offers: the three rungs of the ladder, hardest first,
/// and then the warmup marker. A warmup is picked in the same dropdown because
/// it is what the set was — `effort` is the column that records it — but it is
/// not a fourth rung; see [`train::WARMUP`]. A set recorded with one of the
/// older levels still shows its own name.
async fn efforts(db: &Db) -> Res<Vec<EffortRow>> {
    let rows = sqlx::query!(
        r#"SELECT id AS "id!", name AS "name!", sort AS "sort!"
           FROM effort WHERE name IN ('hard', 'form fail', 'fail', 'warmup') ORDER BY sort"#
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| EffortRow {
            id: r.id,
            hint: match r.name.as_str() {
                "hard" => "3+ reps from failure".to_string(),
                "form fail" => "1–2 reps from failure".to_string(),
                train::WARMUP => "not near failure: outside the ladder".to_string(),
                _ => "0 reps from failure".to_string(),
            },
            name: r.name,
            rank: r.sort,
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
            time: local_time(&r.date),
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
    // Warmups are left out on both sides, so a light warmup of many reps cannot
    // take the record and the record is never a warmup.
    let all_dates = session_dates(db).await?;
    let mut per_ex: HashMap<i64, BTreeMap<NaiveDate, f64>> = HashMap::new();
    for s in &sets {
        if !s.is_working() {
            continue;
        }
        let Some(d) = all_dates.get(&s.session).copied() else {
            continue;
        };
        if let Some(e) = train::one_rep_max(s.load, s.rep) {
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
                row.sets.push(s.row(&index));
            }
        }
        let mut prs: Vec<PrRow> = Vec::new();
        for s in by_session.get(&row.id).cloned().unwrap_or_default() {
            if !s.is_working() {
                continue;
            }
            let Some(e) = train::one_rep_max(s.load, s.rep) else {
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

/// The session currently being recorded: the newest one that has not been
/// ended.
async fn live_session(db: &Db, index: &HashMap<i64, String>) -> Res<Option<LiveSession>> {
    let row = sqlx::query!(
        r#"SELECT s.id AS "id!", s.date, p.name AS "place!", s.note AS "note!",
                  s.deload AS "deload!"
           FROM session s JOIN place p ON p.id = s.place
           WHERE s.ended IS NULL
           ORDER BY julianday(s.date) DESC, s.id DESC LIMIT 1"#
    )
    .fetch_optional(&db.pool)
    .await?;
    let Some(r) = row else {
        return Ok(None);
    };
    let items = sqlx::query!(
        r#"SELECT sli.exercise AS "exercise!", e.name AS "name!", sli.target_sets,
                  (SELECT MAX(ts.date) FROM training_set ts WHERE ts.exercise = sli.exercise) AS "last"
           FROM session_list_item sli JOIN exercise e ON e.id = sli.exercise
           WHERE sli.session = ?1 ORDER BY sli.position"#,
        r.id
    )
    .fetch_all(&db.pool)
    .await?;
    let mine: Vec<RawSet> = all_sets(db)
        .await?
        .into_iter()
        .filter(|s| s.session == r.id)
        .collect();
    let mut list: Vec<ChecklistRow> = items
        .into_iter()
        .map(|i| ChecklistRow {
            exercise: i.exercise,
            name: i.name,
            last_used: i.last.as_deref().and_then(local_date).map(iso),
            sets: 0,
            target: i.target_sets,
        })
        .collect();
    // The checklist's count is work only: a warmup is recorded and shown like
    // any other set, but it is not one of the sets the target asks for, so it
    // can neither start an exercise nor tick it off.
    for s in mine.iter().filter(|s| s.is_working()) {
        if let Some(item) = list.iter_mut().find(|i| i.exercise == s.exercise) {
            item.sets += 1;
        }
    }
    let sets = mine.iter().map(|s| s.row(index)).collect();
    let date = local_date(&r.date).map(iso).unwrap_or_default();
    Ok(Some(LiveSession {
        id: r.id,
        time: local_time(&r.date),
        date,
        place: r.place,
        deload: r.deload != 0,
        note: r.note,
        list,
        sets,
    }))
}

pub async fn gym_daily(db: &Db) -> Res<GymDaily> {
    let s = settings(db).await?;
    let p = phase(db).await?;
    let (index, exercises) = exercise_index(db).await?;
    let open = live_session(db, &index).await?;
    // The offer is for a list that is still empty. Once the reader has put
    // something on it they have built a session of their own, so a second offer
    // would be an argument rather than a prompt; applying the preset empties the
    // question by filling the list.
    let offer = match open.as_ref() {
        Some(live) if live.list.is_empty() => split_offer(db, &live.date).await?,
        _ => None,
    };
    // The reminder counts from the last deload anywhere in the log, not from
    // the last few sessions: a deload 30 sessions ago is still the last one.
    // The row stores an instant, so its date is the local day it fell on, the
    // same way every other date on the page is derived.
    let last_deload: Option<String> = sqlx::query_scalar!(
        r#"SELECT date FROM session WHERE deload = 1 ORDER BY date DESC LIMIT 1"#
    )
    .fetch_optional(&db.pool)
    .await?
    .and_then(|at| local_date(&at).map(iso));
    let today = Local::now().date_naive();
    let weeks_since = last_deload
        .as_ref()
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .map(|d| (today - d).num_days() / 7);
    Ok(GymDaily {
        today: iso(today),
        now: clock(),
        places: places(db).await?,
        exercises,
        presets: gym_presets(db).await?,
        efforts: efforts(db).await?,
        open,
        last_deload,
        deload_weeks: s.deload_weeks,
        weeks_since_deload: weeks_since,
        phase: p,
        offer,
    })
}

/// What an active split asks for on a day.
///
/// The day is the session's own date, not the clock: a session started late on
/// Sunday and ended on Monday is Sunday's session, and the suggestion must
/// match the day the reader sat down to train.
async fn split_offer(db: &Db, date: &str) -> Res<Option<SplitOffer>> {
    let Some(day) = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok() else {
        return Ok(None);
    };
    let weekday = day.weekday().number_from_monday() as i64;
    let row = sqlx::query!(
        r#"SELECT s.name AS "split!", el.id AS "preset!", el.name AS "preset_name!",
                  (SELECT count(*) FROM exercise_list_item i WHERE i.list = el.id) AS "exercises!: i64"
           FROM split s
           JOIN split_day d ON d.split = s.id
           JOIN exercise_list el ON el.id = d.preset
           WHERE s.active = 1 AND d.weekday = ?1"#,
        weekday
    )
    .fetch_optional(&db.pool)
    .await?;
    Ok(row.map(|r| SplitOffer {
        weekday: weekday_name(weekday).to_string(),
        split: r.split,
        preset: r.preset,
        preset_name: r.preset_name,
        exercises: r.exercises,
    }))
}

/// The gym's presets: a named list of exercises to work through.
pub async fn gym_presets(db: &Db) -> Res<Vec<GymPreset>> {
    let lists =
        sqlx::query!(r#"SELECT id AS "id!", name AS "name!" FROM exercise_list ORDER BY name"#)
            .fetch_all(&db.pool)
            .await?;
    let items = sqlx::query!(
        r#"SELECT li.list AS "list!", li.exercise AS "exercise!", e.name AS "name!",
                  li.target_sets AS "sets!"
           FROM exercise_list_item li JOIN exercise e ON e.id = li.exercise
           ORDER BY li.list, li.position"#
    )
    .fetch_all(&db.pool)
    .await?;
    let mut by_list: HashMap<i64, Vec<PresetExercise>> = HashMap::new();
    for i in items {
        by_list.entry(i.list).or_default().push(PresetExercise {
            exercise: i.exercise,
            name: i.name,
            sets: i.sets,
        });
    }
    Ok(lists
        .into_iter()
        .map(|l| GymPreset {
            items: by_list.remove(&l.id).unwrap_or_default(),
            id: l.id,
            name: l.name,
        })
        .collect())
}

/// The splits, each with all seven weekdays filled in so an editor can offer
/// every day rather than only the ones already set.
pub async fn splits(db: &Db) -> Res<Vec<SplitWeek>> {
    let rows = sqlx::query!(
        r#"SELECT s.id AS "id!", s.name AS "name!", s.active AS "active!",
                  d.weekday AS "weekday", d.preset AS "preset", e.name AS "preset_name"
           FROM split s
           LEFT JOIN split_day d ON d.split = s.id
           LEFT JOIN exercise_list e ON e.id = d.preset
           ORDER BY s.name, d.weekday"#
    )
    .fetch_all(&db.pool)
    .await?;
    let mut by_split: HashMap<i64, (String, bool, Vec<SplitDayRow>)> = HashMap::new();
    for r in rows {
        let entry = by_split
            .entry(r.id)
            .or_insert_with(|| (r.name, r.active != 0, Vec::new()));
        if let Some(weekday) = r.weekday {
            entry.2.push(SplitDayRow {
                weekday,
                name: weekday_name(weekday).to_string(),
                preset: r.preset,
                preset_name: r.preset_name.unwrap_or_default(),
            });
        }
    }
    let mut out: Vec<SplitWeek> = by_split
        .into_iter()
        .map(|(id, (name, active, mut days))| {
            for weekday in 1..=7 {
                if !days.iter().any(|d| d.weekday == weekday) {
                    days.push(SplitDayRow {
                        weekday,
                        name: weekday_name(weekday).to_string(),
                        preset: None,
                        preset_name: String::new(),
                    });
                }
            }
            days.sort_by_key(|d| d.weekday);
            SplitWeek {
                id,
                name,
                active,
                days,
            }
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// One exercise while it is being recorded: this session's sets, the last
/// completed sessions of it, and what to attempt next.
pub async fn exercise_window(db: &Db, session: i64, exercise: i64) -> Res<ExerciseWindow> {
    let live = sqlx::query!(
        r#"SELECT s.deload AS "deload!", s.date FROM session s WHERE s.id = ?1"#,
        session
    )
    .fetch_optional(&db.pool)
    .await?
    .ok_or_else(|| format!("No session with id '{session}'."))?;
    let (index, exercises) = exercise_index(db).await?;
    let row = exercises
        .iter()
        .find(|e| e.id == exercise)
        .ok_or_else(|| format!("No exercise with id '{exercise}'."))?;
    let group = row
        .groups
        .first()
        .map(|g| g.name.clone())
        .unwrap_or_else(|| "ungrouped".to_string());
    let p = phase(db).await?;
    let target_rep = p.as_ref().map(|p| p.target_rep).unwrap_or(8);

    let all = all_sets(db).await?;
    // A deload session's rows are shown in the history but are not the numbers
    // to beat: its lighter load is deliberate, and treating it as the last
    // performance would read the planned dip as a regression.
    let deloaded: HashMap<i64, bool> =
        sqlx::query!(r#"SELECT id AS "id!", deload AS "deload!" FROM session"#)
            .fetch_all(&db.pool)
            .await?
            .into_iter()
            .map(|r| (r.id, r.deload != 0))
            .collect();
    let mut by_session: BTreeMap<NaiveDate, (bool, Vec<&RawSet>)> = BTreeMap::new();
    for s in all
        .iter()
        .filter(|s| s.exercise == exercise && s.session != session)
    {
        if let Some(d) = local_date(&s.date) {
            let entry = by_session.entry(d).or_insert_with(|| {
                (
                    deloaded.get(&s.session).copied().unwrap_or(false),
                    Vec::new(),
                )
            });
            entry.1.push(s);
        }
    }
    let mut finished: Vec<(NaiveDate, bool, Vec<&RawSet>)> = by_session
        .into_iter()
        .map(|(d, (deload, sets))| (d, deload, sets))
        .collect();
    finished.reverse();

    // The best set of a session is the one with the highest estimated 1RM: one
    // number that compares a heavy single with a light set of many. A session
    // of warmups alone is not the last session of anything: it has no numbers
    // to beat, so it is skipped rather than read as a session that fell behind.
    let normal: Vec<&(NaiveDate, bool, Vec<&RawSet>)> = finished
        .iter()
        .filter(|(_, deload, _)| !*deload)
        .filter(|(_, _, sets)| sets.iter().any(|s| s.is_working()))
        .collect();
    let last = normal.first().and_then(|(_, _, s)| best_of(s));
    let prev = normal.get(1).and_then(|(_, _, s)| best_of(s));
    let last_sets = normal
        .first()
        .map(|(_, _, s)| s.iter().filter(|s| s.is_working()).count() as i64)
        .unwrap_or(0);
    let last_date = normal.first().map(|(d, _, _)| iso(*d)).unwrap_or_default();
    let advice = if live.deload != 0 {
        last.map(|l| {
            // Half the sets, rounded up: half of three sets is a session of
            // two, and a deload of no sets is not a session.
            let half_sets = ((last_sets + 1) / 2).max(1);
            format!(
                "Deload session: {:.1} kg for {} set(s) — half of {:.1} kg × {} set(s) from {last_date}. Rebuild from the normal numbers afterwards.",
                l.load / 2.0,
                half_sets,
                l.load,
                last_sets,
            )
        })
    } else {
        train::recommend(prev, last, target_rep).map(|a| a.why)
    };
    // The row opens on the set just done, because the next set is usually that
    // one again; with none of this session's own sets to copy, it opens on the
    // best set of the last session that did some work. A warmup is neither: it
    // prepared for the numbers, so pre-filling it would repeat the warmup.
    let here = all
        .iter()
        .filter(|s| s.session == session && s.exercise == exercise && s.is_working())
        .next_back();
    let prefill = here
        .map(|s| Prefill {
            load: s.load,
            rep: s.rep,
            effort: s.effort_rank,
        })
        .or_else(|| {
            last.map(|l| Prefill {
                load: l.load,
                rep: l.rep,
                effort: l.effort,
            })
        });
    let history: Vec<HistorySession> = finished
        .iter()
        .take(3)
        .map(|(d, deload, sets)| HistorySession {
            date: iso(*d),
            place: String::new(),
            // The session's own effort, from its first working set: a warmup
            // opened the session, and that is not the effort that was worked at.
            effort: sets
                .iter()
                .find(|s| s.is_working())
                .and_then(|s| s.effort.clone()),
            deload: *deload,
            sets: sets
                .iter()
                .map(|s| HistorySet {
                    load: s.load,
                    rep: s.rep,
                    note: s.note.clone(),
                    time: local_time(&s.date),
                    effort: s.effort.clone(),
                })
                .collect(),
        })
        .collect();

    let sets = all
        .iter()
        .filter(|s| s.session == session && s.exercise == exercise)
        .map(|s| s.row(&index))
        .collect::<Vec<SetRow>>();
    Ok(ExerciseWindow {
        exercise,
        name: row.name.clone(),
        group,
        target_rep,
        deload: live.deload != 0,
        prefill,
        advice,
        history,
        sets,
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
        presets: gym_presets(db).await?,
        splits: splits(db).await?,
        efforts: efforts(db).await?,
        sessions: sessions.rows,
    })
}

/// Gym · analyse: one exercise's strength, and sets per week for it and for
/// the exercises that share its main muscle group.
pub async fn gym_analysis(db: &Db, span_days: i64, want: Option<i64>) -> Res<GymAnalysis> {
    let span_days = span_days.clamp(7, 730);
    let today = Local::now().date_naive();
    let from = today - Duration::days(span_days);
    let (index, exercises) = exercise_index(db).await?;
    let sets = all_sets(db).await?;
    let dates = session_dates(db).await?;

    // Sessions per exercise, which is what the picker offers: a session whose
    // only sets were warmups is not a session of that exercise to analyse.
    let counts: HashMap<i64, i64> = sqlx::query!(
        r#"SELECT ts.exercise AS "exercise!", count(DISTINCT ts.session) AS "n!"
           FROM training_set ts LEFT JOIN effort e ON e.id = ts.effort
           WHERE e.name IS NULL OR e.name <> ?1
           GROUP BY ts.exercise"#,
        train::WARMUP
    )
    .fetch_all(&db.pool)
    .await?
    .into_iter()
    .map(|r| (r.exercise, r.n))
    .collect();
    let group_of: HashMap<i64, String> = exercises
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
    let mut rows: Vec<PickRow> = exercises
        .iter()
        .filter(|e| counts.get(&e.id).copied().unwrap_or(0) >= 2)
        .map(|e| PickRow {
            id: e.id,
            name: e.name.clone(),
            group: group_of.get(&e.id).cloned().unwrap_or_default(),
            sessions: counts.get(&e.id).copied().unwrap_or(0),
        })
        .collect();
    rows.sort_by(|a, b| a.group.cmp(&b.group).then(a.name.cmp(&b.name)));

    // Open on the exercise trained in the most sessions: a chart that opens
    // empty is a chart the reader has to work out first.
    let pick = want
        .or_else(|| rows.iter().max_by_key(|r| r.sessions).map(|r| r.id))
        .filter(|id| rows.iter().any(|r| r.id == *id));

    let in_window = |s: &RawSet| -> Option<NaiveDate> {
        let d = *dates.get(&s.session)?;
        (d >= from).then_some(d)
    };
    let week_of = |d: NaiveDate| d - Duration::days(d.weekday().num_days_from_monday() as i64);

    // Sets per week is volume: a warmup is not work, so counting it would make
    // the bar climb every time the reader prepared for the work.
    let week_sets = |ex: i64| -> Vec<VolumePoint> {
        let mut by_week: BTreeMap<NaiveDate, f64> = BTreeMap::new();
        for s in sets.iter().filter(|s| s.exercise == ex && s.is_working()) {
            if let Some(d) = in_window(s) {
                *by_week.entry(week_of(d)).or_insert(0.0) += 1.0;
            }
        }
        by_week
            .into_iter()
            .map(|(d, sets)| VolumePoint { day: days(d), sets })
            .collect()
    };

    let chosen = pick.map(|id| {
        let name = index.get(&id).cloned().unwrap_or_default();
        let group = group_of.get(&id).cloned().unwrap_or_default();
        // The session's strength is its best working set, the same set the
        // advice compares: a warmup is drawn on neither.
        let mut by_day: BTreeMap<NaiveDate, Vec<SetEffort>> = BTreeMap::new();
        for s in sets.iter().filter(|s| s.exercise == id) {
            if let Some(d) = in_window(s) {
                by_day.entry(d).or_default().push(s.effort());
            }
        }
        let strength: Vec<StrengthPoint> = by_day
            .into_iter()
            .filter_map(|(d, session)| {
                let best = train::best(&session)?;
                let e1rm = train::one_rep_max(best.load, best.rep)?;
                Some(StrengthPoint { day: days(d), e1rm })
            })
            .collect();
        let overlaps = exercises
            .iter()
            .filter(|e| e.id != id && group_of.get(&e.id) == Some(&group))
            .filter(|e| counts.get(&e.id).copied().unwrap_or(0) > 0)
            .map(|e| (e.id, e.name.clone()))
            .map(|(eid, ename)| OverlapSeries {
                exercise: ename,
                sets: week_sets(eid),
            })
            .filter(|o| !o.sets.is_empty())
            .collect();
        AnalysisSeries {
            strength,
            sets: week_sets(id),
            overlaps,
            exercise: name,
            group,
        }
    });

    let deloads = deload_spans(db, from).await?;

    Ok(GymAnalysis {
        exercises: rows,
        chosen,
        span: span_days as usize,
        deloads,
    })
}

/// The last `limit` sessions of one exercise, newest first, wherever it was
/// trained: the window's context does not depend on today's place.
pub async fn exercise_history(db: &Db, exercise: i64, limit: i64) -> Res<History> {
    let name = sqlx::query_scalar!(
        r#"SELECT name AS "name!" FROM exercise WHERE id = ?1"#,
        exercise
    )
    .fetch_optional(&db.pool)
    .await?
    .ok_or_else(|| format!("No exercise with id '{exercise}'."))?;
    let rows = sqlx::query!(
        r#"
SELECT s.date, p.name AS "place!", ts.load AS "load!", ts.rep AS "rep!", ts.note AS "note!",
       s.deload AS "deload!", e.name AS "effort"
FROM training_set ts
JOIN session s ON s.id = ts.session
JOIN place p ON p.id = s.place
LEFT JOIN effort e ON e.id = ts.effort
WHERE ts.exercise = ?1
  AND ts.session IN (
      SELECT ts2.session FROM training_set ts2
      WHERE ts2.exercise = ?1
      GROUP BY ts2.session
      ORDER BY MAX(julianday(ts2.date)) DESC
      LIMIT ?2
  )
ORDER BY julianday(s.date) DESC, ts.position
"#,
        exercise,
        limit
    )
    .fetch_all(&db.pool)
    .await?;

    let mut sessions: Vec<HistorySession> = Vec::new();
    for r in rows {
        let date = iso(local_date(&r.date).unwrap_or_default());
        let set = HistorySet {
            load: r.load,
            rep: r.rep,
            note: r.note,
            time: local_time(&r.date),
            effort: r.effort.clone(),
        };
        match sessions.last_mut() {
            Some(last) if last.date == date => {
                if last.effort.is_none() {
                    last.effort = r.effort;
                }
                last.sets.push(set);
            }
            _ => sessions.push(HistorySession {
                date,
                place: r.place,
                effort: r.effort,
                deload: r.deload != 0,
                sets: vec![set],
            }),
        }
    }
    Ok(History {
        exercise: name,
        sessions,
    })
}
