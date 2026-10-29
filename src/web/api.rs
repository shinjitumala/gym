//! The server functions the pages call.
//!
//! `input = Json` for all of them: the default codec is URL-encoded, which
//! cannot carry a list of rows, and a form here regularly sends one (the meal
//! rows, a preset's exercises, a set of muscle groups). With the JSON body the
//! argument names survive, which is what makes a typed error like "missing
//! field `rows`" impossible to get by accident.
//!
//! Each one is a declaration the WASM half can see and a body only the server
//! compiles, which is how the browser gets data without ever holding a
//! database handle.

use crate::web::dto::*;
use leptos::prelude::*;
use server_fn::codec::Json;

#[cfg(feature = "ssr")]
fn error(e: impl std::fmt::Display) -> ServerFnError {
    ServerFnError::new(e.to_string())
}

#[cfg(feature = "ssr")]
fn config() -> Result<crate::ctx::C, ServerFnError> {
    use fpr_cli_util::Ctx;
    let path = crate::ctx::config_path().map_err(error)?;
    crate::ctx::C::new(&path).map_err(error)
}

#[cfg(feature = "ssr")]
async fn open() -> Result<crate::db::Db, ServerFnError> {
    let c = config()?;
    crate::db::Db::open(&c.cfg.db).await.map_err(error)
}

#[server(input = Json)]
pub async fn load_server_info() -> Result<ServerInfo, ServerFnError> {
    let c = config()?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::data::info(&db, &c.cfg.remote)
        .await
        .map_err(error)
}

#[server(input = Json)]
pub async fn load_food_daily() -> Result<FoodDaily, ServerFnError> {
    let db = open().await?;
    crate::web::data::food_daily(&db).await.map_err(error)
}

#[server(input = Json)]
pub async fn load_food_edit() -> Result<FoodEdit, ServerFnError> {
    let db = open().await?;
    crate::web::data::food_edit(&db).await.map_err(error)
}

#[server(input = Json)]
pub async fn load_weight_daily() -> Result<WeightDaily, ServerFnError> {
    let db = open().await?;
    crate::web::data::weight_daily(&db).await.map_err(error)
}

#[server(input = Json)]
pub async fn load_weight_edit() -> Result<WeightEdit, ServerFnError> {
    let db = open().await?;
    crate::web::data::weight_edit(&db).await.map_err(error)
}

#[server(input = Json)]
pub async fn load_weight_analysis(span: i64) -> Result<WeightAnalysis, ServerFnError> {
    let db = open().await?;
    crate::web::data::weight_analysis(&db, span)
        .await
        .map_err(error)
}

#[server(input = Json)]
pub async fn load_gym_daily() -> Result<GymDaily, ServerFnError> {
    let db = open().await?;
    crate::web::data::gym_daily(&db).await.map_err(error)
}

#[server(input = Json)]
pub async fn load_gym_edit() -> Result<GymEdit, ServerFnError> {
    let db = open().await?;
    crate::web::data::gym_edit(&db).await.map_err(error)
}

#[server(input = Json)]
pub async fn load_gym_analysis(
    span: i64,
    exercise: Option<i64>,
) -> Result<GymAnalysis, ServerFnError> {
    let db = open().await?;
    crate::web::data::gym_analysis(&db, span, exercise)
        .await
        .map_err(error)
}

#[server(input = Json)]
pub async fn load_exercise_window(
    session: i64,
    exercise: i64,
) -> Result<ExerciseWindow, ServerFnError> {
    let db = open().await?;
    crate::web::data::exercise_window(&db, session, exercise)
        .await
        .map_err(error)
}

#[server(input = Json)]
pub async fn act_add_meal(
    date: String,
    time: String,
    food: i64,
    amount: String,
    note: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let amount = crate::web::actions::num(&amount, "amount").map_err(error)?;
    crate::web::actions::add_meal(&db, &date, &time, food, amount, &note)
        .await
        .map_err(error)?;
    Ok(format!("Recorded {amount} × food #{food}."))
}

#[server(input = Json)]
pub async fn act_add_meals(
    date: String,
    time: String,
    rows: Vec<(i64, String, String)>,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let mut parsed = Vec::new();
    for (food, amount, note) in rows {
        parsed.push((
            food,
            crate::web::actions::num(&amount, "amount").map_err(error)?,
            note,
        ));
    }
    let n = crate::web::actions::add_meals(&db, &date, &time, parsed)
        .await
        .map_err(error)?;
    Ok(format!("Recorded {n} meal row(s)."))
}

