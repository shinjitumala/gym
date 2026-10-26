//! The shapes the pages render.
//!
//! Plain data with `serde` and nothing else, so the same declarations compile
//! into the WASM half and into the server. Everything is formatted on the
//! server: a page that needs `chrono` or `sqlx` in the browser would drag both
//! into the WASM bundle for nothing.
//!
//! Recency is decided here too, as `last_used` on each row: a picker shows the
//! list the server sorted, so the browser never re-sorts a list it cannot
//! query.

use serde::{Deserialize, Serialize};

/// Energy and macros, in the units the log already uses.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Totals {
    pub kcal: f64,
    pub protein: f64,
    pub fat: f64,
    pub carbohydrate: f64,
}

impl Totals {
    pub fn plus(&self, other: &Totals) -> Totals {
        Totals {
            kcal: self.kcal + other.kcal,
            protein: self.protein + other.protein,
            fat: self.fat + other.fat,
            carbohydrate: self.carbohydrate + other.carbohydrate,
        }
    }
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
    /// Times it was eaten, shown beside the food on the edit page.
    pub eaten: i64,
    /// The last day it was eaten, `YYYY-MM-DD`, or `None` if never.
    pub last_used: Option<String>,
}

/// A weigh-in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeightRow {
    pub id: i64,
    pub date: String,
    pub time: String,
    pub kg: f64,
    pub bodyfat: Option<f64>,
    pub note: String,
}

/// One day of intake, with the calories split by the food that supplied them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DayRow {
    pub date: String,
    pub day: i64,
    pub totals: Totals,
    /// The top contributors, largest first.
    pub split: Vec<SplitRow>,
    pub logged: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SplitRow {
    pub food: String,
    pub kcal: f64,
    pub protein: f64,
    pub fat: f64,
    pub carbohydrate: f64,
}

/// The trend, ready to display.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TrendView {
    /// Oldest first: the weigh-ins inside the slope's trailing window.
    pub points: Vec<PointRow>,
    /// How many days that window is, so the page can say which window it means.
    pub window_days: i64,
    /// The fitted line, at the same x values as `points`.
    pub fit: Vec<PointRow>,
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

/// A stretch of the log trained in one phase, in day numbers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhaseSpan {
    pub kind: String,
    pub from: i64,
    pub to: i64,
}

/// A stretch of days a chart marks, such as the week a deload fell in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub from: i64,
    pub to: i64,
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
    /// Ordered by last use, the most recent first.
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

/// An effort level the exercise window offers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EffortRow {
    pub id: i64,
    pub name: String,
    /// What the level means, in reps from failure.
    pub hint: String,
    /// A smaller rank is closer to failure, so a smaller rank is harder.
    pub rank: i64,
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
    pub effort: Option<String>,
    /// A deload session is shown, but not used as the numbers to beat.
    pub deload: bool,
    pub sets: Vec<HistorySet>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistorySet {
    pub load: f64,
    pub rep: f64,
    pub note: String,
    pub time: String,
    pub effort: Option<String>,
}

/// The exercises the analysis dropdown offers, and one exercise's own series.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExercisePicker {
    pub exercises: Vec<PickRow>,
    pub chosen: Option<AnalysisSeries>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PickRow {
    pub id: i64,
    pub name: String,
    pub group: String,
    /// Sessions it has, so the dropdown can say what is worth picking.
    pub sessions: i64,
}

/// Food · edit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FoodEdit {
    pub today: String,
    pub now: String,
    pub presets: Vec<PresetRow>,
    pub meals: Vec<MealRow>,
    pub foods: Vec<FoodRow>,
}

/// Weight · daily.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WeightDaily {
    pub today: String,
    pub now: String,
    /// The last handful of weigh-ins, newest first.
    pub weights: Vec<WeightRow>,
    pub last_weight: Option<WeightRow>,
    pub trend: TrendView,
    pub phase: Option<PhaseRow>,
    pub outliers: Vec<OutlierRow>,
}

/// Weight · edit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WeightEdit {
    pub today: String,
    pub now: String,
    pub weights: Vec<WeightRow>,
}

