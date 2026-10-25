//! The six tabs.
//!
//! Each page is one `Resource` against a server function, a `Suspense`, and a
//! view that takes the finished data. Nothing here computes a statistic: the
//! numbers arrive already decided, which is what keeps the analysis honest and
//! the WASM bundle small.

use crate::web::charts;
use crate::web::dto::*;
use crate::web::ui::*;
use leptos::prelude::*;

fn verdict_badge(v: Option<Verdict>) -> impl IntoView {
    let (class, text) = match v {
        Some(Verdict::On) => ("ok", "on goal"),
        Some(Verdict::Off) => ("bad", "off goal"),
        Some(Verdict::Uncertain) => ("warn", "too noisy to say"),
        None => ("dim", "no phase, or no trend"),
    };
    view! { <span class=class>{text}</span> }
}

fn trend_block(t: &TrendView) -> impl IntoView {
    let slope = match (t.per_week, t.band) {
        (Some(s), Some(b)) => format!("{s:+.2} ± {b:.2}"),
        _ => "—".to_string(),
    };
    let points: Vec<(i64, f64)> = t.points.iter().map(|p| (p.day, p.kg)).collect();
    let missing = t.need.saturating_sub(t.have);
    view! {
        <div class="grid two">
            <div class="row">
                <Kpi label="trend" value=slope unit="kg/week" />
                {verdict_badge(t.verdict)}
            </div>
            <div class="row">
                {match t.goal.clone() {
                    Some(g) => view! { <span class="dim">{g}</span> }.into_any(),
                    None => view! { <span class="dim">"no phase set"</span> }.into_any(),
                }}
            </div>
        </div>
        {if points.len() < 2 {
            view! {
                <Hint text=format!(
                    "{} weigh-in(s) in the window: a line needs two, a slope needs {}.",
                    points.len(),
                    t.need,
                ) />
            }
                .into_any()
        } else {
            view! { <charts::Line points=points unit="kg" /> }.into_any()
        }}
        {if missing > 0 {
            view! {
                <Hint text=format!(
                    "{missing} more weigh-in(s) needed in the window before a slope is shown ({} of {}).",
                    t.have,
                    t.need,
                ) />
            }
                .into_any()
        } else {
            ().into_any()
        }}
    }
}

// ---------------------------------------------------------------- food daily

