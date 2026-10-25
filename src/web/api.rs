//! The server functions the pages call.
//!
//! Each one is a declaration the WASM half can see and a body only the server
//! compiles, which is how the browser gets data without ever holding a
//! database handle. They are read-only for now; the write calls arrive with the
//! daily and intervention forms.

use crate::web::dto::*;
use leptos::prelude::*;

#[cfg(feature = "ssr")]
fn error(e: impl std::fmt::Display) -> ServerFnError {
    ServerFnError::new(e.to_string())
}

#[server]
pub async fn load_server_info() -> Result<ServerInfo, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::data::info(&db, &c.cfg.remote)
        .await
        .map_err(error)
}

#[server]
pub async fn load_food_daily() -> Result<FoodDaily, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::data::food_daily(&db).await.map_err(error)
}

#[server]
pub async fn load_food_edit() -> Result<FoodEdit, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::data::food_edit(&db).await.map_err(error)
}

#[server]
pub async fn load_food_analysis() -> Result<FoodAnalysis, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::data::food_analysis(&db).await.map_err(error)
}

#[server]
pub async fn load_gym_daily() -> Result<GymDaily, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::data::gym_daily(&db).await.map_err(error)
}

#[server]
pub async fn load_gym_edit() -> Result<GymEdit, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::data::gym_edit(&db).await.map_err(error)
}

#[server]
pub async fn load_gym_analysis() -> Result<GymAnalysis, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::data::gym_analysis(&db).await.map_err(error)
}

/// Every write goes through one of these. Each takes the form's fields as
/// strings, validates on the server, and returns the error text the form shows.
/// The date and time fields default to "now" on the page, so the common case is
/// one click.

#[server]
pub async fn act_add_meal(
    date: String,
    time: String,
    food: i64,
    amount: String,
    note: String,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let amount = crate::web::actions::num(&amount, "amount").map_err(error)?;
    crate::web::actions::add_meal(&db, &date, &time, food, amount, &note)
        .await
        .map_err(error)?;
    Ok(format!("Recorded {amount} × food #{food}."))
}

#[server]
pub async fn act_delete_meal(id: i64) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::delete_meal(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted meal #{id}."))
}

#[server]
pub async fn act_edit_meal(
    id: i64,
    date: String,
    time: String,
    food: i64,
    amount: String,
    note: String,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let amount = crate::web::actions::num(&amount, "amount").map_err(error)?;
    crate::web::actions::edit_meal(&db, id, &date, &time, food, amount, &note)
        .await
        .map_err(error)?;
    Ok(format!("Updated meal #{id}."))
}

#[server]
pub async fn act_add_weight(
    date: String,
    kg: String,
    bodyfat: String,
    note: String,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let kg = crate::web::actions::num(&kg, "weight").map_err(error)?;
    let bodyfat = crate::web::actions::opt_num(&bodyfat).map_err(error)?;
    crate::web::actions::add_weight(&db, &date, kg, bodyfat, &note)
        .await
        .map_err(error)?;
    Ok(format!("Weighed in at {kg} kg."))
}

#[server]
pub async fn act_edit_weight(
    id: i64,
    date: String,
    kg: String,
    bodyfat: String,
    note: String,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let kg = crate::web::actions::num(&kg, "weight").map_err(error)?;
    let bodyfat = crate::web::actions::opt_num(&bodyfat).map_err(error)?;
    crate::web::actions::edit_weight(&db, id, &date, kg, bodyfat, &note)
        .await
        .map_err(error)?;
    Ok(format!("Updated weigh-in #{id}."))
}

#[server]
pub async fn act_delete_weight(id: i64) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::delete_weight(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted weigh-in #{id}."))
}

#[server]
pub async fn act_add_food(
    name: String,
    calories: String,
    protein: String,
    fat: String,
    carbohydrate: String,
    note: String,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let calories = crate::web::actions::num(&calories, "calories").map_err(error)?;
    let protein = crate::web::actions::opt_num(&protein).map_err(error)?;
    let fat = crate::web::actions::opt_num(&fat).map_err(error)?;
    let carbohydrate = crate::web::actions::opt_num(&carbohydrate).map_err(error)?;
    let id = crate::web::actions::add_food(&db, &name, calories, protein, fat, carbohydrate, &note)
        .await
        .map_err(error)?;
    Ok(format!("Added food #{id}: {name}."))
}

#[server]
pub async fn act_edit_food(
    id: i64,
    name: String,
    calories: String,
    protein: String,
    fat: String,
    carbohydrate: String,
    note: String,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let calories = crate::web::actions::num(&calories, "calories").map_err(error)?;
    let protein = crate::web::actions::opt_num(&protein).map_err(error)?;
    let fat = crate::web::actions::opt_num(&fat).map_err(error)?;
    let carbohydrate = crate::web::actions::opt_num(&carbohydrate).map_err(error)?;
    crate::web::actions::edit_food(&db, id, &name, calories, protein, fat, carbohydrate, &note)
        .await
        .map_err(error)?;
    Ok(format!("Updated food #{id}."))
}