/// Weight · analyse: the trend, the phases it ran through, and the intake bars
/// underneath it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WeightAnalysis {
    /// Every weigh-in in the chosen range, oldest first.
    ///
    /// The line is the range's observations, not the trend's trailing window:
    /// the slope is judged over the last few weeks, but the chart is asked about
    /// the whole range, so a log whose last weigh-in predates the slope window
    /// still has a line.
    pub line: Vec<PointRow>,
    pub trend: TrendView,
    /// The newest weigh-in anywhere in the log, so a window that holds none can
    /// say when the last one was rather than showing an unexplained blank.
    pub last_weight: Option<WeightRow>,
    pub phases: Vec<PhaseSpan>,
    /// Body fat percentage, oldest first, when it was measured.
    pub bodyfat: Vec<PointRow>,
    pub days: Vec<DayRow>,
    pub weights: Vec<WeightRow>,
    /// Every food in the range and what it contributed, complete rather than
    /// cut to a day's top few, for the ranking under the bars.
    pub ranking: Vec<SplitRow>,
    /// The deload weeks in the window, so the line can shade them.
    pub deloads: Vec<Span>,
    pub span: usize,
    pub logged_days: usize,
    pub maintenance: Option<MaintenanceView>,
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
    /// The effort's name, when one was recorded.
    pub effort: Option<String>,
    /// The effort's rank: smaller is harder.
    pub effort_rank: Option<i64>,
    /// The wall clock time it was recorded, `HH:MM`.
    pub time: String,
}

/// A session with its sets.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionRow {
    pub id: i64,
    pub date: String,
    pub time: String,
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
    /// The last day a session happened here, `YYYY-MM-DD`.
    pub last_used: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExerciseRow {
    pub id: i64,
    pub name: String,
    pub note: String,
    pub video_url: Option<String>,
    pub groups: Vec<GroupRow>,
    /// The last day it was trained, `YYYY-MM-DD`.
    pub last_used: Option<String>,
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

/// One line of a session's checklist: an exercise the session intends to do,
/// and how many sets it has recorded so far.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChecklistRow {
    pub exercise: i64,
    pub name: String,
    pub last_used: Option<String>,
    /// Sets recorded for this exercise in this session.
    pub sets: i64,
    /// The effort of the last set recorded for it, for the default.
    pub last_effort: Option<i64>,
}

/// The session being recorded.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LiveSession {
    pub id: i64,
    pub date: String,
    pub time: String,
    pub place: String,
    pub deload: bool,
    pub note: String,
    /// The checklist, in the order the exercises were added.
    pub list: Vec<ChecklistRow>,
    pub sets: Vec<SetRow>,
}

/// A named list of exercises: the gym's preset, applied to a session as its
/// checklist.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GymPreset {
    pub id: i64,
    pub name: String,
    pub items: Vec<PresetExercise>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PresetExercise {
    pub exercise: i64,
    pub name: String,
}

/// Gym · daily.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GymDaily {
    pub today: String,
    pub now: String,
    /// Ordered by the last session at each place, most recent first.
    pub places: Vec<PlaceRow>,
    /// Ordered by the last set of each exercise, most recent first.
    pub exercises: Vec<ExerciseRow>,
    pub presets: Vec<GymPreset>,
    pub efforts: Vec<EffortRow>,
    pub open: Option<LiveSession>,
    pub last_deload: Option<String>,
    pub deload_weeks: i64,
    pub weeks_since_deload: Option<i64>,
    pub phase: Option<PhaseRow>,
}

/// One exercise, while it is being recorded: its sets in this session, the
/// numbers to beat, and what to attempt next.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ExerciseWindow {
    pub exercise: i64,
    pub name: String,
    pub group: String,
    pub target_rep: i64,
    pub deload: bool,
    /// This session's sets of this exercise.
    pub sets: Vec<SetRow>,
    /// The last load used for it here, which the first row defaults to.
    pub last_load: Option<f64>,
    /// The last effort used for it, defaulted in the dropdown.
    pub last_effort: Option<i64>,
    pub advice: Option<String>,
    pub history: Vec<HistorySession>,
}

/// Gym · edit.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GymEdit {
    pub exercises: Vec<ExerciseRow>,
    pub places: Vec<PlaceRow>,
    pub groups: Vec<MuscleRow>,
    pub presets: Vec<GymPreset>,
    pub efforts: Vec<EffortRow>,
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

/// Sets in a week.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolumePoint {
    pub day: i64,
    pub sets: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolumeSeries {
    pub group: String,
    pub points: Vec<VolumePoint>,
}

/// A company of sets per week for one exercise, drawn beside the chosen one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlapSeries {
    pub exercise: String,
    pub sets: Vec<VolumePoint>,
}

/// Gym · analyse: one exercise's strength, its sets per week, and the sets per
/// week of the exercises that share its main muscle group.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AnalysisSeries {
    pub exercise: String,
    pub group: String,
    pub strength: Vec<StrengthPoint>,
    pub sets: Vec<VolumePoint>,
    pub overlaps: Vec<OverlapSeries>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GymAnalysis {
    pub exercises: Vec<PickRow>,
    pub chosen: Option<AnalysisSeries>,
    pub span: usize,
    /// The deload weeks in the window, so the chart can shade them.
    pub deloads: Vec<Span>,
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