#[component]
pub fn FoodDailyPage() -> impl IntoView {
    // Bumped after a form saves, so the page shows the row it just wrote.
    let (tick, set_tick) = signal(0u32);
    let saved = Callback::new(move |()| set_tick.update(|t| *t += 1));
    let data = Resource::new(
        move || tick.get(),
        |_| async { crate::web::api::load_food_daily().await },
    );
    view! {
        <Page title="gym" here="/food">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <FoodDailyView d=d saved=saved /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn FoodDailyView(d: FoodDaily, saved: Callback<()>) -> impl IntoView {
    let totals = d.totals.clone();
    let today_a = d.today.clone();
    let today_b = d.today.clone();
    let today_c = d.today.clone();
    let maintenance = d.maintenance;
    let today_kcal = totals.kcal;
    view! {
        <Section title=format!("Today · {}", d.today)>
            <div class="kpis">
                <Kpi label="calories" value=kcal(totals.kcal) unit="kcal" />
                <Kpi label="protein" value=grams(totals.protein) unit="g" />
                <Kpi label="fat" value=grams(totals.fat) unit="g" />
                <Kpi label="carbohydrate" value=grams(totals.carbohydrate) unit="g" />
            </div>
            {match maintenance {
                Some(m) => {
                    let diff = today_kcal - m.kcal;
                    let klass = if diff >= 0.0 { "ok" } else { "bad" };
                    view! {
                        <div class="row">
                            <span class="dim">
                                "maintenance "
                                <span class="ok">{kcal(m.kcal)}</span>
                                " kcal from "
                                {m.days}
                                " logged day(s); today is "
                                <span class=klass>{format!("{diff:+.0}")}</span>
                                " kcal against it"
                            </span>
                        </div>
                    }
                        .into_any()
                }
                None => {
                    view! {
                        <Hint text=format!(
                            "Maintenance is withheld: {}/{} logged days in the window. A day needs {} kcal to count.",
                            d.logged_days,
                            d.need_days,
                            crate::stats::LOGGED_DAY_KCAL as i64,
                        ) />
                    }
                        .into_any()
                }
            }}
        </Section>

        <Section title="Record a meal">
            <AddMeal foods=d.foods.clone() today=today_a saved=saved />
        </Section>

        {if d.presets.is_empty() {
            ().into_any()
        } else {
            view! {
                <Section title="Presets">
                    <ApplyPreset presets=d.presets.clone() today=today_b saved=saved />
                </Section>
            }
                .into_any()
        }}

        <Section title="Meals">
            {if d.meals.is_empty() {
                view! { <Hint text="Nothing logged today." /> }.into_any()
            } else {
                view! { <MealTable meals=d.meals.clone() /> }.into_any()
            }}
        </Section>

        <Section title="Record a weigh-in">
            <AddWeight today=today_c saved=saved />
        </Section>

        <Section title="Weight">
            {match d.last_weight.clone() {
                Some(w) => {
                    view! {
                        <div class="row">
                            <Kpi label="last weigh-in" value=kg(w.kg) unit="kg" />
                            <Kpi label="body fat" value=opt(w.bodyfat, one) unit="%" />
                            <span class="dim">{format!("{} · {}", w.date, w.note)}</span>
                        </div>
                    }
                        .into_any()
                }
                None => view! { <Hint text="No weigh-ins yet." /> }.into_any(),
            }}
            {trend_block(&d.trend)}
        </Section>

        {if d.outliers.is_empty() {
            ().into_any()
        } else {
            view! {
                <Section title="Days the trend does not explain">
                    <Hint text="Days whose intake sits far from what the weight trend implies. A day, never a food: without a controlled repeat a mis-valued food is not identifiable." />
                    <table>
                        <thead>
                            <tr>
                                <th>"day"</th>
                                <th class="num">"intake"</th>
                                <th class="num">"implied"</th>
                                <th class="num">"diff"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {d.outliers
                                .iter()
                                .map(|o| {
                                    view! {
                                        <tr>
                                            <td>{day_label(o.day)}</td>
                                            <td class="num">{kcal(o.kcal)}</td>
                                            <td class="num">{kcal(o.expected)}</td>
                                            <td class="num">{format!("{:+.0}", o.delta)}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                </Section>
            }
                .into_any()
        }}
    }
}

#[component]
fn MealTable(meals: Vec<MealRow>) -> impl IntoView {
    view! {
        <table>
            <thead>
                <tr>
                    <th>"time"</th>
                    <th>"food"</th>
                    <th class="num">"amount"</th>
                    <th class="num">"kcal"</th>
                    <th class="num">"protein"</th>
                    <th class="num">"fat"</th>
                    <th class="num">"carbs"</th>
                    <th>"note"</th>
                </tr>
            </thead>
            <tbody>
                {meals
                    .into_iter()
                    .map(|m| {
                        view! {
                            <tr>
                                <td class="dim">{m.time}</td>
                                <td>{m.food}</td>
                                <td class="num">{one(m.amount)}</td>
                                <td class="num">{kcal(m.kcal)}</td>
                                <td class="num">{grams(m.protein)}</td>
                                <td class="num">{grams(m.fat)}</td>
                                <td class="num">{grams(m.carbohydrate)}</td>
                                <td class="dim wrap">{m.note}</td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

// ----------------------------------------------------------------- food edit

#[component]
pub fn FoodEditPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| async { crate::web::api::load_food_edit().await });
    view! {
        <Page title="gym" here="/food/edit">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <FoodEditView d=d /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn FoodEditView(d: FoodEdit) -> impl IntoView {
    view! {
        <Section title="Meals · last 120 days">
            {if d.meals.is_empty() {
                view! { <Hint text="Nothing logged." /> }.into_any()
            } else {
                view! {
                    <table>
                        <thead>
                            <tr>
                                <th>"date"</th>
                                <th>"time"</th>
                                <th>"food"</th>
                                <th class="num">"amount"</th>
                                <th class="num">"kcal"</th>
                                <th>"note"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {d.meals
                                .iter()
                                .map(|m| {
                                    view! {
                                        <tr>
                                            <td class="dim">{m.date.clone()}</td>
                                            <td class="dim">{m.time.clone()}</td>
                                            <td>{m.food.clone()}</td>
                                            <td class="num">{one(m.amount)}</td>
                                            <td class="num">{kcal(m.kcal)}</td>
                                            <td class="dim wrap">{m.note.clone()}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                }
                    .into_any()
            }}
        </Section>

        <Section title="Weigh-ins · last 120 days">
            <table>
                <thead>
                    <tr>
                        <th>"date"</th>
                        <th class="num">"kg"</th>
                        <th class="num">"body fat"</th>
                        <th>"note"</th>
                    </tr>
                </thead>
                <tbody>
                    {d.weights
                        .iter()
                        .map(|w| {
                            view! {
                                <tr>
                                    <td class="dim">{w.date.clone()}</td>
                                    <td class="num">{kg(w.kg)}</td>
                                    <td class="num">{opt(w.bodyfat, one)}</td>
                                    <td class="dim wrap">{w.note.clone()}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </Section>

        <Section title="Foods">
            <table>
                <thead>
                    <tr>
                        <th>"food"</th>
                        <th class="num">"kcal"</th>
                        <th class="num">"protein"</th>
                        <th class="num">"fat"</th>
                        <th class="num">"carbs"</th>
                        <th class="num">"eaten"</th>
                        <th>"note"</th>
                    </tr>
                </thead>
                <tbody>
                    {d.foods
                        .iter()
                        .map(|f| {
                            view! {
                                <tr>
                                    <td>{f.name.clone()}</td>
                                    <td class="num">{one(f.per_unit.kcal)}</td>
                                    <td class="num">{one(f.per_unit.protein)}</td>
                                    <td class="num">{one(f.per_unit.fat)}</td>
                                    <td class="num">{one(f.per_unit.carbohydrate)}</td>
                                    <td class="num dim">{f.eaten}</td>
                                    <td class="dim wrap">{f.note.clone()}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </Section>
    }
}

// ------------------------------------------------------------- food analysis

#[component]
pub fn FoodAnalysisPage() -> impl IntoView {
    let data = Resource::new(
        || (),
        |_| async { crate::web::api::load_food_analysis().await },
    );
    view! {
        <Page title="gym" here="/food/analysis">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <FoodAnalysisView d=d /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn FoodAnalysisView(d: FoodAnalysis) -> impl IntoView {
    let days: Vec<Vec<(String, f64)>> = d
        .days
        .iter()
        .map(|day| {
            day.split
                .iter()
                .map(|s| (s.food.clone(), s.kcal))
                .collect::<Vec<_>>()
        })
        .collect();
    let labels: Vec<String> = d.days.iter().map(|d| d.date[5..].to_string()).collect();
    view! {
        <Section title="Weight and intake">
            <Hint text=format!(
                "{} days shown, {} of them with a full day's logging.",
                d.span, d.logged_days,
            ) />
            {trend_block(&d.trend)}
        </Section>

        <Section title="Intake by day, split by food">
            {if days.is_empty() {
                view! { <Hint text="No meals in the window." /> }.into_any()
            } else {
                view! { <charts::Stacked days=days labels=labels unit="kcal" /> }.into_any()
            }}
        </Section>

        <Section title="Most eaten">
            <table>
                <thead>
                    <tr>
                        <th>"food"</th>
                        <th class="num">"times"</th>
                        <th class="num">"kcal each"</th>
                    </tr>
                </thead>
                <tbody>
                    {d.favourite
                        .iter()
                        .take(15)
                        .map(|f| {
                            view! {
                                <tr>
                                    <td>{f.name.clone()}</td>
                                    <td class="num">{f.eaten}</td>
                                    <td class="num">{one(f.per_unit.kcal)}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </Section>

        <Section title="Most calories contributed">
            <table>
                <thead>
                    <tr>
                        <th>"food"</th>
                        <th class="num">"times"</th>
                        <th class="num">"kcal total"</th>
                    </tr>
                </thead>
                <tbody>
                    {d.by_calories
                        .iter()
                        .take(15)
                        .map(|f| {
                            view! {
                                <tr>
                                    <td>{f.name.clone()}</td>
                                    <td class="num">{f.eaten}</td>
                                    <td class="num">{kcal(f.per_unit.kcal * f.eaten as f64)}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </Section>
    }
}

// ------------------------------------------------------------------ gym daily

#[component]
pub fn GymDailyPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| async { crate::web::api::load_gym_daily().await });
    view! {
        <Page title="gym" here="/gym">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <GymDailyView d=d /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn GymDailyView(d: GymDaily) -> impl IntoView {
    let reminder = match (d.weeks_since_deload, d.deload_weeks) {
        (Some(w), want) if w >= want => format!(
            "{w} weeks since the last deload; the reminder is set to {want}. Entering one is still a choice."
        ),
        (Some(w), want) => format!("{w} weeks since the last deload (reminder at {want})."),
        (None, want) => format!("No deload logged yet (reminder at {want} weeks)."),
    };
    view! {
        <Section title="Deload">
            <div class="row">
                <span class="dim">{reminder}</span>
            </div>
        </Section>

        <Section title="Recent sessions">
            {if d.sessions.is_empty() {
                view! { <Hint text="No sessions logged." /> }.into_any()
            } else {
                view! { <Sessions sessions=d.sessions.clone() /> }.into_any()
            }}
        </Section>

        <Section title="Places">
            <div class="row">
                {d.places
                    .iter()
                    .map(|p| view! { <span class="dim">{p.name.clone()}</span> })
                    .collect_view()}
            </div>
        </Section>
    }
}

#[component]
fn Sessions(sessions: Vec<SessionRow>) -> impl IntoView {
    view! {
        {sessions
            .into_iter()
            .map(|s| {
                let head = format!("{} · {}", s.date, s.place);
                view! {
                    <div class="row">
                        <span class="ok">{head}</span>
                        {if s.deload {
                            view! { <span class="warn">"deload"</span> }.into_any()
                        } else {
                            ().into_any()
                        }}
                        <span class="dim">{s.note}</span>
                    </div>
                    {s.prs
                        .iter()
                        .map(|p| {
                            view! {
                                <div class="row">
                                    <span class="warn">
                                        {format!(
                                            "PR {} · {} x {} = {} e1rm",
                                            p.exercise,
                                            one(p.load),
                                            one(p.rep),
                                            one(p.e1rm),
                                        )}
                                    </span>
                                </div>
                            }
                        })
                        .collect_view()}
                    <table>
                        <thead>
                            <tr>
                                <th class="num">"#"</th>
                                <th>"exercise"</th>
                                <th class="num">"load"</th>
                                <th class="num">"reps"</th>
                                <th>"note"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {s.sets
                                .iter()
                                .map(|x| {
                                    view! {
                                        <tr>
                                            <td class="num dim">{x.position}</td>
                                            <td>
                                                {x.exercise.clone()}
                                                {x
                                                    .superset
                                                    .map(|_| {
                                                        view! { <span class="warn">" · superset"</span> }
                                                    })}
                                                {if x.inferred {
                                                    view! { <span class="dim">" · inferred"</span> }.into_any() } else { ().into_any() }}
                                            </td>
                                            <td class="num">{one(x.load)}</td>
                                            <td class="num">{one(x.rep)}</td>
                                            <td class="dim wrap">{x.note.clone()}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                }
            })
            .collect_view()}
    }
}

// ------------------------------------------------------------------- gym edit

#[component]
pub fn GymEditPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| async { crate::web::api::load_gym_edit().await });
    view! {
        <Page title="gym" here="/gym/edit">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <GymEditView d=d /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn GymEditView(d: GymEdit) -> impl IntoView {
    view! {
        <Section title="Exercises">
            <Hint text="Muscle groups decide the analysis: an exercise with none is listed as ungrouped and appears in no body-part trend." />
            <table>
                <thead>
                    <tr>
                        <th>"exercise"</th>
                        <th>"muscle groups"</th>
                        <th>"video"</th>
                    </tr>
                </thead>
                <tbody>
                    {d.exercises
                        .iter()
                        .map(|e| {
                            let klass = if e.groups.is_empty() { "dim" } else { "" };
                            let groups = if e.groups.is_empty() {
                                "—".to_string()
                            } else {
                                e.groups
                                    .iter()
                                    .map(|g| format!("{} {:.1}", g.name, g.amount))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            };
                            view! {
                                <tr>
                                    <td>{e.name.clone()}</td>
                                    <td class=klass>{groups}</td>
                                    <td class="dim wrap">
                                        {e.video_url.clone().unwrap_or_else(|| "—".to_string())}
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </Section>

        <Section title="Places">
            <table>
                <thead>
                    <tr>
                        <th>"place"</th>
                        <th>"note"</th>
                    </tr>
                </thead>
                <tbody>
                    {d.places
                        .iter()
                        .map(|p| {
                            view! {
                                <tr>
                                    <td>{p.name.clone()}</td>
                                    <td class="dim wrap">{p.note.clone()}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </Section>

        <Section title="Muscle groups">
            <div class="row">
                {d.groups
                    .iter()
                    .map(|g| view! { <span class="dim">{g.name.clone()}</span> })
                    .collect_view()}
            </div>
        </Section>

        <Section title="Recent sessions">
            <Sessions sessions=d.sessions.clone() />
        </Section>
    }
}

// --------------------------------------------------------------- gym analysis

#[component]
pub fn GymAnalysisPage() -> impl IntoView {
    let data = Resource::new(
        || (),
        |_| async { crate::web::api::load_gym_analysis().await },
    );
    view! {
        <Page title="gym" here="/gym/analysis">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <GymAnalysisView d=d /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn GymAnalysisView(d: GymAnalysis) -> impl IntoView {
    let groups: Vec<String> = {
        let mut v: Vec<String> = d.strength.iter().map(|s| s.group.clone()).collect();
        v.sort();
        v.dedup();
        v
    };
    view! {
        <Section title="Strength">
            <Hint text="Best estimated 1RM per exercise per session, from the rep-factor table. Sets above 30 reps are excluded." />
            {if d.strength.is_empty() {
                view! { <Hint text="No exercise has two sessions yet." /> }.into_any()
            } else {
                view! {
                    {groups
                        .iter()
                        .map(|g| {
                            view! {
                                <div class="row">
                                    <span class="ok">{g.clone()}</span>
                                </div>
                                {d.strength
                                    .iter()
                                    .filter(|s| s.group == *g)
                                    .map(|s| {
                                        let points: Vec<(i64, f64)> = s
                                            .points
                                            .iter()
                                            .map(|p| (p.day, p.e1rm))
                                            .collect();
                                        view! {
                                            <div class="row">
                                                <span class="dim">{s.exercise.clone()}</span>
                                            </div>
                                            <charts::Line points=points unit="kg" />
                                        }
                                    })
                                    .collect_view()}
                            }
                        })
                        .collect_view()}
                }
                    .into_any()
            }}
        </Section>

        <Section title="Sets per muscle group per week">
            {if d.volume.is_empty() {
                view! { <Hint text="No exercise has a muscle group yet, so there is no volume to show." /> }
                    .into_any()
            } else {
                view! {
                    <table>
                        <thead>
                            <tr>
                                <th>"muscle group"</th>
                                <th class="num">"weeks"</th>
                                <th class="num">"sets/week (last 4)"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {d.volume
                                .iter()
                                .map(|v| {
                                    let last: f64 = v
                                        .points
                                        .iter()
                                        .rev()
                                        .take(4)
                                        .map(|p| p.sets)
                                        .sum::<f64>()
                                        / v.points.len().min(4).max(1) as f64;
                                    view! {
                                        <tr>
                                            <td>{v.group.clone()}</td>
                                            <td class="num dim">{v.points.len()}</td>
                                            <td class="num">{format!("{last:.1}")}</td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                }
                    .into_any()
            }}
        </Section>

        {if d.ungrouped.is_empty() {
            ().into_any()
        } else {
            view! {
                <Section title="Ungrouped exercises">
                    <Hint text=format!(
                        "{} exercise(s) have no muscle group and appear in no body-part trend.",
                        d.ungrouped.len(),
                    ) />
                    <div class="row">
                        {d.ungrouped
                            .iter()
                            .map(|e| view! { <span class="dim">{e.clone()}</span> })
                            .collect_view()}
                    </div>
                </Section>
            }
                .into_any()
        }}
    }
}

// -------------------------------------------------------------------- status

#[component]
pub fn StatusPage() -> impl IntoView {
    let data = Resource::new(
        || (),
        |_| async { crate::web::api::load_server_info().await },
    );
    view! {
        <Page title="gym" here="/status">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(i) => view! { <StatusView i=i /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn StatusView(i: ServerInfo) -> impl IntoView {
    view! {
        <Section title="Database">
            <table>
                <tbody>
                    <tr>
                        <th>"file"</th>
                        <td class="wrap">{i.db}</td>
                    </tr>
                    <tr>
                        <th>"remote"</th>
                        <td class="wrap">{i.remote}</td>
                    </tr>
                    <tr>
                        <th>"sessions"</th>
                        <td class="num">{i.sessions}</td>
                    </tr>
                    <tr>
                        <th>"sets"</th>
                        <td class="num">{i.sets}</td>
                    </tr>
                    <tr>
                        <th>"meals"</th>
                        <td class="num">{i.meals}</td>
                    </tr>
                    <tr>
                        <th>"weigh-ins"</th>
                        <td class="num">{i.weights}</td>
                    </tr>
                </tbody>
            </table>
        </Section>
    }
}

// --------------------------------------------------------------------- forms

/// The value a browser currently holds for `r`.
///
/// A form field is DOM state: a submit reads what the user sees rather than a
/// mirrored signal, which can lag behind (autofill, IME composition, a harness
/// setting the value). The fields are therefore uncontrolled — an initial
/// `value` and nothing reactive — so typing is never fought by a re-render.
#[cfg(feature = "hydrate")]
fn read(r: NodeRef<leptos::html::Input>) -> Option<String> {
    r.get().map(|e| e.value())
}

#[cfg(not(feature = "hydrate"))]
fn read(_r: NodeRef<leptos::html::Input>) -> Option<String> {
    None
}

/// The same, for a `<select>`.
#[cfg(feature = "hydrate")]
fn read_select(r: NodeRef<leptos::html::Select>) -> Option<String> {
    r.get().map(|e| e.value())
}

#[cfg(not(feature = "hydrate"))]
fn read_select(_r: NodeRef<leptos::html::Select>) -> Option<String> {
    None
}

/// A field's value: what the browser holds, else the initial the page rendered.
fn value_of(r: NodeRef<leptos::html::Input>, initial: &str) -> String {
    read(r).unwrap_or_else(|| initial.to_string())
}

fn selected(r: NodeRef<leptos::html::Select>, initial: i64) -> i64 {
    read_select(r)
        .and_then(|v| v.parse().ok())
        .unwrap_or(initial)
}

/// A one-line result: the server's message, or its error.
#[component]
fn Outcome(outcome: ReadSignal<Option<Result<String, ServerFnError>>>) -> impl IntoView {
    view! {
        {move || {
            outcome
                .get()
                .map(|r| match r {
                    Ok(m) => view! { <span class="ok">{m}</span> }.into_any(),
                    Err(e) => view! { <span class="bad">{e.to_string()}</span> }.into_any(),
                })
        }}
    }
}

#[component]
fn AddMeal(foods: Vec<FoodRow>, today: String, saved: Callback<()>) -> impl IntoView {
    let food = NodeRef::<leptos::html::Select>::new();
    let amount = NodeRef::<leptos::html::Input>::new();
    let date = NodeRef::<leptos::html::Input>::new();
    let time = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let first = foods.first().map(|f| f.id).unwrap_or(0);
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(i64, String, String, String, String)| {
        let (food, amount, date, time, note) = args.clone();
        async move { crate::web::api::act_add_meal(date, time, food, amount, note).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let options: Vec<(i64, String)> = foods
        .iter()
        .map(|f| (f.id, format!("{} · {:.0} kcal", f.name, f.per_unit.kcal)))
        .collect();
    let date0 = today.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action
                .dispatch((
                    selected(food, first),
                    value_of(amount, "1"),
                    value_of(date, &date0),
                    value_of(time, ""),
                    value_of(note, ""),
                ));
        }>
            <label>
                "food"
                <select node_ref=food>
                    {options
                        .iter()
                        .map(|(id, label)| view! { <option value=id.to_string()>{label.clone()}</option> })
                        .collect_view()}
                </select>
            </label>
            <label>
                "amount"
                <input type="text" size="5" value="1" node_ref=amount />
            </label>
            <label>
                "date"
                <input type="date" value=today.clone() node_ref=date />
            </label>
            <label>
                "time"
                <input type="time" value="" node_ref=time />
            </label>
            <label>
                "note"
                <input type="text" size="18" value="" node_ref=note />
            </label>
            <button type="submit" disabled=move || action.pending().get()>
                "add"
            </button>
            <Outcome outcome=outcome />
        </form>
    }
}

#[component]
fn ApplyPreset(presets: Vec<PresetRow>, today: String, saved: Callback<()>) -> impl IntoView {
    let preset = NodeRef::<leptos::html::Select>::new();
    let multiplier = NodeRef::<leptos::html::Input>::new();
    let date = NodeRef::<leptos::html::Input>::new();
    let first = presets.first().map(|p| p.id).unwrap_or(0);
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(i64, String, String)| {
        let (preset, date, multiplier) = args.clone();
        async move { crate::web::api::act_apply_preset(preset, date, String::new(), multiplier).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let options: Vec<(i64, String)> = presets
        .iter()
        .map(|p| {
            (
                p.id,
                format!(
                    "{} · {:.0} kcal · {} item(s)",
                    p.name, p.per_unit.kcal, p.items
                ),
            )
        })
        .collect();
    let date0 = today.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action
                .dispatch((
                    selected(preset, first),
                    value_of(date, &date0),
                    value_of(multiplier, "1"),
                ));
        }>
            <label>
                "preset"
                <select node_ref=preset>
                    {options
                        .iter()
                        .map(|(id, label)| view! { <option value=id.to_string()>{label.clone()}</option> })
                        .collect_view()}
                </select>
            </label>
            <label>
                "×"
                <input type="text" size="4" value="1" node_ref=multiplier />
            </label>
            <label>
                "date"
                <input type="date" value=today.clone() node_ref=date />
            </label>
            <button type="submit" disabled=move || action.pending().get()>
                "record the whole preset"
            </button>
            <Outcome outcome=outcome />
        </form>
    }
}

#[component]
fn AddWeight(today: String, saved: Callback<()>) -> impl IntoView {
    let date = NodeRef::<leptos::html::Input>::new();
    let kg = NodeRef::<leptos::html::Input>::new();
    let bodyfat = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String, String, String)| {
        let (date, kg, bodyfat, note) = args.clone();
        async move { crate::web::api::act_add_weight(date, kg, bodyfat, note).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let date1 = today.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action
                .dispatch((
                    value_of(date, &date1),
                    value_of(kg, ""),
                    value_of(bodyfat, ""),
                    value_of(note, ""),
                ));
        }>
            <label>
                "date"
                <input type="date" value=today.clone() node_ref=date />
            </label>
            <label>
                "weight (kg)"
                <input type="text" size="6" value="" node_ref=kg />
            </label>
            <label>
                "body fat (%)"
                <input type="text" size="4" value="" node_ref=bodyfat />
            </label>
            <label>
                "note"
                <input type="text" size="16" value="" node_ref=note />
            </label>
            <button type="submit" disabled=move || action.pending().get()>
                "save"
            </button>
            <Outcome outcome=outcome />
        </form>
        <Hint text="Leave body fat blank if you did not measure it: blank is stored as unknown, which is not the same as 0%." />
    }
}