#[server(input = Json)]
pub async fn act_delete_meal(id: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::delete_meal(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted meal #{id}."))
}

#[server(input = Json)]
pub async fn act_edit_meal(
    id: i64,
    date: String,
    time: String,
    food: i64,
    amount: String,
    note: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let amount = crate::web::actions::num(&amount, "amount").map_err(error)?;
    crate::web::actions::edit_meal(&db, id, &date, &time, food, amount, &note)
        .await
        .map_err(error)?;
    Ok(format!("Saved meal #{id}."))
}

#[server(input = Json)]
pub async fn act_add_food(
    name: String,
    calories: String,
    protein: String,
    fat: String,
    carbohydrate: String,
    note: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let calories = crate::web::actions::num(&calories, "calories").map_err(error)?;
    let id = crate::web::actions::add_food(
        &db,
        &name,
        calories,
        crate::web::actions::opt_num(&protein).map_err(error)?,
        crate::web::actions::opt_num(&fat).map_err(error)?,
        crate::web::actions::opt_num(&carbohydrate).map_err(error)?,
        &note,
    )
    .await
    .map_err(error)?;
    Ok(format!("Added food #{id}."))
}

#[server(input = Json)]
pub async fn act_edit_food(
    id: i64,
    name: String,
    calories: String,
    protein: String,
    fat: String,
    carbohydrate: String,
    note: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let calories = crate::web::actions::num(&calories, "calories").map_err(error)?;
    crate::web::actions::edit_food(
        &db,
        id,
        &name,
        calories,
        crate::web::actions::opt_num(&protein).map_err(error)?,
        crate::web::actions::opt_num(&fat).map_err(error)?,
        crate::web::actions::opt_num(&carbohydrate).map_err(error)?,
        &note,
    )
    .await
    .map_err(error)?;
    Ok(format!("Saved food #{id}."))
}

#[server(input = Json)]
pub async fn act_delete_food(id: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::delete_food(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted food #{id}."))
}

#[server(input = Json)]
pub async fn act_touch_preset(preset: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::touch_preset(&db, preset)
        .await
        .map_err(error)?;
    Ok(format!("Loaded preset #{preset}."))
}

#[server(input = Json)]
pub async fn act_add_preset(
    name: String,
    foods: Vec<(i64, String)>,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let mut items = Vec::new();
    for (food, amount) in foods {
        items.push((
            food,
            crate::web::actions::num(&amount, "amount").map_err(error)?,
        ));
    }
    let id = crate::web::actions::add_preset(&db, &name, items)
        .await
        .map_err(error)?;
    Ok(format!("Added preset #{id}."))
}

#[server(input = Json)]
pub async fn act_edit_preset(
    id: i64,
    name: String,
    foods: Vec<(i64, String)>,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let mut items = Vec::new();
    for (food, amount) in foods {
        items.push((
            food,
            crate::web::actions::num(&amount, "amount").map_err(error)?,
        ));
    }
    crate::web::actions::edit_preset(&db, id, &name, items)
        .await
        .map_err(error)?;
    Ok(format!("Saved preset #{id}."))
}

#[server(input = Json)]
pub async fn act_delete_preset(id: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::delete_preset(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted preset #{id}."))
}

#[server(input = Json)]
pub async fn act_add_weight(
    date: String,
    time: String,
    kg: String,
    bodyfat: String,
    note: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let kg = crate::web::actions::num(&kg, "weight").map_err(error)?;
    crate::web::actions::add_weight(
        &db,
        &date,
        &time,
        kg,
        crate::web::actions::opt_num(&bodyfat).map_err(error)?,
        &note,
    )
    .await
    .map_err(error)?;
    Ok(format!("Recorded {kg} kg."))
}

#[server(input = Json)]
pub async fn act_edit_weight(
    id: i64,
    date: String,
    time: String,
    kg: String,
    bodyfat: String,
    note: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let kg = crate::web::actions::num(&kg, "weight").map_err(error)?;
    crate::web::actions::edit_weight(
        &db,
        id,
        &date,
        &time,
        kg,
        crate::web::actions::opt_num(&bodyfat).map_err(error)?,
        &note,
    )
    .await
    .map_err(error)?;
    Ok(format!("Saved weigh-in #{id}."))
}

#[server(input = Json)]
pub async fn act_delete_weight(id: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::delete_weight(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted weigh-in #{id}."))
}

#[server(input = Json)]
pub async fn act_start_session(
    place: i64,
    date: String,
    time: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let id = crate::web::actions::start_session(&db, place, &date, &time)
        .await
        .map_err(error)?;
    Ok(id.to_string())
}

#[server(input = Json)]
pub async fn act_add_exercise(session: i64, exercise: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::add_exercise(&db, session, exercise)
        .await
        .map_err(error)?;
    Ok("Added to the checklist.".to_string())
}

#[server(input = Json)]
pub async fn act_add_named_exercise(session: i64, name: String) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::add_named_exercise(&db, session, &name)
        .await
        .map_err(error)?;
    Ok(format!("Added '{}' to the checklist.", name.trim()))
}

#[server(input = Json)]
pub async fn act_apply_exercise_preset(session: i64, preset: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    let n = crate::web::actions::apply_exercise_preset(&db, session, preset)
        .await
        .map_err(error)?;
    Ok(format!("Added {n} exercise(s) to the checklist."))
}

#[server(input = Json)]
pub async fn act_log_set(
    session: i64,
    exercise: i64,
    load: String,
    rep: String,
    effort: String,
    note: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let load = crate::web::actions::num(&load, "load").map_err(error)?;
    let rep = crate::web::actions::num(&rep, "rep").map_err(error)?;
    let id = crate::web::actions::log_set(&db, session, exercise, load, rep, &effort, &note)
        .await
        .map_err(error)?;
    Ok(format!("Recorded set #{id}: {load} kg × {rep}."))
}

#[server(input = Json)]
pub async fn act_end_session(session: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::end_session(&db, session)
        .await
        .map_err(error)
}

#[server(input = Json)]
pub async fn act_set_deload(id: i64, deload: bool) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::set_deload(&db, id, deload)
        .await
        .map_err(error)?;
    Ok(format!(
        "Session #{id} is {}a deload.",
        if deload { "" } else { "no longer " }
    ))
}

#[server(input = Json)]
pub async fn act_edit_set(
    id: i64,
    load: String,
    rep: String,
    effort: String,
    note: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let load = crate::web::actions::num(&load, "load").map_err(error)?;
    let rep = crate::web::actions::num(&rep, "rep").map_err(error)?;
    crate::web::actions::edit_set(&db, id, load, rep, &effort, &note)
        .await
        .map_err(error)?;
    Ok(format!("Saved set #{id}."))
}

#[server(input = Json)]
pub async fn act_delete_set(id: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::delete_set(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted set #{id}."))
}

#[server(input = Json)]
pub async fn act_set_phase(
    kind: String,
    start: String,
    rate: String,
    rep: String,
    weight: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let rate = crate::web::actions::num(&rate, "kg/month").map_err(error)?;
    let rep = crate::web::actions::num(&rep, "reps").map_err(error)?;
    crate::web::actions::set_phase(
        &db,
        &kind,
        &start,
        rate,
        rep as i64,
        crate::web::actions::opt_num(&weight).map_err(error)?,
    )
    .await
    .map_err(error)?;
    Ok(format!("Started a {kind} on {start}."))
}

#[server(input = Json)]
pub async fn act_add_exercise_preset(
    name: String,
    exercises: Vec<(i64, String)>,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let items = preset_items(exercises)?;
    let id = crate::web::actions::add_exercise_preset(&db, &name, items)
        .await
        .map_err(error)?;
    Ok(format!("Added gym preset #{id}."))
}

#[server(input = Json)]
pub async fn act_edit_exercise_preset(
    id: i64,
    name: String,
    exercises: Vec<(i64, String)>,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let items = preset_items(exercises)?;
    crate::web::actions::edit_exercise_preset(&db, id, &name, items)
        .await
        .map_err(error)?;
    Ok(format!("Saved gym preset #{id}."))
}

/// A preset's rows as the form sends them: the exercise, and the sets typed
/// beside it.
#[cfg(feature = "ssr")]
fn preset_items(exercises: Vec<(i64, String)>) -> Result<Vec<(i64, i64)>, ServerFnError> {
    let mut items = Vec::new();
    for (exercise, sets) in exercises {
        items.push((
            exercise,
            crate::web::actions::count(&sets, "sets").map_err(error)?,
        ));
    }
    Ok(items)
}

#[server(input = Json)]
pub async fn act_delete_exercise_preset(id: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::delete_exercise_preset(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted gym preset #{id}."))
}

#[server(input = Json)]
pub async fn act_add_split(
    name: String,
    active: bool,
    days: Vec<(i64, String)>,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let days = split_days(days)?;
    let id = crate::web::actions::add_split(&db, &name, active, days)
        .await
        .map_err(error)?;
    Ok(format!("Added split #{id}."))
}

#[server(input = Json)]
pub async fn act_edit_split(
    id: i64,
    name: String,
    active: bool,
    days: Vec<(i64, String)>,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let days = split_days(days)?;
    crate::web::actions::edit_split(&db, id, &name, active, days)
        .await
        .map_err(error)?;
    Ok(format!("Saved split #{id}."))
}

/// A split's day rows as the form sends them: the weekday and the preset id its
/// dropdown held, empty for the rest day it defaults to.
#[cfg(feature = "ssr")]
fn split_days(days: Vec<(i64, String)>) -> Result<Vec<(i64, Option<i64>)>, ServerFnError> {
    let mut out = Vec::new();
    for (weekday, preset) in days {
        let preset = preset.trim();
        let preset = if preset.is_empty() {
            None
        } else {
            Some(
                preset
                    .parse::<i64>()
                    .map_err(|_| ServerFnError::new(format!("'{}' is not a preset.", preset)))?,
            )
        };
        out.push((weekday, preset));
    }
    Ok(out)
}

#[server(input = Json)]
pub async fn act_delete_split(id: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::delete_split(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted split #{id}."))
}

#[server(input = Json)]
pub async fn act_edit_exercise(
    id: i64,
    name: String,
    note: String,
    video_url: String,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::edit_exercise(
        &db,
        id,
        &name,
        &note,
        (!video_url.trim().is_empty()).then_some(video_url.as_str()),
    )
    .await
    .map_err(error)?;
    Ok(format!("Saved exercise #{id}."))
}

#[server(input = Json)]
pub async fn act_set_groups(
    exercise: i64,
    groups: Vec<(i64, String)>,
) -> Result<String, ServerFnError> {
    let db = open().await?;
    let mut parsed = Vec::new();
    for (group, amount) in groups {
        parsed.push((
            group,
            crate::web::actions::num(&amount, "amount").map_err(error)?,
        ));
    }
    crate::web::actions::set_groups(&db, exercise, parsed)
        .await
        .map_err(error)?;
    Ok(format!("Saved the muscle groups of exercise #{exercise}."))
}

#[server(input = Json)]
pub async fn act_rename_exercise(id: i64, name: String) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::rename_exercise(&db, id, &name)
        .await
        .map_err(error)?;
    Ok(format!("Renamed exercise #{id}."))
}

#[server(input = Json)]
pub async fn act_merge_exercises(from: i64, to: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::merge_exercises(&db, from, to)
        .await
        .map_err(error)?;
    Ok(format!("Merged exercise #{from} into #{to}."))
}

#[server(input = Json)]
pub async fn act_add_place(name: String, note: String) -> Result<String, ServerFnError> {
    let db = open().await?;
    let id = crate::web::actions::add_place(&db, &name, &note)
        .await
        .map_err(error)?;
    Ok(format!("Added place #{id}."))
}

#[server(input = Json)]
pub async fn act_edit_place(id: i64, name: String, note: String) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::rename_place(&db, id, &name, &note)
        .await
        .map_err(error)?;
    Ok(format!("Saved place #{id}."))
}

#[server(input = Json)]
pub async fn act_delete_place(id: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::delete_place(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted place #{id}."))
}

#[server(input = Json)]
pub async fn act_add_group(name: String, note: String) -> Result<String, ServerFnError> {
    let db = open().await?;
    let id = crate::web::actions::add_group(&db, &name, &note)
        .await
        .map_err(error)?;
    Ok(format!("Added muscle group #{id}."))
}

#[server(input = Json)]
pub async fn act_delete_group(id: i64) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::delete_group(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted muscle group #{id}."))
}

#[server(input = Json)]
pub async fn act_set_setting(key: String, value: String) -> Result<String, ServerFnError> {
    let db = open().await?;
    crate::web::actions::set_setting(&db, &key, &value)
        .await
        .map_err(error)?;
    Ok(format!("{key} is now {value}."))
}
