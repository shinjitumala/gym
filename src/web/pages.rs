//! The pages.
//!
//! Three modes — gym, food, weight — each with a daily page, and a mode's edit
//! and analyse pages reached from buttons at the bottom of the daily page
//! rather than from the tab bar.
//!
//! Each page is one `Resource` against a server function, a `Suspense`, and a
//! view that takes the finished data. Nothing here computes a statistic: the
//! numbers arrive already decided, which is what keeps the analysis honest and
//! the WASM bundle small. The two things the browser does decide are UI state —
//! which exercise window is open, which day of the intake chart was picked —
//! and a form's preview, which mirrors the amount being typed.

use crate::web::charts;
use crate::web::dto::*;
use crate::web::ui::*;
use crate::web::ui::{
    BAD, CHIP, CHIPS, DIM, FOLD, GROW, KPIS, LINE, NARROW, OK, PANEL, RECORD, RIGHT, ROW, SMALL,
    SMALL_BAD, WARN, WHEN,
};
use leptos::prelude::*;

// ---------------------------------------------------------------- shared bits

fn verdict_badge(v: Option<Verdict>) -> impl IntoView {
    let (class, text) = match v {
        Some(Verdict::On) => ("ok", "on goal"),
        Some(Verdict::Off) => ("bad", "off goal"),
        Some(Verdict::Uncertain) => ("warn", "too noisy to say"),
        None => ("dim", "no phase, or no trend"),
    };
    view! { <span class=class>{text}</span> }
}

