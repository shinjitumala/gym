//! The shapes the pages render.
//!
//! Plain data with `serde` and nothing else, so the same declarations compile
//! into the WASM half and into the server. Everything is formatted on the
//! server: a page that needs `chrono` or `sqlx` in the browser would drag both
//! into the WASM bundle for nothing.

use serde::{Deserialize, Serialize};

/// Energy and macros, in the units the log already uses.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Totals {
    pub kcal: f64,
    pub protein: f64,
    pub fat: f64,
    pub carbohydrate: f64,
}

/// One meal, joined with the food it points at.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MealRow {
    pub id: i64,
    pub date: String,
    pub time: String,
    pub food: String,
    pub amount: f64,
    pub kcal: f64,
    pub protein: f64,
    pub fat: f64,
    pub carbohydrate: f64,
    pub note: String,
}

/// A food, with its numbers for one unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FoodRow {
    pub id: i64,
    pub name: String,
    pub per_unit: Totals,
    pub note: String,
    /// Times it was eaten, for the ranking on the analysis tab.
    pub eaten: i64,
}

/// A weigh-in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeightRow {
    pub id: i64,
    pub date: String,
    pub kg: f64,
    pub bodyfat: Option<f64>,
    pub note: String,
}

/// One day of intake, with the calories split by the food that supplied them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DayRow {
    pub date: String,
    pub totals: Totals,
    /// The top contributors, largest first.
    pub split: Vec<SplitRow>,
    pub logged: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SplitRow {
    pub food: String,
    pub kcal: f64,
}

/// The trend, ready to display.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TrendView {
    /// Oldest first.
    pub points: Vec<PointRow>,
    pub per_week: Option<f64>,
    pub band: Option<f64>,
    pub have: usize,
    pub need: usize,
    pub verdict: Option<Verdict>,
    pub goal: Option<String>,
    /// The goal range in kg/week, for drawing it.
    pub goal_low: Option<f64>,
    pub goal_high: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointRow {
    pub day: i64,
    pub kg: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    On,
    Off,
    Uncertain,
}

/// The maintenance estimate, when it could be made.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaintenanceView {
    pub kcal: f64,
    pub days: usize,
    pub intake: f64,
}

/// A day whose intake the weight trend does not explain.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutlierRow {
    pub day: i64,
    pub kcal: f64,
    pub expected: f64,
    pub delta: f64,
}

/// The phase that was active.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhaseRow {
    pub kind: String,
    pub start: String,
    pub target_rate: f64,
    pub target_rep: i64,
    pub target_weight: Option<f64>,
}

/// Food · daily.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FoodDaily {
    pub today: String,
    /// The local clock when the page was rendered, `HH:MM`, for the "now"
    /// button. Sent from the server so the browser needs no clock.
    pub now: String,
    pub totals: Totals,
    pub meals: Vec<MealRow>,
    pub foods: Vec<FoodRow>,
    pub last_weight: Option<WeightRow>,
    pub trend: TrendView,
    pub maintenance: Option<MaintenanceView>,
    pub logged_days: usize,
    pub need_days: usize,
    pub outliers: Vec<OutlierRow>,
    pub phase: Option<PhaseRow>,
    pub presets: Vec<PresetRow>,
}

/// A named set of foods to record in one action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PresetRow {
    pub id: i64,
    pub name: String,
    pub items: usize,
    pub per_unit: Totals,
    /// What it is made of, for the management panel.
    pub parts: Vec<PresetPart>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PresetPart {
    pub food: i64,
    pub name: String,
    pub amount: f64,
}

/// The last few sessions of one exercise, for the recording form.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct History {
    pub exercise: String,
    pub sessions: Vec<HistorySession>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistorySession {
    pub date: String,
    pub place: String,
    pub sets: Vec<HistorySet>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistorySet {
    pub load: f64,
    pub rep: f64,
    pub note: String,
}

/// The exercises the analysis dropdown offers, and one exercise's own series.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExercisePicker {
    pub exercises: Vec<PickRow>,
    pub chosen: Option<StrengthSeries>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PickRow {
    pub id: i64,
    pub name: String,
    pub group: String,
    /// Sessions it has, so the dropdown can say what is worth picking.
    pub sessions: i64,
}

/// Food · intervention.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FoodEdit {
    pub today: String,
    pub now: String,
    pub presets: Vec<PresetRow>,
    pub meals: Vec<MealRow>,
    pub weights: Vec<WeightRow>,
    pub foods: Vec<FoodRow>,
}

/// Food · analysis.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FoodAnalysis {
    pub days: Vec<DayRow>,
    pub trend: TrendView,
    pub favourite: Vec<FoodRow>,
    pub by_calories: Vec<FoodRow>,
    pub weights: Vec<WeightRow>,
    pub logged_days: usize,
    pub span: usize,
}

/// A set, as the pages show it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SetRow {
    pub id: i64,
    pub position: i64,
    pub exercise: String,
    pub load: f64,
    pub rep: f64,
    pub note: String,
    pub superset: Option<i64>,
    pub inferred: bool,
}

/// A session with its sets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: i64,
    pub date: String,
    pub place: String,
    pub note: String,
    pub deload: bool,
    pub sets: Vec<SetRow>,
    /// Best estimated 1RM in the session, and whether it beat the exercise's
    /// best before this session.
    pub prs: Vec<PrRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PrRow {
    pub exercise: String,
    pub load: f64,
    pub rep: f64,
    pub e1rm: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaceRow {
    pub id: i64,
    pub name: String,
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExerciseRow {
    pub id: i64,
    pub name: String,
    pub note: String,
    pub video_url: Option<String>,
    pub groups: Vec<GroupRow>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GroupRow {
    pub id: i64,
    pub name: String,
    pub amount: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MuscleRow {
    pub id: i64,
    pub name: String,
}

/// Gym · daily.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GymDaily {
    pub today: String,
    pub now: String,
    pub last_set: i64,
    pub places: Vec<PlaceRow>,
    pub exercises: Vec<ExerciseRow>,
    pub sessions: Vec<SessionRow>,
    pub last_deload: Option<String>,
    pub deload_weeks: i64,
    pub weeks_since_deload: Option<i64>,
    pub phase: Option<PhaseRow>,
    pub place: Option<i64>,
}

/// Gym · intervention.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GymEdit {
    pub exercises: Vec<ExerciseRow>,
    pub places: Vec<PlaceRow>,
    pub groups: Vec<MuscleRow>,
    pub sessions: Vec<SessionRow>,
}

/// One point of a strength series.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct StrengthPoint {
    pub day: i64,
    pub e1rm: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StrengthSeries {
    pub exercise: String,
    pub group: String,
    pub points: Vec<StrengthPoint>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolumePoint {
    pub day: i64,
    pub sets: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolumeSeries {
    pub group: String,
    pub points: Vec<VolumePoint>,
}

/// Gym · analysis.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GymAnalysis {
    pub strength: Vec<StrengthSeries>,
    pub volume: Vec<VolumeSeries>,
    pub ungrouped: Vec<String>,
    pub deloads: Vec<i64>,
    pub span: usize,
}

/// The database the server is reading, for the header.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ServerInfo {
    pub db: String,
    pub remote: String,
    pub sessions: i64,
    pub sets: i64,
    pub meals: i64,
    pub weights: i64,
}