#[server]
pub async fn act_delete_food(id: i64) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::delete_food(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted food #{id}."))
}

#[server]
pub async fn act_apply_preset(
    preset: i64,
    date: String,
    time: String,
    multiplier: String,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let multiplier = if multiplier.trim().is_empty() {
        1.0
    } else {
        crate::web::actions::num(&multiplier, "multiplier").map_err(error)?
    };
    let n = crate::web::actions::apply_preset(&db, preset, &date, &time, multiplier)
        .await
        .map_err(error)?;
    Ok(format!("Recorded {n} meal(s) from the preset."))
}

#[server]
pub async fn act_add_preset(
    name: String,
    foods: Vec<(i64, String)>,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
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

#[server]
pub async fn act_delete_preset(id: i64) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::delete_preset(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted preset #{id}."))
}

#[server]
pub async fn act_add_session(
    place: i64,
    date: String,
    time: String,
    note: String,
    deload: bool,
    sets: Vec<(i64, String, String, String)>,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let mut out = Vec::new();
    for (exercise, load, rep, note) in sets {
        out.push(crate::web::actions::NewSet {
            exercise,
            load: crate::web::actions::num(&load, "load").map_err(error)?,
            rep: crate::web::actions::num(&rep, "rep").map_err(error)?,
            note,
        });
    }
    let id = crate::web::actions::add_session(
        &db,
        crate::web::actions::NewSession {
            place,
            date,
            time,
            note,
            deload,
            sets: out,
        },
    )
    .await
    .map_err(error)?;
    Ok(format!("Recorded session #{id}."))
}

#[server]
pub async fn act_set_deload(id: i64, deload: bool) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::set_deload(&db, id, deload)
        .await
        .map_err(error)?;
    Ok(if deload {
        format!("Session #{id} is a deload.")
    } else {
        format!("Session #{id} is no longer a deload.")
    })
}

#[server]
pub async fn act_edit_set(
    id: i64,
    load: String,
    rep: String,
    note: String,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let load = crate::web::actions::num(&load, "load").map_err(error)?;
    let rep = crate::web::actions::num(&rep, "rep").map_err(error)?;
    crate::web::actions::edit_set(&db, id, load, rep, &note)
        .await
        .map_err(error)?;
    Ok(format!("Updated set #{id}."))
}

#[server]
pub async fn act_delete_set(id: i64) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::delete_set(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted set #{id}."))
}

#[server]
pub async fn act_edit_exercise(
    id: i64,
    name: String,
    note: String,
    video_url: String,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::edit_exercise(&db, id, &name, &note, Some(&video_url))
        .await
        .map_err(error)?;
    Ok(format!("Updated exercise #{id}."))
}

#[server]
pub async fn act_set_groups(
    exercise: i64,
    groups: Vec<(i64, String)>,
) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let mut out = Vec::new();
    for (g, amount) in groups {
        out.push((
            g,
            crate::web::actions::num(&amount, "amount").map_err(error)?,
        ));
    }
    crate::web::actions::set_groups(&db, exercise, out)
        .await
        .map_err(error)?;
    Ok(format!("Set the muscle groups of exercise #{exercise}."))
}

#[server]
pub async fn act_merge_exercises(from: i64, to: i64) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::merge_exercises(&db, from, to)
        .await
        .map_err(error)?;
    Ok(format!("Merged exercise #{from} into #{to}."))
}

#[server]
pub async fn act_add_place(name: String, note: String) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let id = crate::web::actions::add_place(&db, &name, &note)
        .await
        .map_err(error)?;
    Ok(format!("Added place #{id}."))
}

#[server]
pub async fn act_edit_place(id: i64, name: String, note: String) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::rename_place(&db, id, &name, &note)
        .await
        .map_err(error)?;
    Ok(format!("Updated place #{id}."))
}

#[server]
pub async fn act_delete_place(id: i64) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::delete_place(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted place #{id}."))
}

#[server]
pub async fn act_add_group(name: String, note: String) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    let id = crate::web::actions::add_group(&db, &name, &note)
        .await
        .map_err(error)?;
    Ok(format!("Added muscle group #{id}."))
}

#[server]
pub async fn act_delete_group(id: i64) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::delete_group(&db, id)
        .await
        .map_err(error)?;
    Ok(format!("Deleted muscle group #{id}."))
}

#[server]
pub async fn act_set_setting(key: String, value: String) -> Result<String, ServerFnError> {
    let c = crate::ctx::C::new().map_err(error)?;
    let db = crate::db::Db::open(&c.cfg.db).await.map_err(error)?;
    crate::web::actions::set_setting(&db, &key, &value)
        .await
        .map_err(error)?;
    Ok(format!("Set {key} = {value}."))
}