/// The trend's numbers: the slope with its band, the goal verdict, and how many
/// weigh-ins are still missing before a slope is shown at all.
///
/// The chart itself is the caller's: a page that draws the trend in a phase
/// colour must not also draw it in one colour here.
fn trend_summary(t: &TrendView) -> impl IntoView {
    let slope = match (t.per_week, t.band) {
        (Some(s), Some(b)) => format!("{s:+.2} ± {b:.2}"),
        _ => "—".to_string(),
    };
    let missing = t.need.saturating_sub(t.have);
    let goal = t.goal.clone();
    let count = t.points.len();
    let need = t.need;
    view! {
        <div class=ROW>
            <Kpi label="trend" value=slope unit="kg/week" />
            {verdict_badge(t.verdict)}
            {match goal {
                Some(g) => view! { <span class=DIM>{g}</span> }.into_any(),
                None => view! { <span class=DIM>"no phase set"</span> }.into_any(),
            }}
        </div>
        {if count < 2 {
            view! {
                <Hint text=format!(
                    "{count} weigh-in(s) in the window: a line needs two, a slope needs {need}.",
                ) />
            }
                .into_any()
        } else if missing > 0 {
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

/// The trend with its own line chart, for the page that shows one weight chart.
fn trend_block(t: &TrendView) -> impl IntoView {
    let points: Vec<(i64, f64)> = t.points.iter().map(|p| (p.day, p.kg)).collect();
    let fit: Vec<(i64, f64)> = t.fit.iter().map(|p| (p.day, p.kg)).collect();
    let draw = points.len() >= 2;
    view! {
        {trend_summary(t)}
        {if draw {
            view! { <charts::Line points=points fit=fit unit="kg" /> }.into_any()
        } else {
            ().into_any()
        }}
    }
}

fn outliers_block(outliers: &[OutlierRow]) -> impl IntoView {
    if outliers.is_empty() {
        return ().into_any();
    }
    let rows: Vec<OutlierRow> = outliers.to_vec();
    view! {
        <Section title="Days the trend does not explain">
            <Hint text="Days whose intake sits far from what the weight trend implies. A day, never a food: without a controlled repeat a mis-valued food is not identifiable." />
            <table>
                <thead>
                    <tr>
                        <th>"day"</th>
                        <th class=RIGHT>"intake"</th>
                        <th class=RIGHT>"implied"</th>
                        <th class=RIGHT>"diff"</th>
                    </tr>
                </thead>
                <tbody>
                    {rows
                        .iter()
                        .map(|o| {
                            view! {
                                <tr>
                                    <td>{day_label(o.day)}</td>
                                    <td class=RIGHT>{kcal(o.kcal)}</td>
                                    <td class=RIGHT>{kcal(o.expected)}</td>
                                    <td class=RIGHT>{format!("{:+.0}", o.delta)}</td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </Section>
    }
        .into_any()
}

// ---------------------------------------------------------------------- food

#[component]
pub fn FoodDailyPage() -> impl IntoView {
    let (tick, set_tick) = signal(0u32);
    let saved = Callback::new(move |()| set_tick.update(|t| *t += 1));
    let data = Resource::new(
        move || tick.get(),
        |_| async { crate::web::api::load_food_daily().await },
    );
    view! {
        <Page title="food" here="/food">
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
    let today = d.today.clone();
    let now = d.now.clone();
    let maintenance = d.maintenance;
    let totals = d.totals.clone();
    let today_kcal = totals.kcal;
    let phase = d.phase.clone();
    view! {
        <Section title=format!("Today · {}", d.today.clone())>
            <div class=KPIS>
                <Kpi label="calories" value=kcal(totals.kcal) unit="kcal" />
                <Kpi label="protein" value=grams(totals.protein) unit="g" />
                <Kpi label="fat" value=grams(totals.fat) unit="g" />
                <Kpi label="carbohydrate" value=grams(totals.carbohydrate) unit="g" />
            </div>
            {match (maintenance, phase) {
                (Some(m), _) => {
                    let diff = today_kcal - m.kcal;
                    let klass = if diff >= 0.0 { OK } else { BAD };
                    view! {
                        <div class=ROW>
                            <span class=DIM>
                                "maintenance "
                                <span class=OK>{kcal(m.kcal)}</span>
                                " kcal from "
                                {m.days}
                                " logged day(s) · today is "
                                <span class=klass>{format!("{diff:+.0}")}</span>
                                " kcal against it"
                            </span>
                            <span class=DIM>
                                {format!(
                                    "protein {:.0} g, fat {:.0} g, carbohydrate {:.0} g today",
                                    totals.protein,
                                    totals.fat,
                                    totals.carbohydrate,
                                )}
                            </span>
                        </div>
                    }
                        .into_any()
                }
                (None, _) => {
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
            <MealForm
                foods=d.foods.clone()
                presets=d.presets.clone()
                today=today.clone()
                now=now.clone()
                saved=saved
            />
        </Section>

        <Section title="Meals">
            {if d.meals.is_empty() {
                view! { <Hint text="Nothing logged today." /> }.into_any()
            } else {
                view! { <MealTable meals=d.meals.clone() /> }.into_any()
            }}
        </Section>
    }
}

/// One draft meal row, before it is saved.
#[derive(Clone, Debug, PartialEq)]
struct Draft {
    food: FoodRow,
    amount: f64,
    note: String,
}

impl Draft {
    fn totals(&self) -> Totals {
        Totals {
            kcal: self.food.per_unit.kcal * self.amount,
            protein: self.food.per_unit.protein * self.amount,
            fat: self.food.per_unit.fat * self.amount,
            carbohydrate: self.food.per_unit.carbohydrate * self.amount,
        }
    }
}

/// A meal: presets to record in one tap, or a food picked from the list and
/// added to the draft.
///
/// The draft is the preview: each row says what it contributes, and the footer
/// says what the day would come to. `save` writes every row in one transaction,
/// so a half-entered meal is never stored.
#[component]
fn MealForm(
    foods: Vec<FoodRow>,
    presets: Vec<PresetRow>,
    today: String,
    now: String,
    saved: Callback<()>,
) -> impl IntoView {
    let (drafts, set_drafts) = signal(Vec::<Draft>::new());
    let (pending, set_pending) = signal(Option::<FoodRow>::None);
    let (preview, set_preview) = signal(Totals::default());
    let date = NodeRef::<leptos::html::Input>::new();
    let time = NodeRef::<leptos::html::Input>::new();
    let amount = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String, Vec<(i64, String, String)>)| {
        let (date, time, rows) = args.clone();
        async move { crate::web::api::act_add_meals(date, time, rows).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
                set_drafts.set(Vec::new());
            }
            set_outcome.set(Some(r));
        }
    });
    let foods = StoredValue::new(foods);
    let date0 = today.clone();
    let pick = Callback::new(move |id: i64| {
        foods.with_value(|foods| {
            if let Some(f) = foods.iter().find(|f| f.id == id) {
                set_pending.set(Some(f.clone()));
                if let Some(e) = amount.get() {
                    e.set_value("1");
                }
                set_preview.set(f.per_unit.clone());
            }
        });
    });
    let add = Callback::new(move |()| {
        let Some(f) = pending.get() else { return };
        let a = value_of(amount, "1").parse::<f64>().unwrap_or(1.0);
        let n = value_of(note, "");
        set_drafts.update(|d| {
            d.push(Draft {
                food: f,
                amount: a,
                note: n,
            })
        });
        set_pending.set(None);
        set_preview.set(Totals::default());
    });
    let total = Memo::new(move |_| {
        drafts
            .get()
            .iter()
            .fold(Totals::default(), |t, d| t.plus(&d.totals()))
    });
    let shown = move || pending.get().map(|f| f.name.clone()).unwrap_or_default();
    let has_pending = move || pending.get().is_some();
    let rows_for_picker: Vec<(i64, String, String)> = foods.with_value(|foods| {
        foods
            .iter()
            .map(|f| {
                let note = match &f.last_used {
                    Some(day) => ago(day, &today).unwrap_or_default(),
                    None => "never eaten".to_string(),
                };
                (f.id, f.name.clone(), note)
            })
            .collect::<Vec<_>>()
    });
    let preset_list = presets.clone();
    view! {
        {if preset_list.is_empty() {
            ().into_any()
        } else {
            view! {
                <div class=CHIPS>
                    {preset_list
                        .iter()
                        .map(|p| {
                            view! {
                                <PresetChip
                                    p=p.clone()
                                    today=today.clone()
                                    now=now.clone()
                                    saved=saved
                                />
                            }
                        })
                        .collect_view()}
                </div>
            }
                .into_any()
        }}

        <Picker
            label="food · most recently eaten first"
            placeholder="filter foods"
            rows=rows_for_picker
            on_pick=pick
        />

        <div class=ROW>
            <span class=OK>{shown}</span>
            <label class=NARROW>
                "×"
                <input type="text" size="4" value="1" node_ref=amount on:input=move |e| {
                    let a = event_target_value(&e).parse::<f64>().unwrap_or(0.0);
                    set_preview
                        .update(|t| {
                            if let Some(f) = pending.get() {
                                t.kcal = f.per_unit.kcal * a;
                                t.protein = f.per_unit.protein * a;
                                t.fat = f.per_unit.fat * a;
                                t.carbohydrate = f.per_unit.carbohydrate * a;
                            }
                        });
                }
                />
            </label>
            <label class=GROW>
                "note"
                <input type="text" size="10" value="" node_ref=note />
            </label>
            <button type="button" disabled=move || !has_pending() on:click=move |_| add.run(())>
                "add row"
            </button>
        </div>
        <div class=ROW>
            <span class=DIM>
                "this row: "
                <span class=OK>{move || {
                    let t = preview.get();
                    format!("{:.0} kcal · P {:.0} F {:.0} C {:.0} g", t.kcal, t.protein, t.fat, t.carbohydrate)
                }}</span>
            </span>
        </div>

        {move || {
            let rows = drafts.get();
            if rows.is_empty() {
                ().into_any()
            } else {
                view! {
                    <div class=PANEL>
                        {rows
                            .iter()
                            .enumerate()
                            .map(|(i, d)| {
                                let label = format!(
                                    "{} · {:.2} × · {:.0} kcal · P {:.0} F {:.0} C {:.0} g",
                                    d.food.name,
                                    d.amount,
                                    d.totals().kcal,
                                    d.totals().protein,
                                    d.totals().fat,
                                    d.totals().carbohydrate,
                                );
                                view! {
                                    <div class=ROW>
                                        <span class=GROW>{label}</span>
                                        <button
                                            type="button"
                                            class=SMALL_BAD
                                            on:click=move |_| set_drafts
                                                .update(|r| {
                                                    r.remove(i);
                                                })
                                        >
                                            "remove"
                                        </button>
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                    <div class=ROW>
                        <span class=WARN>
                            {move || {
                                let t = total.get();
                                format!(
                                    "preview: {:.0} kcal · P {:.0} F {:.0} C {:.0} g",
                                    t.kcal, t.protein, t.fat, t.carbohydrate,
                                )
                            }}
                        </span>
                    </div>
                }
                    .into_any()
            }
        }}

        <div class=WHEN>
            <label>
                "date"
                <input type="date" value=today.clone() node_ref=date />
            </label>
            <label>
                "time"
                <input type="time" value="" node_ref=time />
            </label>
            <NowButton date=date time=time today=today.clone() now=now.clone() />
        </div>
        <div class=ROW>
            <button
                type="button"
                disabled=move || action.pending().get() || drafts.get().is_empty()
                on:click=move |_| {
                    let rows = drafts
                        .get()
                        .into_iter()
                        .map(|d| (d.food.id, format!("{}", d.amount), d.note))
                        .collect::<Vec<_>>();
                    action.dispatch((value_of(date, &date0), value_of(time, ""), rows));
                }
            >
                "save"
            </button>
            <Outcome outcome=outcome />
        </div>
        <Hint text="Pick a food from the list to add a row, then save. The time defaults to now: press now to fill it from the server's clock." />
    }
}

/// One preset, as a button that records it.
#[component]
fn PresetChip(p: PresetRow, today: String, now: String, saved: Callback<()>) -> impl IntoView {
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(i64, String, String, String)| {
        let (preset, today, now, _) = args.clone();
        async move { crate::web::api::act_apply_preset(preset, today, now, "1".into()).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let name = p.name.clone();
    let kcal = p.per_unit.kcal;
    let items = p.items;
    let id = p.id;
    view! {
        <div class=CHIP>
            <button
                type="button"
                title=format!("Record {name}: {items} item(s), {kcal:.0} kcal")
                on:click=move |_| {
                    action.dispatch((id, today.clone(), now.clone(), String::new()));
                }
                disabled=move || action.pending().get()
            >
                {format!("{name} · {kcal:.0} kcal")}
            </button>
            {move || match outcome.get() {
                Some(Err(e)) => view! { <span class=BAD>{e.to_string()}</span> }.into_any(),
                Some(Ok(_)) => view! { <span class=OK>"✓"</span> }.into_any(),
                None => ().into_any(),
            }}
        </div>
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
                    <th class=RIGHT>"×"</th>
                    <th class=RIGHT>"kcal"</th>
                    <th class="text-right max-sm:hidden">"P"</th>
                    <th class="text-right max-sm:hidden">"F"</th>
                    <th class="text-right max-sm:hidden">"C"</th>
                    <th class="max-sm:hidden">"note"</th>
                </tr>
            </thead>
            <tbody>
                {meals
                    .iter()
                    .map(|m| {
                        view! {
                            <tr>
                                <td>{m.time.clone()}</td>
                                <td>{m.food.clone()}</td>
                                <td class=RIGHT>{format!("{:.2}", m.amount)}</td>
                                <td class=RIGHT>{kcal(m.kcal)}</td>
                                <td class="text-right max-sm:hidden">{grams(m.protein)}</td>
                                <td class="text-right max-sm:hidden">{grams(m.fat)}</td>
                                <td class="text-right max-sm:hidden">{grams(m.carbohydrate)}</td>
                                <td class="text-gray-500 whitespace-normal max-sm:hidden">
                                    {m.note.clone()}
                                </td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

// ---------------------------------------------------------------- food · edit

#[component]
pub fn FoodEditPage() -> impl IntoView {
    let (tick, set_tick) = signal(0u32);
    let saved = Callback::new(move |()| set_tick.update(|t| *t += 1));
    let data = Resource::new(
        move || tick.get(),
        |_| async { crate::web::api::load_food_edit().await },
    );
    view! {
        <Page title="food · edit" here="/food/edit">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <FoodEditView d=d saved=saved /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn FoodEditView(d: FoodEdit, saved: Callback<()>) -> impl IntoView {
    let (view, set_view) = signal("meals".to_string());
    let pick = Callback::new(move |v: String| set_view.set(v));
    let (meals, foods, presets) = (d.clone(), d.clone(), d.clone());
    view! {
        <Segmented
            options=vec![
                ("meals", "meals"),
                ("foods", "foods"),
                ("presets", "presets"),
            ]
            chosen=view
            on_pick=pick
        />
        <div class:hidden=move || view.get() != "meals">
            <MealsEdit d=meals saved=saved />
        </div>
        <div class:hidden=move || view.get() != "foods">
            <FoodPanels d=foods saved=saved />
        </div>
        <div class:hidden=move || view.get() != "presets">
            <PresetPanels d=presets saved=saved />
        </div>
    }
}

/// The editing list for meals: one row per meal, each opening its own form.
#[component]
fn MealsEdit(d: FoodEdit, saved: Callback<()>) -> impl IntoView {
    let meals = d.meals.clone();
    let foods = d.foods.clone();
    view! {
        <Section title="Meals · last 120 days">
            {if meals.is_empty() {
                view! { <Hint text="Nothing logged." /> }.into_any()
            } else {
                view! {
                    {meals
                        .into_iter()
                        .map(|m| view! { <MealEditRow m=m foods=foods.clone() saved=saved /> })
                        .collect_view()}
                }
                    .into_any()
            }}
        </Section>
    }
}

#[component]
fn MealEditRow(m: MealRow, foods: Vec<FoodRow>, saved: Callback<()>) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let date = NodeRef::<leptos::html::Input>::new();
    let time = NodeRef::<leptos::html::Input>::new();
    let food = NodeRef::<leptos::html::Select>::new();
    let amount = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let save = Action::new(move |args: &(i64, String, String, i64, String, String)| {
        let (id, date, time, food, amount, note) = args.clone();
        async move { crate::web::api::act_edit_meal(id, date, time, food, amount, note).await }
    });
    // The food this row already points at, found by name; a select whose value
    // is not among its options reports the first one, which would silently
    // change the meal to a different food.
    let current_food = foods
        .iter()
        .find(|f| f.name == m.food)
        .map(|f| f.id)
        .unwrap_or(-1);
    let del = Action::new(move |id: &i64| {
        let id = *id;
        async move { crate::web::api::act_delete_meal(id).await }
    });
    Effect::new(move |_| {
        if let Some(r) = save.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    Effect::new(move |_| {
        if let Some(r) = del.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let (id, fname, fdate, ftime, famount, fnote) = (
        m.id,
        m.food.clone(),
        m.date.clone(),
        m.time.clone(),
        format!("{}", m.amount),
        m.note.clone(),
    );
    let opts: Vec<(i64, String)> = foods
        .iter()
        .map(|f| (f.id, format!("{} · {:.0} kcal", f.name, f.per_unit.kcal)))
        .collect();
    let d0 = fdate.clone();
    let t0 = ftime.clone();
    let a0 = famount.clone();
    let n0 = fnote.clone();
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=DIM>{format!("{} {}", fdate, ftime)}</span>
                <span class=GROW>{fname.clone()}</span>
                <span class=RIGHT>{famount.clone()}</span>
                <span class=RIGHT>{kcal(m.kcal)}</span>
                <button type="button" class=SMALL on:click=move |_| set_editing.update(|e| *e = !*e)>
                    {move || if editing.get() { "close" } else { "edit" }}
                </button>
                <button
                    type="button"
                    class=SMALL_BAD
                    on:click=move |_| {
                        del.dispatch(id);
                    }
                    disabled=move || del.pending().get()
                >
                    "delete"
                </button>
            </div>
            <div class=FOLD class:hidden=move || !editing.get()>
                <form on:submit=move |e| {
                    e.prevent_default();
                    save
                        .dispatch((
                            id,
                            value_of(date, &d0),
                            value_of(time, &t0),
                            selected(food, current_food),
                            value_of(amount, &a0),
                            value_of(note, &n0),
                        ));
                }>
                    <div class=ROW>
                        <label>
                            "date"
                            <input type="date" value=fdate.clone() node_ref=date />
                        </label>
                        <label>
                            "time"
                            <input type="time" value=ftime.clone() node_ref=time />
                        </label>
                        <label class=GROW>
                            "food"
                            <select node_ref=food>
                                {opts
                                    .iter()
                                    .map(|(fid, label)| {
                                        let sel = foods
                                            .iter()
                                            .find(|f| f.id == *fid)
                                            .map(|f| f.name == fname)
                                            .unwrap_or(false);
                                        view! {
                                            <option value=fid.to_string() selected=sel>
                                                {label.clone()}
                                            </option>
                                        }
                                    })
                                    .collect_view()}
                            </select>
                        </label>
                        <label class=NARROW>
                            "×"
                            <input type="text" size="4" value=famount.clone() node_ref=amount />
                        </label>
                        <label class=GROW>
                            "note"
                            <input type="text" size="10" value=fnote.clone() node_ref=note />
                        </label>
                        <button type="submit" disabled=move || save.pending().get()>
                            "save"
                        </button>
                    </div>
                    <Outcome outcome=outcome />
                </form>
            </div>
            <div class:hidden=move || editing.get() || outcome.get().is_none()>
                <Outcome outcome=outcome />
            </div>
        </div>
    }
}

#[component]
fn FoodPanels(d: FoodEdit, saved: Callback<()>) -> impl IntoView {
    let foods = d.foods.clone();
    view! {
        <Section title="Foods">
            <Hint text="A number left blank is stored as unknown, which is not the same as zero." />
            {foods
                .iter()
                .map(|f| view! { <FoodEditRow f=f.clone() saved=saved /> })
                .collect_view()}
        </Section>
        <Section title="Add a food">
            <AddFoodRow saved=saved />
        </Section>
    }
}

#[component]
fn FoodEditRow(f: FoodRow, saved: Callback<()>) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let name = NodeRef::<leptos::html::Input>::new();
    let cals = NodeRef::<leptos::html::Input>::new();
    let p = NodeRef::<leptos::html::Input>::new();
    let fa = NodeRef::<leptos::html::Input>::new();
    let c = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let save = Action::new(
        move |args: &(i64, String, String, String, String, String, String)| {
            let (id, name, cals, p, fa, c, note) = args.clone();
            async move { crate::web::api::act_edit_food(id, name, cals, p, fa, c, note).await }
        },
    );
    let del = Action::new(move |id: &i64| {
        let id = *id;
        async move { crate::web::api::act_delete_food(id).await }
    });
    Effect::new(move |_| {
        if let Some(r) = save.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    Effect::new(move |_| {
        if let Some(r) = del.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let id = f.id;
    let (n0, c0, p0, f0, cb0, no0) = (
        f.name.clone(),
        format!("{}", f.per_unit.kcal),
        f.per_unit.protein.to_string(),
        f.per_unit.fat.to_string(),
        f.per_unit.carbohydrate.to_string(),
        f.note.clone(),
    );
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=GROW>{f.name.clone()}</span>
                <span class=RIGHT>{format!("{:.0} kcal", f.per_unit.kcal)}</span>
                <span class="text-gray-500 hide-narrow">{format!(
                    "P {:.0} F {:.0} C {:.0}",
                    f.per_unit.protein,
                    f.per_unit.fat,
                    f.per_unit.carbohydrate,
                )}</span>
                <span class=DIM>{format!("{}×", f.eaten)}</span>
                <button type="button" class=SMALL on:click=move |_| set_editing.update(|e| *e = !*e)>
                    {move || if editing.get() { "close" } else { "edit" }}
                </button>
                <button
                    type="button"
                    class=SMALL_BAD
                    on:click=move |_| {
                        del.dispatch(id);
                    }
                    disabled=move || del.pending().get()
                >
                    "delete"
                </button>
            </div>
            <div class=FOLD class:hidden=move || !editing.get()>
                <form on:submit=move |e| {
                    e.prevent_default();
                    save
                        .dispatch((
                            id,
                            value_of(name, &n0),
                            value_of(cals, &c0),
                            value_of(p, &p0),
                            value_of(fa, &f0),
                            value_of(c, &cb0),
                            value_of(note, &no0),
                        ));
                }>
                    <div class=ROW>
                        <label class=GROW>
                            "name"
                            <input type="text" size="14" value=f.name.clone() node_ref=name />
                        </label>
                        <label class=NARROW>
                            "kcal"
                            <input type="text" size="5" value=c0.clone() node_ref=cals />
                        </label>
                        <label class=NARROW>
                            "protein"
                            <input type="text" size="4" value=p0.clone() node_ref=p />
                        </label>
                        <label class=NARROW>
                            "fat"
                            <input type="text" size="4" value=f0.clone() node_ref=fa />
                        </label>
                        <label class=NARROW>
                            "carbs"
                            <input type="text" size="4" value=cb0.clone() node_ref=c />
                        </label>
                        <label class=GROW>
                            "note"
                            <input type="text" size="8" value=no0.clone() node_ref=note />
                        </label>
                        <button type="submit" disabled=move || save.pending().get()>
                            "save"
                        </button>
                    </div>
                    <Outcome outcome=outcome />
                </form>
            </div>
            <div class:hidden=move || editing.get() || outcome.get().is_none()>
                <Outcome outcome=outcome />
            </div>
        </div>
    }
}

#[component]
fn AddFoodRow(saved: Callback<()>) -> impl IntoView {
    let name = NodeRef::<leptos::html::Input>::new();
    let cals = NodeRef::<leptos::html::Input>::new();
    let p = NodeRef::<leptos::html::Input>::new();
    let fa = NodeRef::<leptos::html::Input>::new();
    let c = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(
        move |args: &(String, String, String, String, String, String)| {
            let (name, cals, p, fa, c, note) = args.clone();
            async move { crate::web::api::act_add_food(name, cals, p, fa, c, note).await }
        },
    );
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action
                .dispatch((
                    value_of(name, ""),
                    value_of(cals, ""),
                    value_of(p, ""),
                    value_of(fa, ""),
                    value_of(c, ""),
                    value_of(note, ""),
                ));
        }>
            <div class=ROW>
                <label class=GROW>
                    "name"
                    <input type="text" size="14" value="" node_ref=name />
                </label>
                <label class=NARROW>
                    "kcal"
                    <input type="text" size="5" value="" node_ref=cals />
                </label>
                <label class=NARROW>
                    "protein"
                    <input type="text" size="4" value="" node_ref=p />
                </label>
                <label class=NARROW>
                    "fat"
                    <input type="text" size="4" value="" node_ref=fa />
                </label>
                <label class=NARROW>
                    "carbs"
                    <input type="text" size="4" value="" node_ref=c />
                </label>
                <label class=GROW>
                    "note"
                    <input type="text" size="8" value="" node_ref=note />
                </label>
                <button type="submit" disabled=move || action.pending().get()>
                    "add"
                </button>
            </div>
            <Outcome outcome=outcome />
        </form>
    }
}

#[component]
fn PresetPanels(d: FoodEdit, saved: Callback<()>) -> impl IntoView {
    let foods = d.foods.clone();
    let presets = d.presets.clone();
    view! {
        <Section title="Presets">
            <Hint text="A preset is a named set of foods with amounts, recorded in one tap on the daily page." />
            {if presets.is_empty() {
                view! { <Hint text="No presets yet." /> }.into_any()
            } else {
                view! {
                    {presets
                        .into_iter()
                        .map(|p| view! { <PresetEditRow p=p saved=saved /> })
                        .collect_view()}
                }
                    .into_any()
            }}
        </Section>
        <Section title="Add a preset">
            <AddPresetRow foods=foods saved=saved />
        </Section>
    }
}

#[component]
fn PresetEditRow(p: PresetRow, saved: Callback<()>) -> impl IntoView {
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let del = Action::new(move |id: &i64| {
        let id = *id;
        async move { crate::web::api::act_delete_preset(id).await }
    });
    Effect::new(move |_| {
        if let Some(r) = del.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let id = p.id;
    let parts = p
        .parts
        .iter()
        .map(|x| format!("{} ×{}", x.name, one(x.amount)))
        .collect::<Vec<_>>()
        .join(", ");
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=GROW>{p.name.clone()}</span>
                <span class=RIGHT>{format!("{:.0} kcal", p.per_unit.kcal)}</span>
                <button
                    type="button"
                    class=SMALL_BAD
                    on:click=move |_| {
                        del.dispatch(id);
                    }
                    disabled=move || del.pending().get()
                >
                    "delete"
                </button>
            </div>
            <div class=ROW>
                <span class="text-gray-500 whitespace-normal">{parts}</span>
            </div>
            <Outcome outcome=outcome />
        </div>
    }
}

#[component]
fn AddPresetRow(foods: Vec<FoodRow>, saved: Callback<()>) -> impl IntoView {
    let name = NodeRef::<leptos::html::Input>::new();
    let rows = [
        (
            NodeRef::<leptos::html::Select>::new(),
            NodeRef::<leptos::html::Input>::new(),
        ),
        (
            NodeRef::<leptos::html::Select>::new(),
            NodeRef::<leptos::html::Input>::new(),
        ),
        (
            NodeRef::<leptos::html::Select>::new(),
            NodeRef::<leptos::html::Input>::new(),
        ),
    ];
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, Vec<(i64, String)>)| {
        let (name, foods) = args.clone();
        async move { crate::web::api::act_add_preset(name, foods).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let opts: Vec<(i64, String)> = foods
        .iter()
        .map(|f| (f.id, format!("{} · {:.0} kcal", f.name, f.per_unit.kcal)))
        .collect();
    let opts_for_rows = opts.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            let mut out = Vec::new();
            for (food, amount) in rows {
                let id = selected(food, -1);
                let a = value_of(amount, "");
                if id < 0 || a.trim().is_empty() {
                    continue;
                }
                out.push((id, a));
            }
            action.dispatch((value_of(name, ""), out));
        }>
            <div class=ROW>
                <label class=GROW>
                    "name"
                    <input type="text" size="16" value="" node_ref=name />
                </label>
            </div>
            {rows
                .iter()
                .enumerate()
                .map(|(i, (food, amount))| {
                    let opts = opts_for_rows.clone();
                    let food = *food;
                    let amount = *amount;
                    view! {
                        <div class=LINE>
                            <label class=GROW>
                                "food"
                                <select node_ref=food>
                                    {if i > 0 {
                                        view! { <option value="-1">"—"</option> }.into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                    {opts
                                        .iter()
                                        .map(|(id, label)| {
                                            view! { <option value=id.to_string()>{label.clone()}</option> }
                                        })
                                        .collect_view()}
                                </select>
                            </label>
                            <label class=NARROW>
                                "×"
                                <input type="text" size="4" value=if i == 0 { "1" } else { "" } node_ref=amount />
                            </label>
                        </div>
                    }
                })
                .collect_view()}
            <div class=ROW>
                <button type="submit" disabled=move || action.pending().get()>
                    "add preset"
                </button>
                <Outcome outcome=outcome />
            </div>
        </form>
    }
}

// -------------------------------------------------------------------- weight

#[component]
pub fn WeightDailyPage() -> impl IntoView {
    let (tick, set_tick) = signal(0u32);
    let saved = Callback::new(move |()| set_tick.update(|t| *t += 1));
    let data = Resource::new(
        move || tick.get(),
        |_| async { crate::web::api::load_weight_daily().await },
    );
    view! {
        <Page title="weight" here="/weight">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <WeightDailyView d=d saved=saved /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn WeightDailyView(d: WeightDaily, saved: Callback<()>) -> impl IntoView {
    let phase_line = match &d.phase {
        Some(p) => format!(
            "{} since {} · target {:.2} kg/week at {} reps",
            p.kind, p.start, p.target_rate, p.target_rep
        ),
        None => "no phase yet: the trend has no goal to be measured against".to_string(),
    };
    view! {
        <Section title="Weight">
            <div class=KPIS>
                {match d.last_weight.clone() {
                    Some(w) => {
                        view! {
                            <Kpi label="last weigh-in" value=kg(w.kg) unit="kg" />
                            <Kpi label="body fat" value=opt(w.bodyfat, one) unit="%" />
                            <Kpi
                                label="when"
                                value=format!("{} {}", w.date, w.time)
                                unit=""
                            />
                        }
                            .into_any()
                    }
                    None => view! { <Hint text="No weigh-ins yet." /> }.into_any(),
                }}
            </div>
            <div class=ROW>
                <span class=DIM>{phase_line}</span>
            </div>
            {trend_block(&d.trend)}
        </Section>

        <Section title="Record a weigh-in">
            <WeighInForm today=d.today.clone() now=d.now.clone() saved=saved />
        </Section>

        <Section title="Recent weigh-ins">
            {if d.weights.is_empty() {
                view! { <Hint text="Nothing logged yet." /> }.into_any()
            } else {
                view! { <WeightList weights=d.weights.clone() /> }.into_any()
            }}
        </Section>

        {outliers_block(&d.outliers)}
    }
}

#[component]
fn WeighInForm(today: String, now: String, saved: Callback<()>) -> impl IntoView {
    let date = NodeRef::<leptos::html::Input>::new();
    let time = NodeRef::<leptos::html::Input>::new();
    let kgin = NodeRef::<leptos::html::Input>::new();
    let bf = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String, String, String, String)| {
        let (date, time, kg, bf, note) = args.clone();
        async move { crate::web::api::act_add_weight(date, time, kg, bf, note).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let date0 = today.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action
                .dispatch((
                    value_of(date, &date0),
                    value_of(time, ""),
                    value_of(kgin, ""),
                    value_of(bf, ""),
                    value_of(note, ""),
                ));
        }>
            <div class=WHEN>
                <label>
                    "date"
                    <input type="date" value=today.clone() node_ref=date />
                </label>
                <label>
                    "time"
                    <input type="time" value="" node_ref=time />
                </label>
                <NowButton date=date time=time today=today.clone() now=now.clone() />
            </div>
            <div class=ROW>
                <label class=NARROW>
                    "weight (kg)"
                    <input type="text" size="6" value="" node_ref=kgin />
                </label>
                <label class=NARROW>
                    "body fat (%)"
                    <input type="text" size="4" value="" node_ref=bf />
                </label>
                <label class=GROW>
                    "note"
                    <input type="text" size="12" value="" node_ref=note />
                </label>
                <button type="submit" disabled=move || action.pending().get()>
                    "save"
                </button>
            </div>
            <Outcome outcome=outcome />
            <Hint text="The time defaults to now: press now to fill it from the server's clock. Body fat left blank is stored as unknown, which is not the same as 0%." />
        </form>
    }
}

#[component]
fn WeightList(weights: Vec<WeightRow>) -> impl IntoView {
    view! {
        <table>
            <thead>
                <tr>
                    <th>"when"</th>
                    <th class=RIGHT>"kg"</th>
                    <th class=RIGHT>"bf %"</th>
                    <th class="max-sm:hidden">"note"</th>
                </tr>
            </thead>
            <tbody>
                {weights
                    .iter()
                    .map(|w| {
                        view! {
                            <tr>
                                <td>{format!("{} {}", w.date, w.time)}</td>
                                <td class=RIGHT>{kg(w.kg)}</td>
                                <td class=RIGHT>{opt(w.bodyfat, one)}</td>
                                <td class="text-gray-500 whitespace-normal max-sm:hidden">
                                    {w.note.clone()}
                                </td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

// --------------------------------------------------------------- weight · edit

#[component]
pub fn WeightEditPage() -> impl IntoView {
    let (tick, set_tick) = signal(0u32);
    let saved = Callback::new(move |()| set_tick.update(|t| *t += 1));
    let data = Resource::new(
        move || tick.get(),
        |_| async { crate::web::api::load_weight_edit().await },
    );
    view! {
        <Page title="weight · edit" here="/weight/edit">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <WeightEditView d=d saved=saved /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn WeightEditView(d: WeightEdit, saved: Callback<()>) -> impl IntoView {
    let rows = d.weights.clone();
    view! {
        <Section title="Weigh-ins · last 120">
            {if rows.is_empty() {
                view! { <Hint text="No weigh-ins yet." /> }.into_any()
            } else {
                view! {
                    {rows
                        .into_iter()
                        .map(|w| view! { <WeightEditRow w=w saved=saved /> })
                        .collect_view()}
                }
                    .into_any()
            }}
        </Section>
        <Section title="Add a weigh-in">
            <AddWeightRow today=d.today.clone() now=d.now.clone() saved=saved />
        </Section>
    }
}

#[component]
fn WeightEditRow(w: WeightRow, saved: Callback<()>) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let date = NodeRef::<leptos::html::Input>::new();
    let time = NodeRef::<leptos::html::Input>::new();
    let kgin = NodeRef::<leptos::html::Input>::new();
    let bf = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let save = Action::new(
        move |args: &(i64, String, String, String, String, String)| {
            let (id, date, time, kg, bf, note) = args.clone();
            async move { crate::web::api::act_edit_weight(id, date, time, kg, bf, note).await }
        },
    );
    let del = Action::new(move |id: &i64| {
        let id = *id;
        async move { crate::web::api::act_delete_weight(id).await }
    });
    Effect::new(move |_| {
        if let Some(r) = save.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    Effect::new(move |_| {
        if let Some(r) = del.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let id = w.id;
    let (d0, t0, k0, b0, n0) = (
        w.date.clone(),
        w.time.clone(),
        format!("{}", w.kg),
        w.bodyfat.map(|b| format!("{b}")).unwrap_or_default(),
        w.note.clone(),
    );
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=DIM>{format!("{} {}", w.date, w.time)}</span>
                <span class=GROW>{format!("{} kg", kg(w.kg))}</span>
                <span class=DIM>{opt(w.bodyfat, one)}</span>
                <button type="button" class=SMALL on:click=move |_| set_editing.update(|e| *e = !*e)>
                    {move || if editing.get() { "close" } else { "edit" }}
                </button>
                <button
                    type="button"
                    class=SMALL_BAD
                    on:click=move |_| {
                        del.dispatch(id);
                    }
                    disabled=move || del.pending().get()
                >
                    "delete"
                </button>
            </div>
            <div class=FOLD class:hidden=move || !editing.get()>
                <form on:submit=move |e| {
                    e.prevent_default();
                    save
                        .dispatch((
                            id,
                            value_of(date, &d0),
                            value_of(time, &t0),
                            value_of(kgin, &k0),
                            value_of(bf, &b0),
                            value_of(note, &n0),
                        ));
                }>
                    <div class=ROW>
                        <label>
                            "date"
                            <input type="date" value=w.date.clone() node_ref=date />
                        </label>
                        <label>
                            "time"
                            <input type="time" value=w.time.clone() node_ref=time />
                        </label>
                        <label class=NARROW>
                            "kg"
                            <input type="text" size="6" value=k0.clone() node_ref=kgin />
                        </label>
                        <label class=NARROW>
                            "bf %"
                            <input type="text" size="4" value=b0.clone() node_ref=bf />
                        </label>
                        <label class=GROW>
                            "note"
                            <input type="text" size="10" value=w.note.clone() node_ref=note />
                        </label>
                        <button type="submit" disabled=move || save.pending().get()>
                            "save"
                        </button>
                    </div>
                    <Outcome outcome=outcome />
                </form>
            </div>
            <div class:hidden=move || editing.get() || outcome.get().is_none()>
                <Outcome outcome=outcome />
            </div>
        </div>
    }
}

#[component]
fn AddWeightRow(today: String, now: String, saved: Callback<()>) -> impl IntoView {
    let date = NodeRef::<leptos::html::Input>::new();
    let time = NodeRef::<leptos::html::Input>::new();
    let kgin = NodeRef::<leptos::html::Input>::new();
    let bf = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String, String, String, String)| {
        let (date, time, kg, bf, note) = args.clone();
        async move { crate::web::api::act_add_weight(date, time, kg, bf, note).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let date0 = today.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action
                .dispatch((
                    value_of(date, &date0),
                    value_of(time, ""),
                    value_of(kgin, ""),
                    value_of(bf, ""),
                    value_of(note, ""),
                ));
        }>
            <div class=WHEN>
                <label>
                    "date"
                    <input type="date" value=today.clone() node_ref=date />
                </label>
                <label>
                    "time"
                    <input type="time" value="" node_ref=time />
                </label>
                <NowButton date=date time=time today=today.clone() now=now.clone() />
            </div>
            <div class=ROW>
                <label class=NARROW>
                    "kg"
                    <input type="text" size="6" value="" node_ref=kgin />
                </label>
                <label class=NARROW>
                    "bf %"
                    <input type="text" size="4" value="" node_ref=bf />
                </label>
                <label class=GROW>
                    "note"
                    <input type="text" size="10" value="" node_ref=note />
                </label>
                <button type="submit" disabled=move || action.pending().get()>
                    "add"
                </button>
            </div>
            <Outcome outcome=outcome />
        </form>
    }
}

// ----------------------------------------------------------- weight · analyse

#[component]
pub fn WeightAnalysisPage() -> impl IntoView {
    let (span, set_span) = signal(90i64);
    let data = Resource::new(
        move || span.get(),
        |s| async move { crate::web::api::load_weight_analysis(s).await },
    );
    let pick = Callback::new(move |v: i64| set_span.set(v));
    view! {
        <Page title="weight · analyse" here="/weight/analyse">
            <div class=ROW>
                <span class=DIM>"time range:"</span>
                {[30i64, 90, 180, 365]
                    .into_iter()
                    .map(|d| {
                        view! {
                            <button
                                type="button"
                                class:on=move || span.get() == d
                                on:click=move |_| pick.run(d)
                            >
                                {format!("{d}d")}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <WeightAnalysisView d=d /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn WeightAnalysisView(d: WeightAnalysis) -> impl IntoView {
    let points: Vec<(i64, f64)> = d.trend.points.iter().map(|p| (p.day, p.kg)).collect();
    let bodyfat: Vec<(i64, f64)> = d.bodyfat.iter().map(|p| (p.day, p.kg)).collect();
    let (metric, set_metric) = signal("kcal".to_string());
    let last = d.days.last().map(|x| x.day).unwrap_or(0);
    let (day, set_day) = signal(Option::<i64>::None);
    let pick_day = Callback::new(move |v: i64| set_day.set(Some(v)));
    let days_for_bars = d.days.clone();
    let bars = move || -> Vec<charts::Bar> {
        let m = metric.get();
        days_for_bars
            .iter()
            .map(|day| charts::Bar {
                day: day.day,
                label: day.date[5..].to_string(),
                parts: day
                    .split
                    .iter()
                    .map(|s| {
                        let v = match m.as_str() {
                            "protein" => s.protein,
                            "fat" => s.fat,
                            "carbohydrate" => s.carbohydrate,
                            _ => s.kcal,
                        };
                        (s.food.clone(), v)
                    })
                    .collect(),
            })
            .collect()
    };
    let days = d.days.clone();
    let ranking = d.ranking.clone();
    let chosen = move || {
        day.get()
            .or(Some(last))
            .and_then(|d| days.iter().find(|x| x.day == d).cloned())
    };
    // The ranking is the range's answer, not one day's: every food's total over
    // the whole window, re-sorted by the metric the bars are showing.
    let ranked = move || -> Vec<(String, f64)> {
        let m = metric.get();
        let mut rows: Vec<(String, f64)> = ranking
            .iter()
            .map(|s| {
                let v = match m.as_str() {
                    "protein" => s.protein,
                    "fat" => s.fat,
                    "carbohydrate" => s.carbohydrate,
                    _ => s.kcal,
                };
                (s.food.clone(), v)
            })
            .filter(|(_, v)| *v > 0.0)
            .collect();
        rows.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        rows
    };
    view! {
        <Section title="Weight by phase">
            <Hint text="One line, coloured by the phase each weigh-in was taken in: green in a bulk, amber in a cut." />
            {trend_summary(&d.trend)}
            {if points.len() >= 2 {
                view! {
                    <charts::PhaseLine
                        points=points
                        phases=d.phases.clone()
                        deloads=d.deloads.clone()
                        unit="kg"
                    />
                }
                    .into_any()
            } else {
                ().into_any()
            }}
        </Section>

        <Section title="Body fat">
            {if bodyfat.len() >= 2 {
                view! { <charts::Line points=bodyfat unit="%" /> }.into_any()
            } else {
                view! {
                    <Hint text=format!(
                        "{} weigh-in(s) with a body fat reading: a line needs two.",
                        bodyfat.len(),
                    ) />
                }
                    .into_any()
            }}
        </Section>

        <Section title="Intake">
            <div class=ROW>
                <span class=DIM>{format!("{} days shown", d.span)}</span>
                <span class=DIM>
                    {match d.maintenance {
                        Some(m) => format!("maintenance {:.0} kcal from {} logged days", m.kcal, m.days),
                        None => format!("{} of the days count as logged", d.logged_days),
                    }}
                </span>
            </div>
            <Segmented
                options=vec![
                    ("kcal", "calories"),
                    ("protein", "protein"),
                    ("fat", "fat"),
                    ("carbohydrate", "carbs"),
                ]
                chosen=metric
                on_pick=Callback::new(move |v: String| set_metric.set(v))
            />
            {move || {
                let b = bars();
                if b.is_empty() {
                    view! { <Hint text="No meals in the window." /> }.into_any()
                } else {
                    let unit = metric.get();
                    view! { <charts::Intake bars=b unit=unit on_pick=pick_day /> }.into_any()
                }
            }}
            <div class=ROW>
                <span class=DIM>"press a bar to see what made it"</span>
            </div>
            {move || match chosen() {
                Some(day) => {
                    view! {
                        <div class=PANEL>
                            <div class=ROW>
                                <span class=OK>{day.date.clone()}</span>
                                <span class=DIM>
                                    {format!(
                                        "{:.0} kcal · P {:.0} F {:.0} C {:.0} g",
                                        day.totals.kcal,
                                        day.totals.protein,
                                        day.totals.fat,
                                        day.totals.carbohydrate,
                                    )}
                                </span>
                            </div>
                            <table>
                                <thead>
                                    <tr>
                                        <th>"food"</th>
                                        <th class=RIGHT>"kcal"</th>
                                        <th class=RIGHT>"P"</th>
                                        <th class=RIGHT>"F"</th>
                                        <th class=RIGHT>"C"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {day
                                        .split
                                        .iter()
                                        .map(|s| {
                                            view! {
                                                <tr>
                                                    <td>{s.food.clone()}</td>
                                                    <td class=RIGHT>{kcal(s.kcal)}</td>
                                                    <td class=RIGHT>{grams(s.protein)}</td>
                                                    <td class=RIGHT>{grams(s.fat)}</td>
                                                    <td class=RIGHT>{grams(s.carbohydrate)}</td>
                                                </tr>
                                            }
                                        })
                                        .collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }
                        .into_any()
                }
                None => view! { <Hint text="No day has intake in the window." /> }.into_any(),
            }}
            <div class=PANEL id="food-ranking">
                <div class=ROW>
                    <span class=OK>"Foods over the range"</span>
                    <span class=DIM>
                        {move || match metric.get().as_str() {
                            "protein" => "ranked by protein",
                            "fat" => "ranked by fat",
                            "carbohydrate" => "ranked by carbohydrate",
                            _ => "ranked by calories",
                        }}
                    </span>
                </div>
                {move || {
                    let rows = ranked();
                    let unit = if metric.get() == "kcal" { "kcal" } else { "g" };
                    if rows.is_empty() {
                        view! { <Hint text="No meal in the window, so there is nothing to rank." /> }
                            .into_any()
                    } else {
                        let total: f64 = rows.iter().map(|(_, v)| *v).sum();
                        view! {
                            <table>
                                <thead>
                                    <tr>
                                        <th>"food"</th>
                                        <th class=RIGHT>{format!("{unit}")}</th>
                                        <th class=RIGHT>"share"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {rows
                                        .iter()
                                        .map(|(name, v)| {
                                            let share = if total > 0.0 { v / total * 100.0 } else { 0.0 };
                                            view! {
                                                <tr>
                                                    <td>{name.clone()}</td>
                                                    <td class=RIGHT>{kcal(*v)}</td>
                                                    <td class=RIGHT>{format!("{share:.0}%")}</td>
                                                </tr>
                                            }
                                        })
                                        .collect_view()}
                                </tbody>
                            </table>
                        }
                            .into_any()
                    }
                }}
                <Hint text="The ranking is the whole range, so it answers which food the calories or the protein came from, not just what one day held." />
            </div>
        </Section>
    }
}

// ----------------------------------------------------------------------- gym

#[component]
pub fn GymDailyPage() -> impl IntoView {
    let (tick, set_tick) = signal(0u32);
    let saved = Callback::new(move |()| set_tick.update(|t| *t += 1));
    let (focus, set_focus) = signal(None::<i64>);
    let data = Resource::new(
        move || tick.get(),
        |_| async { crate::web::api::load_gym_daily().await },
    );
    view! {
        <Page title="gym" here="/gym">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => {
                                view! {
                                    <GymDailyView
                                        d=d
                                        saved=saved
                                        tick=tick
                                        focus=focus
                                        set_focus=set_focus
                                    />
                                }
                                    .into_any()
                            }
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn GymDailyView(
    d: GymDaily,
    saved: Callback<()>,
    tick: ReadSignal<u32>,
    focus: ReadSignal<Option<i64>>,
    set_focus: WriteSignal<Option<i64>>,
) -> impl IntoView {
    let reminder = match (d.weeks_since_deload, d.deload_weeks) {
        (Some(w), want) if w >= want => format!(
            "{w} weeks since the last deload; the reminder is set to {want}. Entering one is still a choice: mark the session after starting it.",
        ),
        (Some(w), want) => format!("{w} weeks since the last deload (reminder at {want})."),
        (None, want) => format!("No deload logged yet (reminder at {want} weeks)."),
    };
    let today = d.today.clone();
    let now = d.now.clone();
    let phase = d.phase.clone();
    let efforts = d.efforts.clone();
    let sessions = d.sessions.clone();
    let open = d.open.clone();
    let places = d.places.clone();
    let exercises = d.exercises.clone();
    let presets = d.presets.clone();
    let close = Callback::new(move |()| set_focus.set(None));
    let choose = Callback::new(move |ex: i64| set_focus.set(Some(ex)));
    let today_flow = today.clone();
    let now_flow = now.clone();
    view! {
        <Section title="Phase">
            <div class=ROW>
                <span class=OK>{match &phase {
                    Some(p) => format!(
                        "{} since {} · target {:.2} kg/week at {} reps",
                        p.kind, p.start, p.target_rate, p.target_rep,
                    ),
                    None => "no phase yet".to_string(),
                }}</span>
                <PhaseSwitch kind=phase.as_ref().map(|p| p.kind.clone()) today=today.clone() saved=saved />
            </div>
            <Hint text=reminder />
        </Section>

        {move || {
            let open = open.clone();
            let places = places.clone();
            let today = today_flow.clone();
            let now = now_flow.clone();
            let efforts = efforts.clone();
            let presets = presets.clone();
            let exercises = exercises.clone();
            match open {
                None => {
                    view! {
                        <Section title="Start a session">
                            <StartSession places=places today=today now=now saved=saved />
                        </Section>
                    }
                        .into_any()
                }
                Some(live) => {
                    match focus.get() {
                        Some(ex) => {
                            let session = live.id;
                            view! {
                                <Section title="Exercise">
                                    <ExerciseWindowView
                                        session=session
                                        exercise=ex
                                        tick=tick
                                        saved=saved
                                        efforts=efforts
                                        on_close=close
                                    />
                                </Section>
                            }
                                .into_any()
                        }
                        None => {
                            let live_place = live.place.clone();
                            let live_date = live.date.clone();
                            view! {
                                <Section title=format!("Session · {live_place} · {live_date}")>
                                    <LiveSessionView
                                        live=live
                                        presets=presets
                                        exercises=exercises
                                        efforts=efforts
                                        today=today
                                        saved=saved
                                        on_open=choose
                                    />
                                </Section>
                            }
                                .into_any()
                        }
                    }
                }
            }
        }}

        <Section title="Recent sessions">
            {if sessions.is_empty() {
                view! { <Hint text="No sessions logged." /> }.into_any()
            } else {
                view! { <Sessions sessions=sessions.clone() /> }.into_any()
            }}
        </Section>
    }
}

/// The cut/bulk switch: two buttons, because changing phase is a decision the
/// reader makes, not a form they fill in.
#[component]
fn PhaseSwitch(kind: Option<String>, today: String, saved: Callback<()>) -> impl IntoView {
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String)| {
        let (kind, start) = args.clone();
        async move {
            crate::web::api::act_set_phase(kind, start, "1".into(), "8".into(), "".into()).await
        }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let current = kind.clone();
    let start = today.clone();
    view! {
        {["cut", "bulk"]
            .into_iter()
            .map(|k| {
                let on = current.as_deref() == Some(k);
                let start = start.clone();
                view! {
                    <button
                        type="button"
                        class:on=on
                        on:click=move |_| {
                            action.dispatch((k.to_string(), start.clone()));
                        }
                        disabled=move || action.pending().get()
                    >
                        {if on { format!("{k} ✓") } else { k.to_string() }}
                    </button>
                }
            })
            .collect_view()}
        <Outcome outcome=outcome />
    }
}

/// Start a session: choose the place from a searchable list, newest first.
#[component]
fn StartSession(
    places: Vec<PlaceRow>,
    today: String,
    now: String,
    saved: Callback<()>,
) -> impl IntoView {
    let (place, set_place) = signal(places.first().map(|p| p.id));
    let date = NodeRef::<leptos::html::Input>::new();
    let time = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(i64, String, String)| {
        let (place, date, time) = args.clone();
        async move { crate::web::api::act_start_session(place, date, time).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let rows: Vec<(i64, String, String)> = places
        .iter()
        .map(|p| {
            let note = match &p.last_used {
                Some(day) => format!("last session {}", ago(day, &today).unwrap_or_default()),
                None => "never used".to_string(),
            };
            (p.id, p.name.clone(), note)
        })
        .collect();
    let places_for_pick = places.clone();
    let picked = move || {
        place
            .get()
            .and_then(|id| {
                places_for_pick
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.name.clone())
            })
            .unwrap_or_else(|| "—".to_string())
    };
    let date0 = today.clone();
    view! {
        <Picker
            label="place · most recently used first"
            placeholder="filter places"
            rows=rows
            on_pick=Callback::new(move |id: i64| set_place.set(Some(id)))
        />
        <div class=ROW>
            <span class=WARN>{picked}</span>
        </div>
        <div class=WHEN>
            <label>
                "date"
                <input type="date" value=today.clone() node_ref=date />
            </label>
            <label>
                "time"
                <input type="time" value="" node_ref=time />
            </label>
            <NowButton date=date time=time today=today.clone() now=now.clone() />
        </div>
        <div class=ROW>
            <button
                type="button"
                disabled=move || action.pending().get() || place.get().is_none()
                on:click=move |_| {
                    if let Some(p) = place.get() {
                        action.dispatch((p, value_of(date, &date0), value_of(time, "")));
                    }
                }
            >
                "start session"
            </button>
            <Outcome outcome=outcome />
        </div>
        {if places.is_empty() {
            view! { <Hint text="No places yet: add one in edit." /> }.into_any()
        } else {
            ().into_any()
        }}
    }
}

/// A session in progress: its preset checklists, the exercises on the list,
/// and the way to the next one.
#[component]
fn LiveSessionView(
    live: LiveSession,
    presets: Vec<GymPreset>,
    exercises: Vec<ExerciseRow>,
    efforts: Vec<EffortRow>,
    today: String,
    saved: Callback<()>,
    on_open: Callback<i64>,
) -> impl IntoView {
    let session = live.id;
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let apply = Action::new(move |args: &(i64, i64)| {
        let (session, preset) = *args;
        async move { crate::web::api::act_apply_exercise_preset(session, preset).await }
    });
    let add = Action::new(move |args: &(i64, i64)| {
        let (session, ex) = *args;
        async move { crate::web::api::act_add_exercise(session, ex).await }
    });
    let end = Action::new(move |session: &i64| {
        let session = *session;
        async move { crate::web::api::act_end_session(session).await }
    });
    Effect::new(move |_| {
        if let Some(r) = apply.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    Effect::new(move |_| {
        if let Some(r) = add.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    Effect::new(move |_| {
        if let Some(r) = end.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let live_for_rows = live.clone();
    let rows: Vec<(i64, String, String)> = exercises
        .iter()
        .map(|e| {
            let note = match &e.last_used {
                Some(day) => ago(day, &today).unwrap_or_default(),
                None => "never trained".to_string(),
            };
            (e.id, e.name.clone(), note)
        })
        .collect();
    let pick = Callback::new(move |ex: i64| {
        if let Some(live) = Some(live_for_rows.clone()) {
            add.dispatch((live.id, ex));
        }
        on_open.run(ex);
    });
    let total_sets: i64 = live.sets.len() as i64;
    view! {
        <div class=ROW>
            <span class=DIM>{format!("started {} · {}", live.time, live.place)}</span>
            <DeloadToggle session=live.id deload=live.deload saved=saved />
            <span class="grow"></span>
            <span class=DIM>{format!("{total_sets} set(s) recorded")}</span>
        </div>

        {if presets.is_empty() {
            ().into_any()
        } else {
            view! {
                <div class=CHIPS>
                    {presets
                        .iter()
                        .map(|p| {
                            let id = p.id;
                            let label = format!("{} · {} exercises", p.name, p.items.len());
                            let tooltip = p
                                .items
                                .iter()
                                .map(|i| i.name.clone())
                                .collect::<Vec<_>>()
                                .join(", ");
                            view! {
                                <button
                                    type="button"
                                    title=tooltip
                                    on:click=move |_| {
                                        apply.dispatch((session, id));
                                    }
                                    disabled=move || apply.pending().get()
                                >
                                    {label}
                                </button>
                            }
                        })
                        .collect_view()}
                </div>
            }
                .into_any()
        }}

        <div class=PANEL>
            <div class=ROW>
                <span class=OK>"checklist"</span>
                <span class=DIM>"the exercise window opens from here"</span>
            </div>
            {if live.list.is_empty() {
                view! { <Hint text="No exercises on the list yet: apply a preset or pick one below." /> }
                    .into_any()
            } else {
                view! {
                    {live
                        .list
                        .iter()
                        .map(|item| {
                            let ex = item.exercise;
                            let mark = if item.sets > 0 { "☑" } else { "☐" };
                            let klass = if item.sets > 0 { OK } else { DIM };
                            let label = format!(
                                "{mark} {} · {}",
                                item.name,
                                if item.sets > 0 {
                                    format!("{} set(s)", item.sets)
                                } else {
                                    "not started".to_string()
                                },
                            );
                            view! {
                                <button
                                    type="button"
                                    class=PICK
                                    on:click=move |_| on_open.run(ex)
                                >
                                    <span class=klass>{label}</span>
                                </button>
                            }
                        })
                        .collect_view()}
                }
                    .into_any()
            }}
        </div>

        <SupersetPanel live=live.clone() efforts=efforts saved=saved />

        <Picker
            label="add an exercise · most recently trained first"
            placeholder="filter exercises"
            rows=rows
            on_pick=pick
        />

        <div class=ROW>
            <button
                type="button"
                on:click=move |_| {
                    end.dispatch(session);
                }
                disabled=move || end.pending().get()
            >
                "end session"
            </button>
            <Outcome outcome=outcome />
        </div>
        <Hint text="Ending a session with no sets discards it. The exercise window is where a set is recorded, with its effort and the time it was saved." />
    }
}

/// Two or more checklist exercises recorded as one superset: one group, one
/// load, and a rep count per exercise.
///
/// A superset is a unit, not two adjacent rows, so it is written in one action
/// and the sets carry the group's id. A blank rep row keeps that exercise out
/// of the group: the superset is exactly the exercises that were given reps.
#[component]
fn SupersetPanel(live: LiveSession, efforts: Vec<EffortRow>, saved: Callback<()>) -> impl IntoView {
    let session = live.id;
    let items = live.list.clone();
    // The effort defaults to the last one recorded for the group's first
    // exercise, the same way a single set's form defaults: the group's effort is
    // one judgement about the round, and an absent one would leave the rows out
    // of every later comparison.
    let effort0 = items.first().and_then(|i| i.last_effort);
    let load = NodeRef::<leptos::html::Input>::new();
    let effort = NodeRef::<leptos::html::Select>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let reps: Vec<NodeRef<leptos::html::Input>> =
        (0..items.len()).map(|_| NodeRef::new()).collect();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(i64, String, String, String, String)| {
        let (session, load, reps, effort, note) = args.clone();
        async move { crate::web::api::act_log_superset(session, load, reps, effort, note).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    // The groups already in this session, in the order they were saved.
    let mut groups: Vec<(i64, f64, Vec<(String, f64)>)> = Vec::new();
    for s in &live.sets {
        if let Some(group) = s.superset {
            match groups.iter_mut().find(|(id, _, _)| *id == group) {
                Some((_, _, rows)) => rows.push((s.exercise.clone(), s.rep)),
                None => groups.push((group, s.load, vec![(s.exercise.clone(), s.rep)])),
            }
        }
    }
    let reps_for_submit = reps.clone();
    let items_for_submit = items.clone();
    view! {
        <div class=PANEL>
            <div class=ROW>
                <span class=OK>"superset"</span>
                <span class=DIM>
                    "one load for a group of exercises, and a rep count for each"
                </span>
            </div>
            {groups
                .iter()
                .enumerate()
                .map(|(n, (_, load, rows))| {
                    view! {
                        <div class=ROW>
                            <span class=WARN>
                                {format!("superset {} · {:.1} kg", n + 1, load)}
                            </span>
                            <span class=DIM>
                                {rows
                                    .iter()
                                    .map(|(name, rep)| format!("{name} × {}", one(*rep)))
                                    .collect::<Vec<_>>()
                                    .join("  ")}
                            </span>
                        </div>
                    }
                })
                .collect_view()}
            {if items.len() < 2 {
                view! {
                    <Hint text="A superset needs two exercises on the checklist: apply a preset or add them above." />
                }
                    .into_any()
            } else {
                view! {
                    <form on:submit=move |e| {
                        e.prevent_default();
                        let reps = items_for_submit
                            .iter()
                            .zip(reps_for_submit.iter())
                            .filter_map(|(item, r)| {
                                let v = value_of(r.clone(), "");
                                let v = v.trim().to_string();
                                (!v.is_empty()).then(|| format!("{}:{}", item.exercise, v))
                            })
                            .collect::<Vec<_>>()
                            .join(";");
                        action
                            .dispatch((
                                session,
                                value_of(load, ""),
                                reps,
                                chosen(effort),
                                value_of(note, ""),
                            ));
                    }>
                        <div class=LINE>
                            <label class=NARROW>
                                "load"
                                <input type="text" class="ss-load" size="5" value="" node_ref=load />
                            </label>
                            <label class=GROW>
                                "effort"
                                <select class="ss-effort" node_ref=effort>
                                    {efforts
                                        .iter()
                                        .map(|x| {
                                            let sel = effort0 == Some(x.rank);
                                            view! {
                                                <option value=x.name.clone() selected=sel title=x.hint.clone()>
                                                    {format!("{} · {}", x.name, x.hint)}
                                                </option>
                                            }
                                        })
                                        .collect_view()}
                                </select>
                            </label>
                            <label class=GROW>
                                "note (optional)"
                                <input type="text" class="ss-note" size="10" value="" node_ref=note />
                            </label>
                        </div>
                        <div class=ROW>
                            {items
                                .iter()
                                .zip(reps.iter())
                                .map(|(item, r)| {
                                    let r = r.clone();
                                    view! {
                                        <label class=NARROW>
                                            {item.name.clone()}
                                            <input
                                                type="text"
                                                class="ss-rep"
                                                size="3"
                                                placeholder="reps"
                                                value=""
                                                node_ref=r
                                            />
                                        </label>
                                    }
                                })
                                .collect_view()}
                        </div>
                        <div class=ROW>
                            <button type="submit" disabled=move || action.pending().get()>
                                "save superset"
                            </button>
                            <Outcome outcome=outcome />
                        </div>
                    </form>
                }
                    .into_any()
            }}
        </div>
    }
}

/// One exercise, while its sets are recorded.
#[component]
fn ExerciseWindowView(
    session: i64,
    exercise: i64,
    tick: ReadSignal<u32>,
    efforts: Vec<EffortRow>,
    saved: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let data = Resource::new(
        move || (session, exercise, tick.get()),
        |(s, e, _)| async move { crate::web::api::load_exercise_window(s, e).await },
    );
    view! {
        <div class=ROW>
            <button type="button" class=SMALL on:click=move |_| on_close.run(())>
                "✕ back to the checklist"
            </button>
        </div>
        <Suspense fallback=loading>
            {move || {
                data.get()
                    .map(|r| match r {
                        Ok(w) => view! { <WindowBody session=session w=w saved=saved efforts=efforts.clone() /> }.into_any(),
                        Err(e) => failure(e).into_any(),
                    })
            }}
        </Suspense>
    }
}

#[component]
fn WindowBody(
    session: i64,
    w: ExerciseWindow,
    efforts: Vec<EffortRow>,
    saved: Callback<()>,
) -> impl IntoView {
    let target = w.target_rep;
    let load0 = w.last_load.map(|l| format!("{l}")).unwrap_or_default();
    let rep0 = format!("{target}");
    view! {
        <div class=ROW>
            <span class=OK>{format!("{} · {}", w.name, w.group)}</span>
            <span class=DIM>
                {if w.deload {
                    "deload session".to_string()
                } else {
                    format!("target {target} reps")
                }}
            </span>
        </div>
        {match w.advice.clone() {
            Some(a) => view! { <div class=WARN>{a}</div> }.into_any(),
            None => view! { <Hint text="No earlier session of this exercise: the first set sets the baseline." /> }
                .into_any(),
        }}

        <div class=PANEL>
            <div class=ROW>
                <span class=OK>"this session"</span>
            </div>
            {if w.sets.is_empty() {
                view! { <Hint text="No sets yet. Save one below; it is stamped with the time you save it." /> }
                    .into_any()
            } else {
                view! {
                    <table>
                        <thead>
                            <tr>
                                <th>"time"</th>
                                <th class=RIGHT>"load × reps"</th>
                                <th>"effort"</th>
                                <th class="max-sm:hidden">"note"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {w
                                .sets
                                .iter()
                                .map(|s| {
                                    view! {
                                        <tr>
                                            <td>{s.time.clone()}</td>
                                            <td class=RIGHT>{format!("{} × {}", one(s.load), one(s.rep))}</td>
                                            <td>{s.effort.clone().unwrap_or_else(|| "—".to_string())}</td>
                                            <td class="text-gray-500 whitespace-normal max-sm:hidden">
                                                {s.note.clone()}
                                            </td>
                                        </tr>
                                    }
                                })
                                .collect_view()}
                        </tbody>
                    </table>
                }
                    .into_any()
            }}
        </div>

        <SetForm
            session=session
            exercise=w.exercise
            load0=load0
            rep0=rep0
            effort0=w.last_effort
            efforts=efforts
            saved=saved
        />

        <div class=PANEL>
            <div class=ROW>
                <span class=OK>"recent sessions"</span>
                <span class=DIM>"the numbers to beat"</span>
            </div>
            {if w.history.is_empty() {
                view! { <Hint text="No earlier session of this exercise." /> }.into_any()
            } else {
                view! {
                    {w
                        .history
                        .iter()
                        .map(|h| {
                            view! {
                                <div class=ROW>
                                    <span class=DIM>{h.date.clone()}</span>
                                    {if h.deload {
                                        view! { <span class=WARN>"deload"</span> }.into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                    <span class=GROW>
                                        {h
                                            .sets
                                            .iter()
                                            .map(|x| {
                                                format!(
                                                    "{}×{}{}",
                                                    one(x.load),
                                                    one(x.rep),
                                                    x.effort
                                                        .clone()
                                                        .map(|e| format!(" {e}"))
                                                        .unwrap_or_default(),
                                                )
                                            })
                                            .collect::<Vec<_>>()
                                            .join("  ")}
                                    </span>
                                </div>
                            }
                        })
                        .collect_view()}
                }
                    .into_any()
            }}
        </div>
    }
}

/// One set's row: load, reps, effort, note, save.
#[component]
fn SetForm(
    session: i64,
    exercise: i64,
    load0: String,
    rep0: String,
    effort0: Option<i64>,
    efforts: Vec<EffortRow>,
    saved: Callback<()>,
) -> impl IntoView {
    let load = NodeRef::<leptos::html::Input>::new();
    let rep = NodeRef::<leptos::html::Input>::new();
    let effort = NodeRef::<leptos::html::Select>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(i64, i64, String, String, String, String)| {
        let (session, exercise, load, rep, effort, note) = args.clone();
        async move { crate::web::api::act_log_set(session, exercise, load, rep, effort, note).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let l0 = load0.clone();
    let r0 = rep0.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action
                .dispatch((
                    session,
                    exercise,
                    value_of(load, &l0),
                    value_of(rep, &r0),
                    chosen(effort),
                    value_of(note, ""),
                ));
        }>
            <div class=LINE>
                <label class=NARROW>
                    "load"
                    <input type="text" size="5" value=load0.clone() node_ref=load />
                </label>
                <label class=NARROW>
                    "reps"
                    <input type="text" size="4" value=rep0.clone() node_ref=rep />
                </label>
                <label class=GROW>
                    "effort"
                    <select node_ref=effort>
                        {efforts
                            .iter()
                            .map(|x| {
                                let sel = effort0 == Some(x.rank);
                                view! {
                                    <option value=x.name.clone() selected=sel title=x.hint.clone()>
                                        {format!("{} · {}", x.name, x.hint)}
                                    </option>
                                }
                            })
                            .collect_view()}
                    </select>
                </label>
                <label class=GROW>
                    "note (optional)"
                    <input type="text" size="10" value="" node_ref=note />
                </label>
                <button type="submit" disabled=move || action.pending().get()>
                    "save set"
                </button>
            </div>
            <Outcome outcome=outcome />
        </form>
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
                    <div class=ROW>
                        <span class=OK>{head}</span>
                        {if s.deload {
                            view! { <span class=WARN>"deload"</span> }.into_any()
                        } else {
                            ().into_any()
                        }}
                        <span class=DIM>{s.note}</span>
                    </div>
                    {s.prs
                        .iter()
                        .map(|p| {
                            view! {
                                <div class=ROW>
                                    <span class=WARN>
                                        {format!(
                                            "PR {} · {} × {} = {} e1rm",
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
                                <th>"exercise"</th>
                                <th class=RIGHT>"load"</th>
                                <th class=RIGHT>"reps"</th>
                                <th>"effort"</th>
                                <th class="max-sm:hidden">"note"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {s.sets
                                .iter()
                                .map(|x| {
                                    view! {
                                        <tr>
                                            <td>
                                                {x.exercise.clone()}
                                                {x
                                                    .superset
                                                    .map(|_| view! { <span class=WARN>" · ss"</span> })}
                                                {if x.inferred {
                                                    view! { <span class=DIM>" · inferred"</span> }.into_any()
                                                } else {
                                                    ().into_any()
                                                }}
                                            </td>
                                            <td class=RIGHT>{one(x.load)}</td>
                                            <td class=RIGHT>{one(x.rep)}</td>
                                            <td>{x.effort.clone().unwrap_or_else(|| "—".to_string())}</td>
                                            <td class="text-gray-500 whitespace-normal max-sm:hidden">{x.note.clone()}</td>
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

// ------------------------------------------------------------------ gym · edit

#[component]
pub fn GymEditPage() -> impl IntoView {
    let (tick, set_tick) = signal(0u32);
    let saved = Callback::new(move |()| set_tick.update(|t| *t += 1));
    let data = Resource::new(
        move || tick.get(),
        |_| async { crate::web::api::load_gym_edit().await },
    );
    view! {
        <Page title="gym · edit" here="/gym/edit">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(d) => view! { <GymEditView d=d saved=saved /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn GymEditView(d: GymEdit, saved: Callback<()>) -> impl IntoView {
    let (view, set_view) = signal("sessions".to_string());
    let pick = Callback::new(move |v: String| set_view.set(v));
    let (sessions, exercises, presets, places, groups) =
        (d.clone(), d.clone(), d.clone(), d.clone(), d.clone());
    view! {
        <Segmented
            options=vec![
                ("sessions", "sessions"),
                ("exercises", "exercises"),
                ("presets", "presets"),
                ("places", "places"),
                ("groups", "groups"),
            ]
            chosen=view
            on_pick=pick
        />
        <div class:hidden=move || view.get() != "sessions">
            <Section title="Recent sessions">
                <Hint text="Edit a set's load, reps, effort or note, or delete it. Deleting closes the gap in the session's order." />
                <SessionsEdit d=sessions efforts=d.efforts.clone() saved=saved />
            </Section>
        </div>
        <div class:hidden=move || view.get() != "exercises">
            <ExercisePanels d=exercises saved=saved />
        </div>
        <div class:hidden=move || view.get() != "presets">
            <GymPresetPanels d=presets saved=saved />
        </div>
        <div class:hidden=move || view.get() != "places">
            <PlacePanels d=places saved=saved />
        </div>
        <div class:hidden=move || view.get() != "groups">
            <GroupPanels d=groups saved=saved />
        </div>
    }
}

#[component]
fn SessionsEdit(d: GymEdit, efforts: Vec<EffortRow>, saved: Callback<()>) -> impl IntoView {
    let sessions = d.sessions.clone();
    view! {
        {sessions
            .into_iter()
            .map(|s| {
                let head = format!("{} · {}", s.date, s.place);
                view! {
                    <div class=ROW>
                        <span class=OK>{head}</span>
                        <DeloadToggle session=s.id deload=s.deload saved=saved />
                    </div>
                    {s
                        .sets
                        .iter()
                        .map(|x| {
                            view! {
                                <SetEditRow x=x.clone() efforts=efforts.clone() saved=saved />
                            }
                        })
                        .collect_view()}
                }
            })
            .collect_view()}
    }
}

#[component]
fn SetEditRow(x: SetRow, efforts: Vec<EffortRow>, saved: Callback<()>) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let load = NodeRef::<leptos::html::Input>::new();
    let rep = NodeRef::<leptos::html::Input>::new();
    let effort = NodeRef::<leptos::html::Select>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let save = Action::new(move |args: &(i64, String, String, String, String)| {
        let (id, load, rep, effort, note) = args.clone();
        async move { crate::web::api::act_edit_set(id, load, rep, effort, note).await }
    });
    let del = Action::new(move |id: &i64| {
        let id = *id;
        async move { crate::web::api::act_delete_set(id).await }
    });
    Effect::new(move |_| {
        if let Some(r) = save.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    Effect::new(move |_| {
        if let Some(r) = del.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let id = x.id;
    let (l0, r0, n0) = (format!("{}", x.load), format!("{}", x.rep), x.note.clone());
    let current = x.effort.clone().unwrap_or_default();
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=DIM>{x.time.clone()}</span>
                <span class=GROW>{x.exercise.clone()}</span>
                <span class=RIGHT>{format!("{} × {}", one(x.load), one(x.rep))}</span>
                <span class=DIM>{x.effort.clone().unwrap_or_else(|| "—".to_string())}</span>
                <button type="button" class=SMALL on:click=move |_| set_editing.update(|e| *e = !*e)>
                    {move || if editing.get() { "close" } else { "edit" }}
                </button>
                <button
                    type="button"
                    class=SMALL_BAD
                    on:click=move |_| {
                        del.dispatch(id);
                    }
                    disabled=move || del.pending().get()
                >
                    "delete"
                </button>
            </div>
            <div class=FOLD class:hidden=move || !editing.get()>
                <form on:submit=move |e| {
                    e.prevent_default();
                    save
                        .dispatch((
                            id,
                            value_of(load, &l0),
                            value_of(rep, &r0),
                            chosen(effort),
                            value_of(note, &n0),
                        ));
                }>
                    <div class=ROW>
                        <label class=NARROW>
                            "load"
                            <input type="text" size="5" value=l0.clone() node_ref=load />
                        </label>
                        <label class=NARROW>
                            "reps"
                            <input type="text" size="4" value=r0.clone() node_ref=rep />
                        </label>
                        <label class=GROW>
                            "effort"
                            <select node_ref=effort>
                                <option value="" selected=current.is_empty()>"—"</option>
                                {efforts
                                    .iter()
                                    .map(|x| {
                                        let sel = current == x.name;
                                        view! {
                                            <option value=x.name.clone() selected=sel>
                                                {x.name.clone()}
                                            </option>
                                        }
                                    })
                                    .collect_view()}
                            </select>
                        </label>
                        <label class=GROW>
                            "note"
                            <input type="text" size="12" value=n0.clone() node_ref=note />
                        </label>
                        <button type="submit" disabled=move || save.pending().get()>
                            "save"
                        </button>
                    </div>
                    <Outcome outcome=outcome />
                </form>
            </div>
            <div class:hidden=move || editing.get() || outcome.get().is_none()>
                <Outcome outcome=outcome />
            </div>
        </div>
    }
}

/// Flag or unflag one session as a deload.
///
/// The flag belongs to the session, not to a set: a deload is a week's plan,
/// and marking one set of it would say nothing about the other five.
#[component]
fn DeloadToggle(session: i64, deload: bool, saved: Callback<()>) -> impl IntoView {
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let flag = Action::new(move |args: &(i64, bool)| {
        let (id, on) = *args;
        async move { crate::web::api::act_set_deload(id, on).await }
    });
    Effect::new(move |_| {
        if let Some(r) = flag.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    view! {
        {if deload {
            view! { <span class=WARN>"deload"</span> }.into_any()
        } else {
            ().into_any()
        }}
        <button
            type="button"
            class=SMALL
            on:click=move |_| {
                flag.dispatch((session, !deload));
            }
            disabled=move || flag.pending().get()
        >
            {if deload { "un-deload" } else { "mark deload" }}
        </button>
        <Outcome outcome=outcome />
    }
}

#[component]
fn ExercisePanels(d: GymEdit, saved: Callback<()>) -> impl IntoView {
    let exercises = d.exercises.clone();
    let groups = d.groups.clone();
    view! {
        <Section title="Merge two exercises">
            <MergeForm exercises=d.exercises.clone() saved=saved />
        </Section>
        <Section title="Exercises">
            <Hint text="Muscle groups decide the analysis: an exercise with none is listed as ungrouped and appears in no body-part trend." />
            {exercises
                .iter()
                .map(|e| {
                    view! {
                        <ExerciseEditRow e=e.clone() groups=groups.clone() saved=saved />
                    }
                })
                .collect_view()}
        </Section>
    }
}

#[component]
fn MergeForm(exercises: Vec<ExerciseRow>, saved: Callback<()>) -> impl IntoView {
    let from = NodeRef::<leptos::html::Select>::new();
    let to = NodeRef::<leptos::html::Select>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(i64, i64)| {
        let (from, to) = *args;
        async move { crate::web::api::act_merge_exercises(from, to).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action.dispatch((selected(from, 0), selected(to, 0)));
        }>
            <div class=ROW>
                <label class=GROW>
                    "merge"
                    <select node_ref=from>
                        {exercises
                            .iter()
                            .map(|x| view! { <option value=x.id.to_string()>{x.name.clone()}</option> })
                            .collect_view()}
                    </select>
                </label>
                <label class=GROW>
                    "into"
                    <select node_ref=to>
                        {exercises
                            .iter()
                            .map(|x| view! { <option value=x.id.to_string()>{x.name.clone()}</option> })
                            .collect_view()}
                    </select>
                </label>
                <button type="submit" disabled=move || action.pending().get()>
                    "merge"
                </button>
            </div>
            <Outcome outcome=outcome />
            <Hint text="Every set and list item moves to the survivor, its muscle groups are unioned, and the merged-away exercise is deleted. There is no undo but the log's own backup." />
        </form>
    }
}

#[component]
fn ExerciseEditRow(e: ExerciseRow, groups: Vec<MuscleRow>, saved: Callback<()>) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let name = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let video = NodeRef::<leptos::html::Input>::new();
    // The working set of links: start from what the exercise has, then add or
    // remove; "save" replaces the lot in one transaction.
    let (links, set_links) = signal(
        e.groups
            .iter()
            .map(|g| (g.id, g.amount))
            .collect::<Vec<(i64, f64)>>(),
    );
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let save = Action::new(move |args: &(i64, String, String, String)| {
        let (id, name, note, video) = args.clone();
        async move { crate::web::api::act_edit_exercise(id, name, note, video).await }
    });
    let links_save = Action::new(move |args: &(i64, Vec<(i64, String)>)| {
        let (id, links) = args.clone();
        async move { crate::web::api::act_set_groups(id, links).await }
    });
    Effect::new(move |_| {
        if let Some(r) = save.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    Effect::new(move |_| {
        if let Some(r) = links_save.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let id = e.id;
    let (n0, no0, v0) = (
        e.name.clone(),
        e.note.clone(),
        e.video_url.clone().unwrap_or_default(),
    );
    let groups_for_add = groups.clone();
    let groups_for_name = groups.clone();
    let group_name = move |gid: i64| {
        groups_for_name
            .iter()
            .find(|g| g.id == gid)
            .map(|g| g.name.clone())
            .unwrap_or_else(|| format!("#{gid}"))
    };
    let group_names: Vec<(i64, String)> = groups.iter().map(|g| (g.id, g.name.clone())).collect();
    let groups_class = if e.groups.is_empty() { DIM } else { "" };
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=GROW>{e.name.clone()}</span>
                <span class=groups_class>
                    {if e.groups.is_empty() {
                        "ungrouped".to_string()
                    } else {
                        e.groups
                            .iter()
                            .map(|g| format!("{} {:.1}", g.name, g.amount))
                            .collect::<Vec<_>>()
                            .join(", ")
                    }}
                </span>
                <button type="button" class=SMALL on:click=move |_| set_editing.update(|x| *x = !*x)>
                    {move || if editing.get() { "close" } else { "edit" }}
                </button>
            </div>
            <div class=FOLD class:hidden=move || !editing.get()>
                <form on:submit=move |ev| {
                    ev.prevent_default();
                    save
                        .dispatch((
                            id,
                            value_of(name, &n0),
                            value_of(note, &no0),
                            value_of(video, &v0),
                        ));
                }>
                    <div class=ROW>
                        <label class=GROW>
                            "name"
                            <input type="text" size="16" value=n0.clone() node_ref=name />
                        </label>
                        <label class=GROW>
                            "note"
                            <input type="text" size="12" value=no0.clone() node_ref=note />
                        </label>
                        <label class=GROW>
                            "video"
                            <input type="text" size="16" value=v0.clone() node_ref=video />
                        </label>
                        <button type="submit" disabled=move || save.pending().get()>
                            "save"
                        </button>
                    </div>
                    <Outcome outcome=outcome />
                </form>
                <div class=ROW>
                    <span class=DIM>"muscle groups:"</span>
                    {move || {
                        links
                            .get()
                            .into_iter()
                            .map(|(gid, amount)| {
                                let label = format!("{} {:.1} ✕", group_name(gid), amount);
                                view! {
                                    <button
                                        type="button"
                                        class=SMALL
                                        on:click=move |_| {
                                            set_links
                                                .update(|l| l.retain(|(g, _)| *g != gid))
                                        }
                                    >
                                        {label}
                                    </button>
                                }
                            })
                            .collect_view()
                    }}
                    <GroupAdder groups=groups_for_add.clone() on_add=Callback::new(move |p: (i64, f64)| {
                        set_links
                            .update(|l| {
                                l.retain(|(g, _)| *g != p.0);
                                l.push(p);
                            })
                    }) />
                    <button
                        type="button"
                        on:click=move |_| {
                            let out = links
                                .get()
                                .into_iter()
                                .map(|(g, a)| (g, format!("{a}")))
                                .collect::<Vec<_>>();
                            links_save.dispatch((id, out));
                        }
                        disabled=move || links_save.pending().get()
                    >
                        "save groups"
                    </button>
                </div>
                <div class=ROW>
                    <span class=DIM>{group_names.iter().map(|(_, n)| n.clone()).collect::<Vec<_>>().join(" · ")}</span>
                </div>
            </div>
            <div class:hidden=move || editing.get() || outcome.get().is_none()>
                <Outcome outcome=outcome />
            </div>
        </div>
    }
}

/// A muscle-group picker that reports `(group, amount)` to whoever is building
/// a list of links.
#[component]
fn GroupAdder(groups: Vec<MuscleRow>, #[prop(into)] on_add: Callback<(i64, f64)>) -> impl IntoView {
    let group = NodeRef::<leptos::html::Select>::new();
    let amount = NodeRef::<leptos::html::Input>::new();
    view! {
        <select node_ref=group>
            {groups
                .iter()
                .map(|g| view! { <option value=g.id.to_string()>{g.name.clone()}</option> })
                .collect_view()}
        </select>
        <input type="text" size="4" value="1" node_ref=amount />
        <button type="button" class=SMALL on:click=move |_| {
            let g = selected(group, 0);
            let a = value_of(amount, "1").parse::<f64>().unwrap_or(1.0);
            if g != 0 {
                on_add.run((g, a));
            }
        }>
            "+"
        </button>
    }
}

/// The gym presets: a named, ordered list of exercises the daily page applies
/// to a session as its checklist.
#[component]
fn GymPresetPanels(d: GymEdit, saved: Callback<()>) -> impl IntoView {
    let presets = d.presets.clone();
    let names: Vec<(i64, String)> = d.exercises.iter().map(|e| (e.id, e.name.clone())).collect();
    view! {
        <Section title="Presets">
            <Hint text="A preset is a named list of exercises. Applying it on the daily page puts its exercises on the session's checklist." />
            {if presets.is_empty() {
                view! { <Hint text="No presets yet." /> }.into_any()
            } else {
                view! {
                    {presets
                        .into_iter()
                        .map(|p| view! { <GymPresetRow p=p saved=saved /> })
                        .collect_view()}
                }
                    .into_any()
            }}
        </Section>
        <Section title="Add a preset">
            <AddGymPreset names=names saved=saved />
        </Section>
    }
}

#[component]
fn GymPresetRow(p: GymPreset, saved: Callback<()>) -> impl IntoView {
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let del = Action::new(move |id: &i64| {
        let id = *id;
        async move { crate::web::api::act_delete_exercise_preset(id).await }
    });
    Effect::new(move |_| {
        if let Some(r) = del.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let id = p.id;
    let items = p
        .items
        .iter()
        .map(|i| i.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=GROW>{p.name.clone()}</span>
                <span class=DIM>{format!("{} exercise(s)", p.items.len())}</span>
                <button
                    type="button"
                    class=SMALL_BAD
                    on:click=move |_| {
                        del.dispatch(id);
                    }
                    disabled=move || del.pending().get()
                >
                    "delete"
                </button>
            </div>
            <div class=ROW>
                <span class="text-gray-500 whitespace-normal">{items}</span>
            </div>
            <Outcome outcome=outcome />
        </div>
    }
}

#[component]
fn AddGymPreset(names: Vec<(i64, String)>, saved: Callback<()>) -> impl IntoView {
    let name = NodeRef::<leptos::html::Input>::new();
    let slots = [
        NodeRef::<leptos::html::Select>::new(),
        NodeRef::<leptos::html::Select>::new(),
        NodeRef::<leptos::html::Select>::new(),
        NodeRef::<leptos::html::Select>::new(),
    ];
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, Vec<i64>)| {
        let (name, exercises) = args.clone();
        async move { crate::web::api::act_add_exercise_preset(name, exercises).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let names_for_rows = names.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            let mut out: Vec<i64> = Vec::new();
            for slot in slots {
                let id = selected(slot, -1);
                if id > 0 && !out.contains(&id) {
                    out.push(id);
                }
            }
            action.dispatch((value_of(name, ""), out));
        }>
            <div class=ROW>
                <label class=GROW>
                    "name"
                    <input type="text" size="16" value="" node_ref=name />
                </label>
            </div>
            {slots
                .iter()
                .enumerate()
                .map(|(i, slot)| {
                    let slot = *slot;
                    let names = names_for_rows.clone();
                    view! {
                        <div class=LINE>
                            <label class=GROW>
                                "exercise"
                                <select node_ref=slot>
                                    {if i > 0 {
                                        view! { <option value="-1">"—"</option> }.into_any()
                                    } else {
                                        ().into_any()
                                    }}
                                    {names
                                        .iter()
                                        .map(|(id, n)| {
                                            view! { <option value=id.to_string()>{n.clone()}</option> }
                                        })
                                        .collect_view()}
                                </select>
                            </label>
                        </div>
                    }
                })
                .collect_view()}
            <div class=ROW>
                <button type="submit" disabled=move || action.pending().get()>
                    "add preset"
                </button>
                <Outcome outcome=outcome />
            </div>
        </form>
    }
}

#[component]
fn PlacePanels(d: GymEdit, saved: Callback<()>) -> impl IntoView {
    let places = d.places.clone();
    view! {
        <Section title="Places">
            {places
                .iter()
                .map(|p| view! { <PlaceEditRow p=p.clone() saved=saved /> })
                .collect_view()}
        </Section>
        <Section title="Add a place">
            <AddPlaceRow saved=saved />
        </Section>
    }
}

#[component]
fn PlaceEditRow(p: PlaceRow, saved: Callback<()>) -> impl IntoView {
    let name = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let save = Action::new(move |args: &(i64, String, String)| {
        let (id, name, note) = args.clone();
        async move { crate::web::api::act_edit_place(id, name, note).await }
    });
    let del = Action::new(move |id: &i64| {
        let id = *id;
        async move { crate::web::api::act_delete_place(id).await }
    });
    Effect::new(move |_| {
        if let Some(r) = save.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    Effect::new(move |_| {
        if let Some(r) = del.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let id = p.id;
    let (n0, no0) = (p.name.clone(), p.note.clone());
    view! {
        <form class=RECORD on:submit=move |e| {
            e.prevent_default();
            save.dispatch((id, value_of(name, &n0), value_of(note, &no0)));
        }>
            <div class=ROW>
                <label class=GROW>
                    "name"
                    <input type="text" size="16" value=n0.clone() node_ref=name />
                </label>
                <label class=GROW>
                    "note"
                    <input type="text" size="10" value=no0.clone() node_ref=note />
                </label>
                <button type="submit" disabled=move || save.pending().get()>
                    "save"
                </button>
                <button
                    type="button"
                    class=SMALL_BAD
                    on:click=move |_| {
                        del.dispatch(id);
                    }
                    disabled=move || del.pending().get()
                >
                    "delete"
                </button>
            </div>
            <Outcome outcome=outcome />
        </form>
    }
}

#[component]
fn AddPlaceRow(saved: Callback<()>) -> impl IntoView {
    let name = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String)| {
        let (name, note) = args.clone();
        async move { crate::web::api::act_add_place(name, note).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action.dispatch((value_of(name, ""), value_of(note, "")));
        }>
            <div class=ROW>
                <label class=GROW>
                    "name"
                    <input type="text" size="16" value="" node_ref=name />
                </label>
                <label class=GROW>
                    "note"
                    <input type="text" size="10" value="" node_ref=note />
                </label>
                <button type="submit" disabled=move || action.pending().get()>
                    "add"
                </button>
            </div>
            <Outcome outcome=outcome />
        </form>
    }
}

#[component]
fn GroupPanels(d: GymEdit, saved: Callback<()>) -> impl IntoView {
    let groups = d.groups.clone();
    view! {
        <Section title="Muscle groups">
            {groups
                .iter()
                .map(|g| view! { <GroupRowView g=g.clone() saved=saved /> })
                .collect_view()}
        </Section>
        <Section title="Add a muscle group">
            <AddGroupRow saved=saved />
        </Section>
    }
}

#[component]
fn GroupRowView(g: MuscleRow, saved: Callback<()>) -> impl IntoView {
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let del = Action::new(move |id: &i64| {
        let id = *id;
        async move { crate::web::api::act_delete_group(id).await }
    });
    Effect::new(move |_| {
        if let Some(r) = del.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let id = g.id;
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=GROW>{g.name.clone()}</span>
                <button
                    type="button"
                    class=SMALL_BAD
                    on:click=move |_| {
                        del.dispatch(id);
                    }
                    disabled=move || del.pending().get()
                >
                    "delete"
                </button>
            </div>
            <Outcome outcome=outcome />
        </div>
    }
}

#[component]
fn AddGroupRow(saved: Callback<()>) -> impl IntoView {
    let name = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String)| {
        let (name, note) = args.clone();
        async move { crate::web::api::act_add_group(name, note).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action.dispatch((value_of(name, ""), value_of(note, "")));
        }>
            <div class=ROW>
                <label class=GROW>
                    "name"
                    <input type="text" size="16" value="" node_ref=name />
                </label>
                <label class=GROW>
                    "note"
                    <input type="text" size="10" value="" node_ref=note />
                </label>
                <button type="submit" disabled=move || action.pending().get()>
                    "add"
                </button>
            </div>
            <Outcome outcome=outcome />
        </form>
    }
}

// -------------------------------------------------------------- gym · analyse

#[component]
pub fn GymAnalysisPage() -> impl IntoView {
    let (span, set_span) = signal(180i64);
    let (want, set_want) = signal(None::<i64>);
    let data = Resource::new(
        move || (span.get(), want.get()),
        |(s, w)| async move { crate::web::api::load_gym_analysis(s, w).await },
    );
    let pick = Callback::new(move |v: i64| set_span.set(v));
    view! {
        <Page title="gym · analyse" here="/gym/analyse">
            <div class=ROW>
                <span class=DIM>"time range:"</span>
                {[30i64, 90, 180, 365]
                    .into_iter()
                    .map(|d| {
                        view! {
                            <button
                                type="button"
                                class:on=move || span.get() == d
                                on:click=move |_| pick.run(d)
                            >
                                {format!("{d}d")}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(a) => {
                                view! { <GymAnalysisView a=a on_pick=Callback::new(move |id: i64| set_want.set(Some(id))) /> }
                                    .into_any()
                            }
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn GymAnalysisView(a: GymAnalysis, on_pick: Callback<i64>) -> impl IntoView {
    let chosen = a.chosen.clone();
    let options = a.exercises.clone();
    let current = chosen
        .as_ref()
        .and_then(|c| options.iter().find(|o| o.name == c.exercise).map(|o| o.id));
    view! {
        <Section title="Strength and volume">
            <Hint text="The thick line is the chosen exercise's best estimated 1RM per session. The dashed lines are sets per week for that exercise and for the exercises that share its main muscle group, on the right-hand scale: the overlay is what answers whether strength moved when the volume did." />
            <div class=ROW>
                <label class=GROW>
                    "exercise"
                    <select
                        on:change=move |e| {
                            if let Ok(id) = event_target_value(&e).parse::<i64>() {
                                on_pick.run(id);
                            }
                        }
                    >
                        {options
                            .iter()
                            .map(|o| {
                                view! {
                                    <option value=o.id.to_string() selected=current == Some(o.id)>
                                        {format!("{} · {} ({} sessions)", o.name, o.group, o.sessions)}
                                    </option>
                                }
                            })
                            .collect_view()}
                    </select>
                </label>
            </div>
            {match chosen {
                Some(c) => {
                    let strength: Vec<(i64, f64)> = c
                        .strength
                        .iter()
                        .map(|p| (p.day, p.e1rm))
                        .collect();
                    let mut series: Vec<charts::Series> = Vec::new();
                    series
                        .push(charts::Series {
                            name: c.exercise.clone(),
                            colour: "#4ade80",
                            points: c.sets.iter().map(|p| (p.day, p.sets)).collect(),
                        });
                    let palette = ["#fbbf24", "#60a5fa", "#a78bfa", "#f472b6", "#2dd4bf", "#fb923c"];
                    for (i, o) in c.overlaps.iter().enumerate() {
                        series
                            .push(charts::Series {
                                name: o.exercise.clone(),
                                colour: palette[i % palette.len()],
                                points: o.sets.iter().map(|p| (p.day, p.sets)).collect(),
                            });
                    }
                    let sessions = strength.len();
                    view! {
                        <div class=ROW>
                            <span class=OK>{format!("{} · {}", c.exercise, c.group)}</span>
                            <span class=DIM>{format!("{sessions} session(s) in the window")}</span>
                            <span class=DIM>{format!("{} overlapping exercise(s)", c.overlaps.len())}</span>
                        </div>
                        {if strength.len() < 2 {
                            view! {
                                <Hint text="Fewer than two sessions of this exercise in the window: a line needs two." />
                            }
                                .into_any()
                        } else {
                            view! {
                                <charts::Overlay
                                    strength=strength
                                    strength_unit="kg".to_string()
                                    volume=series
                                    volume_unit="sets/wk".to_string()
                                    deloads=a.deloads.clone()
                                />
                            }
                                .into_any()
                        }}
                    }
                        .into_any()
                }
                None => {
                    view! {
                        <Hint text="No exercise has two sessions yet, so there is no trend to draw. Record sets in two sessions first, or widen the time range." />
                    }
                        .into_any()
                }
            }}
            {if a.deloads.is_empty() {
                ().into_any()
            } else {
                view! {
                    <div class=ROW>
                        <span class=WARN>{format!(
                            "{} deload week(s) in the window, shaded rather than dropped",
                            a.deloads.len(),
                        )}</span>
                    </div>
                }
                    .into_any()
            }}
        </Section>
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
        <Bare title="status">
            <Suspense fallback=loading>
                {move || {
                    data.get()
                        .map(|r| match r {
                            Ok(i) => view! { <StatusView i=i /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Bare>
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
                        <td class="whitespace-normal">{i.db}</td>
                    </tr>
                    <tr>
                        <th>"remote"</th>
                        <td class="whitespace-normal">{i.remote}</td>
                    </tr>
                    <tr>
                        <th>"sessions"</th>
                        <td class=RIGHT>{i.sessions}</td>
                    </tr>
                    <tr>
                        <th>"sets"</th>
                        <td class=RIGHT>{i.sets}</td>
                    </tr>
                    <tr>
                        <th>"meals"</th>
                        <td class=RIGHT>{i.meals}</td>
                    </tr>
                    <tr>
                        <th>"weigh-ins"</th>
                        <td class=RIGHT>{i.weights}</td>
                    </tr>
                </tbody>
            </table>
        </Section>
    }
}
