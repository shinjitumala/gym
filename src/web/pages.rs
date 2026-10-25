//! The six tabs.
//!
//! Each page is one `Resource` against a server function, a `Suspense`, and a
//! view that takes the finished data. Nothing here computes a statistic: the
//! numbers arrive already decided, which is what keeps the analysis honest and
//! the WASM bundle small.
//!
//! Pages with more than one subject separate them with a [`Segmented`] switch
//! rather than a route, so the six routes stay six and a phone does not get a
//! second navigation bar.

use crate::web::charts;
use crate::web::dto::*;
use crate::web::ui::*;
use crate::web::ui::{
    BAD, CHIP, CHIPS, DIM, FOLD, GROW, KPIS, LINE, NARROW, OK, PANEL, RECORD, RIGHT, ROW, SMALL,
    SMALL_BAD, WARN, WHEN,
};
use leptos::prelude::*;

// ---------------------------------------------------------------- food daily

#[component]
pub fn FoodDailyPage() -> impl IntoView {
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
    let (view, set_view) = signal("meals".to_string());
    let pick = Callback::new(move |v: String| set_view.set(v));
    let (meals, weight) = (d.clone(), d.clone());
    view! {
        <Segmented
            options=vec![("meals", "meals"), ("weight", "weight")]
            chosen=view
            on_pick=pick
        />
        <div class:hidden=move || view.get() != "meals">
            <MealsDaily d=meals saved=saved />
        </div>
        <div class:hidden=move || view.get() != "weight">
            <WeightDaily d=weight saved=saved />
        </div>
    }
}

#[component]
fn MealsDaily(d: FoodDaily, saved: Callback<()>) -> impl IntoView {
    let totals = d.totals.clone();
    let today_kcal = totals.kcal;
    let today = d.today.clone();
    let now = d.now.clone();
    let today2 = d.today.clone();
    let now2 = d.now.clone();
    view! {
        <Section title=format!("Today · {}", d.today.clone())>
            <div class=KPIS>
                <Kpi label="calories" value=kcal(totals.kcal) unit="kcal" />
                <Kpi label="protein" value=grams(totals.protein) unit="g" />
                <Kpi label="fat" value=grams(totals.fat) unit="g" />
                <Kpi label="carbohydrate" value=grams(totals.carbohydrate) unit="g" />
            </div>
            {match d.maintenance {
                Some(m) => {
                    let diff = today_kcal - m.kcal;
                    let klass = if diff >= 0.0 { "ok" } else { "bad" };
                    view! {
                        <div class=ROW>
                            <span class=DIM>
                                "maintenance "
                                <span class=OK>{kcal(m.kcal)}</span>
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
            <MealForm
                foods=d.foods.clone()
                today=today.clone()
                now=now.clone()
                saved=saved
            />
        </Section>

        {if d.presets.is_empty() {
            ().into_any()
        } else {
            view! {
                <Section title="Presets">
                    <Hint text="One tap records every food in the preset. Define them in edit → presets." />
                    <div class=CHIPS>
                        {d.presets
                            .iter()
                            .map(|p| {
                                view! {
                                    <PresetChip
                                        p=p.clone()
                                        today=today2.clone()
                                        now=now2.clone()
                                        saved=saved
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
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

/// Three rows, because a meal is rarely one thing. A row left on "—" is not
/// recorded, so the common case is still one click.
#[component]
fn MealForm(foods: Vec<FoodRow>, today: String, now: String, saved: Callback<()>) -> impl IntoView {
    let date = NodeRef::<leptos::html::Input>::new();
    let time = NodeRef::<leptos::html::Input>::new();
    let rows = [
        (
            NodeRef::<leptos::html::Select>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
        ),
        (
            NodeRef::<leptos::html::Select>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
        ),
        (
            NodeRef::<leptos::html::Select>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
        ),
    ];
    let empty = foods.is_empty();
    let _ = &foods;
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String, Vec<(i64, String, String)>)| {
        let (date, time, rows) = args.clone();
        async move { crate::web::api::act_add_meals(date, time, rows).await }
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
    let date0 = today.clone();
    let rows_for_submit = rows;
    let opts_for_rows = opts.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            let mut out = Vec::new();
            for (food, amount, note) in rows_for_submit {
                let id = selected(food, -1);
                let a = value_of(amount, "");
                let n = value_of(note, "");
                if id < 0 || a.trim().is_empty() {
                    continue;
                }
                out.push((id, a, n));
            }
            action.dispatch((value_of(date, &date0), value_of(time, ""), out));
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
            {rows
                .iter()
                .enumerate()
                .map(|(i, (food, amount, note))| {
                    let opts = opts_for_rows.clone();
                    let food = *food;
                    let amount = *amount;
                    let note = *note;
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
                                <input
                                    type="text"
                                    size="4"
                                    value=if i == 0 { "1" } else { "" }
                                    node_ref=amount
                                />
                            </label>
                            <label class=GROW>
                                "note"
                                <input type="text" size="10" value="" node_ref=note />
                            </label>
                        </div>
                    }
                })
                .collect_view()}
            <div class=ROW>
                <button type="submit" disabled=move || action.pending().get()>
                    "record"
                </button>
                <Outcome outcome=outcome />
            </div>
        </form>
        {if empty {
            view! { <Hint text="No foods yet: add one in edit → foods." /> }.into_any()
        } else {
            ().into_any()
        }}
    }
}

#[component]
fn WeightDaily(d: FoodDaily, saved: Callback<()>) -> impl IntoView {
    let date = NodeRef::<leptos::html::Input>::new();
    let kgin = NodeRef::<leptos::html::Input>::new();
    let bf = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String, String, String)| {
        let (date, kg, bf, note) = args.clone();
        async move { crate::web::api::act_add_weight(date, kg, bf, note).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let date0 = d.today.clone();
    view! {
        <Section title="Record a weigh-in">
            <form on:submit=move |e| {
                e.prevent_default();
                action
                    .dispatch((
                        value_of(date, &date0),
                        value_of(kgin, ""),
                        value_of(bf, ""),
                        value_of(note, ""),
                    ));
            }>
                <div class=WHEN>
                    <label>
                        "date"
                        <input type="date" value=d.today.clone() node_ref=date />
                    </label>
                    <NowButton date=date today=d.today.clone() now=d.now.clone() />
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
            </form>
            <Hint text="Leave body fat blank if you did not measure it: blank is stored as unknown, which is not the same as 0%." />
        </Section>

        <Section title="Weight">
            {match d.last_weight.clone() {
                Some(w) => {
                    view! {
                        <div class=KPIS>
                            <Kpi label="last weigh-in" value=kg(w.kg) unit="kg" />
                            <Kpi label="body fat" value=opt(w.bodyfat, one) unit="%" />
                        </div>
                        <div class=ROW>
                            <span class=DIM>{format!("{} · {}", w.date, w.note)}</span>
                        </div>
                    }
                        .into_any()
                }
                None => view! { <Hint text="No weigh-ins yet." /> }.into_any(),
            }}
            {trend_block(&d.trend)}
        </Section>

        {outliers_block(&d.outliers)}
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
    let goal = t.goal.clone();
    let count = points.len();
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
                    .into_iter()
                    .map(|m| {
                        view! {
                            <tr>
                                <td class=DIM>{m.time}</td>
                                <td>{m.food}</td>
                                <td class=RIGHT>{one(m.amount)}</td>
                                <td class=RIGHT>{kcal(m.kcal)}</td>
                                <td class="text-right max-sm:hidden">{grams(m.protein)}</td>
                                <td class="text-right max-sm:hidden">{grams(m.fat)}</td>
                                <td class="text-right max-sm:hidden">{grams(m.carbohydrate)}</td>
                                <td class="text-gray-500 whitespace-normal max-sm:hidden">{m.note}</td>
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
    let (tick, set_tick) = signal(0u32);
    let saved = Callback::new(move |()| set_tick.update(|t| *t += 1));
    let data = Resource::new(
        move || tick.get(),
        |_| async { crate::web::api::load_food_edit().await },
    );
    view! {
        <Page title="gym" here="/food/edit">
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
    let (meals, weights, foods, presets) = (d.clone(), d.clone(), d.clone(), d.clone());
    view! {
        <Segmented
            options=vec![
                ("meals", "meals"),
                ("weight", "weight"),
                ("foods", "foods"),
                ("presets", "presets"),
            ]
            chosen=view
            on_pick=pick
        />
        <div class:hidden=move || view.get() != "meals">
            <MealsEdit d=meals saved=saved />
        </div>
        <div class:hidden=move || view.get() != "weight">
            <WeightEditPanels d=weights saved=saved />
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
fn WeightEditPanels(d: FoodEdit, saved: Callback<()>) -> impl IntoView {
    let rows = d.weights.clone();
    view! {
        {if rows.is_empty() {
            view! { <Hint text="No weigh-ins in the window." /> }.into_any()
        } else {
            view! {
                {rows
                    .into_iter()
                    .map(|w| view! { <WeightEditRow w=w saved=saved /> })
                    .collect_view()}
            }
                .into_any()
        }}
        <AddWeightRow today=d.today.clone() saved=saved />
    }
}

#[component]
fn WeightEditRow(w: WeightRow, saved: Callback<()>) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let date = NodeRef::<leptos::html::Input>::new();
    let kgin = NodeRef::<leptos::html::Input>::new();
    let bf = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let save = Action::new(move |args: &(i64, String, String, String, String)| {
        let (id, date, kg, bf, note) = args.clone();
        async move { crate::web::api::act_edit_weight(id, date, kg, bf, note).await }
    });
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
    let (d0, k0, b0, n0) = (
        w.date.clone(),
        format!("{}", w.kg),
        w.bodyfat.map(|b| format!("{b}")).unwrap_or_default(),
        w.note.clone(),
    );
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=DIM>{w.date.clone()}</span>
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
fn AddWeightRow(today: String, saved: Callback<()>) -> impl IntoView {
    let date = NodeRef::<leptos::html::Input>::new();
    let kgin = NodeRef::<leptos::html::Input>::new();
    let bf = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String, String, String)| {
        let (date, kg, bf, note) = args.clone();
        async move { crate::web::api::act_add_weight(date, kg, bf, note).await }
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
        <form class="add" on:submit=move |e| {
            e.prevent_default();
            action
                .dispatch((
                    value_of(date, &date0),
                    value_of(kgin, ""),
                    value_of(bf, ""),
                    value_of(note, ""),
                ));
        }>
            <div class=ROW>
                <label>
                    "add a weigh-in"
                    <input type="date" value=today.clone() node_ref=date />
                </label>
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
                <span class="dim hide-narrow">{format!(
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
            <Hint text="A preset is a named set of foods with amounts, recorded in one tap on the daily tab." />
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
                        <th class=RIGHT>"times"</th>
                        <th class=RIGHT>"kcal each"</th>
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
                                    <td class=RIGHT>{f.eaten}</td>
                                    <td class=RIGHT>{one(f.per_unit.kcal)}</td>
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
                        <th class=RIGHT>"times"</th>
                        <th class=RIGHT>"kcal total"</th>
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
                                    <td class=RIGHT>{f.eaten}</td>
                                    <td class=RIGHT>{kcal(f.per_unit.kcal * f.eaten as f64)}</td>
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
    let (tick, set_tick) = signal(0u32);
    let saved = Callback::new(move |()| set_tick.update(|t| *t += 1));
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
                            Ok(d) => view! { <GymDailyView d=d saved=saved /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn GymDailyView(d: GymDaily, saved: Callback<()>) -> impl IntoView {
    let reminder = match (d.weeks_since_deload, d.deload_weeks) {
        (Some(w), want) if w >= want => format!(
            "{w} weeks since the last deload; the reminder is set to {want}. Entering one is still a choice.",
        ),
        (Some(w), want) => format!("{w} weeks since the last deload (reminder at {want})."),
        (None, want) => format!("No deload logged yet (reminder at {want} weeks)."),
    };
    let today_form = d.today.clone();
    let today_session = d.today.clone();
    let now_session = d.now.clone();
    let phase_line = match &d.phase {
        Some(p) => format!(
            "{} since {} · target {:.2} kg/week at {} reps",
            p.kind, p.start, p.target_rate, p.target_rep
        ),
        None => "no phase yet".to_string(),
    };
    view! {
        <Section title="Phase">
            <div class=ROW>
                <span class=OK>{phase_line}</span>
            </div>
            <PhaseForm today=today_form.clone() saved=saved />
        </Section>

        <Section title="Deload">
            <div class=ROW>
                <span class=DIM>{reminder}</span>
            </div>
        </Section>

        <Section title="Record a session">
            <SessionForm
                places=d.places.clone()
                exercises=d.exercises.clone()
                today=today_session.clone()
                now=now_session.clone()
                last_set=d.last_set
                saved=saved
            />
        </Section>

        <Section title="Recent sessions">
            {if d.sessions.is_empty() {
                view! { <Hint text="No sessions logged." /> }.into_any()
            } else {
                view! { <Sessions sessions=d.sessions.clone() /> }.into_any()
            }}
        </Section>
    }
}

#[component]
fn PhaseForm(today: String, saved: Callback<()>) -> impl IntoView {
    let kind = NodeRef::<leptos::html::Select>::new();
    let start = NodeRef::<leptos::html::Input>::new();
    let rate = NodeRef::<leptos::html::Input>::new();
    let rep = NodeRef::<leptos::html::Input>::new();
    let weight = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let action = Action::new(move |args: &(String, String, String, String, String)| {
        let (kind, start, rate, rep, weight) = args.clone();
        async move { crate::web::api::act_set_phase(kind, start, rate, rep, weight).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    let start0 = today.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            action
                .dispatch((
                    read_select(kind).unwrap_or_else(|| "bulk".to_string()),
                    value_of(start, &start0),
                    value_of(rate, "1"),
                    value_of(rep, "8"),
                    value_of(weight, ""),
                ));
        }>
            <div class=ROW>
                <label>
                    "phase"
                    <select node_ref=kind>
                        <option value="bulk">"bulk"</option>
                        <option value="cut">"cut"</option>
                    </select>
                </label>
                <label>
                    "from"
                    <input type="date" value=today.clone() node_ref=start />
                </label>
                <label class=NARROW>
                    "kg/month"
                    <input type="text" size="4" value="1" node_ref=rate />
                </label>
                <label class=NARROW>
                    "reps"
                    <input type="text" size="3" value="8" node_ref=rep />
                </label>
                <label class=NARROW>
                    "target kg"
                    <input type="text" size="4" value="" node_ref=weight />
                </label>
                <button type="submit" disabled=move || action.pending().get()>
                    "start phase"
                </button>
            </div>
            <Outcome outcome=outcome />
            <Hint text="Starting a phase ends the old one from that date. Past sessions keep the phase they were trained in." />
        </form>
    }
}

#[component]
fn SessionForm(
    places: Vec<PlaceRow>,
    exercises: Vec<ExerciseRow>,
    today: String,
    now: String,
    last_set: i64,
    saved: Callback<()>,
) -> impl IntoView {
    let place = NodeRef::<leptos::html::Select>::new();
    let date = NodeRef::<leptos::html::Input>::new();
    let time = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let deload = NodeRef::<leptos::html::Input>::new();
    let rows = [
        (
            NodeRef::<leptos::html::Select>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
        ),
        (
            NodeRef::<leptos::html::Select>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
        ),
        (
            NodeRef::<leptos::html::Select>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
        ),
        (
            NodeRef::<leptos::html::Select>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
            NodeRef::<leptos::html::Input>::new(),
        ),
    ];
    let first_place = places.first().map(|p| p.id).unwrap_or(0);
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let (seen, set_seen) = signal(Option::<i64>::None);
    let action = Action::new(
        move |args: &(
            i64,
            String,
            String,
            String,
            bool,
            Vec<(i64, String, String, String)>,
        )| {
            let (place, date, time, note, deload, sets) = args.clone();
            async move { crate::web::api::act_add_session(place, date, time, note, deload, sets).await }
        },
    );
    let history = Action::new(move |args: &(i64, i64)| {
        let (ex, pl) = *args;
        async move { crate::web::api::load_exercise_history(ex, pl).await }
    });
    Effect::new(move |_| {
        if let Some(r) = action.value().get() {
            if r.is_ok() {
                saved.run(());
            }
            set_outcome.set(Some(r));
        }
    });
    // An exercise whose name is blank cannot be chosen or read, so it is not
    // offered: buying a row by accident is worse than one extra tap.
    let ex_opts: Vec<(i64, String)> = exercises
        .iter()
        .filter(|e| !e.name.trim().is_empty())
        .map(|e| (e.id, e.name.clone()))
        .collect();
    let ex_for_rows = ex_opts.clone();
    let place_for_rows = places.clone();
    let last = last_set;
    let date0 = today.clone();
    view! {
        <form on:submit=move |e| {
            e.prevent_default();
            let pid = selected(place, first_place);
            let pl = pid;
            let mut sets = Vec::new();
            for (ex, load, rep, note) in rows {
                let id = selected(ex, -1);
                let l = value_of(load, "");
                let r = value_of(rep, "");
                if id < 0 || (l.trim().is_empty() && r.trim().is_empty()) {
                    continue;
                }
                let _ = pl;
                sets.push((id, l, r, value_of(note, "")));
            }
            action
                .dispatch((
                    pid,
                    value_of(date, &date0),
                    value_of(time, ""),
                    value_of(note, ""),
                    checked(deload),
                    sets,
                ));
        }>
            <div class=WHEN>
                <label>
                    "place"
                    <select node_ref=place>
                        {place_for_rows
                            .iter()
                            .map(|p| view! { <option value=p.id.to_string()>{p.name.clone()}</option> })
                            .collect_view()}
                    </select>
                </label>
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
                <label class=GROW>
                    "note"
                    <input type="text" size="16" value="" node_ref=note />
                </label>
                <label class="flex-row! items-center gap-1">
                    <input type="checkbox" node_ref=deload />
                    "deload session"
                </label>
            </div>

            {rows
                .iter()
                .enumerate()
                .map(|(i, (ex, load, rep, note))| {
                    let opts = ex_for_rows.clone();
                    let ex = *ex;
                    let load = *load;
                    let rep = *rep;
                    let note = *note;
                    // Picking an exercise asks for its own recent sessions;
                    // the panel shows that exercise only.
                    let picked = Callback::new(move |e: leptos::ev::Event| {
                        let id = event_target_value(&e).parse().unwrap_or(0);
                        let pl = selected(place, first_place);
                        set_seen.set(Some(id));
                        history.dispatch((id, pl));
                    });
                    view! {
                        <div class=LINE>
                            <label class=GROW>
                                "exercise"
                                <select node_ref=ex on:change=move |e| picked.run(e)>
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
                                "load"
                                <input type="text" size="5" value="" node_ref=load />
                            </label>
                            <label class=NARROW>
                                "reps"
                                <input type="text" size="4" value="" node_ref=rep />
                            </label>
                            <label class=GROW>
                                "note"
                                <input type="text" size="10" value="" node_ref=note />
                            </label>
                        </div>
                    }
                })
                .collect_view()}

            <div class=ROW>
                <button type="submit" disabled=move || action.pending().get()>
                    "record session"
                </button>
                <Outcome outcome=outcome />
            </div>
            <Hint text="A row left on — is not recorded. Picking an exercise shows the last three sessions of it at this place." />
        </form>

        <div class:hidden=move || seen.get().is_none()>
            {move || {
                history
                    .value()
                    .get()
                    .map(|r| match r {
                        Ok(h) => view! { <HistoryPanel h=h /> }.into_any(),
                        Err(e) => failure(e).into_any(),
                    })
            }}
        </div>
        <input type="hidden" value=last.to_string() />
    }
}

#[component]
fn HistoryPanel(h: History) -> impl IntoView {
    view! {
        <div class=PANEL>
            <div class=ROW>
                <span class=OK>{format!("recent · {}", h.exercise)}</span>
            </div>
            {if h.sessions.is_empty() {
                view! { <Hint text="No earlier session of this exercise here." /> }.into_any()
            } else {
                view! {
                    {h.sessions
                        .iter()
                        .map(|s| {
                            view! {
                                <div class=ROW>
                                    <span class=DIM>{format!("{} · {}", s.date, s.place)}</span>
                                    <span class=GROW>
                                        {s.sets
                                            .iter()
                                            .map(|x| format!("{}×{}", one(x.load), one(x.rep)))
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
                                <th class="text-right max-sm:hidden">"#"</th>
                                <th>"exercise"</th>
                                <th class=RIGHT>"load"</th>
                                <th class=RIGHT>"reps"</th>
                                <th class="max-sm:hidden">"note"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {s.sets
                                .iter()
                                .map(|x| {
                                    view! {
                                        <tr>
                                            <td class="num dim hide-narrow">{x.position}</td>
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

// ------------------------------------------------------------------- gym edit

#[component]
pub fn GymEditPage() -> impl IntoView {
    let (tick, set_tick) = signal(0u32);
    let saved = Callback::new(move |()| set_tick.update(|t| *t += 1));
    let data = Resource::new(
        move || tick.get(),
        |_| async { crate::web::api::load_gym_edit().await },
    );
    view! {
        <Page title="gym" here="/gym/edit">
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
    let (view, set_view) = signal("sets".to_string());
    let pick = Callback::new(move |v: String| set_view.set(v));
    let (sets, exercises, places, groups) = (d.clone(), d.clone(), d.clone(), d.clone());
    view! {
        <Segmented
            options=vec![
                ("sets", "sets"),
                ("exercises", "exercises"),
                ("places", "places"),
                ("groups", "groups"),
            ]
            chosen=view
            on_pick=pick
        />
        <div class:hidden=move || view.get() != "sets">
            <Section title="Recent sessions">
                <Hint text="Edit a set's load, reps or note, or delete it. Deleting closes the gap in the session's order." />
                <SessionsEdit d=sets saved=saved />
            </Section>
        </div>
        <div class:hidden=move || view.get() != "exercises">
            <ExercisePanels d=exercises saved=saved />
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
fn SessionsEdit(d: GymEdit, saved: Callback<()>) -> impl IntoView {
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
                    {s.sets
                        .iter()
                        .map(|x| view! { <SetEditRow x=x.clone() saved=saved /> })
                        .collect_view()}
                }
            })
            .collect_view()}
    }
}

#[component]
fn SetEditRow(x: SetRow, saved: Callback<()>) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let load = NodeRef::<leptos::html::Input>::new();
    let rep = NodeRef::<leptos::html::Input>::new();
    let note = NodeRef::<leptos::html::Input>::new();
    let (outcome, set_outcome) = signal(None::<Result<String, ServerFnError>>);
    let save = Action::new(move |args: &(i64, String, String, String)| {
        let (id, load, rep, note) = args.clone();
        async move { crate::web::api::act_edit_set(id, load, rep, note).await }
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
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=GROW>{x.exercise.clone()}</span>
                <span class=RIGHT>{format!("{} × {}", one(x.load), one(x.rep))}</span>
                <span class="text-gray-500 whitespace-normal max-sm:hidden">{x.note.clone()}</span>
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
    view! {
        <div class=RECORD>
            <div class=ROW>
                <span class=GROW>{e.name.clone()}</span>
                <span class=if e.groups.is_empty() { DIM } else { "" }>
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

// --------------------------------------------------------------- gym analysis

#[component]
pub fn GymAnalysisPage() -> impl IntoView {
    let picker = Action::new(move |ex: &Option<i64>| {
        let ex = *ex;
        async move { crate::web::api::load_exercise_picker(ex).await }
    });
    // The first load picks the server's default, so the tab says something
    // before the reader chooses.
    Effect::new(move |_| picker.dispatch(None));
    let volume = Resource::new(
        || (),
        |_| async { crate::web::api::load_gym_analysis().await },
    );
    view! {
        <Page title="gym" here="/gym/analysis">
            <Section title="Strength">
                <Hint text="Best estimated 1RM per session, from the rep-factor table. Sets above 30 reps are excluded. Pick an exercise to see its trend." />
                {move || {
                    picker
                        .value()
                        .get()
                        .map(|r| match r {
                            Ok(p) => view! { <Picker p=p picker=picker /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Section>
            <Suspense fallback=loading>
                {move || {
                    volume
                        .get()
                        .map(|r| match r {
                            Ok(v) => view! { <Volume v=v /> }.into_any(),
                            Err(e) => failure(e).into_any(),
                        })
                }}
            </Suspense>
        </Page>
    }
}

#[component]
fn Picker(
    p: ExercisePicker,
    picker: Action<Option<i64>, Result<ExercisePicker, ServerFnError>>,
) -> impl IntoView {
    let chosen = p.chosen.clone();
    view! {
        <div class=ROW>
            <label class=GROW>
                "exercise"
                <select
                    on:change=move |e| {
                        let id = event_target_value(&e).parse::<i64>().ok();
                        picker.dispatch(id);
                    }
                >
                    {p.exercises
                        .iter()
                        .map(|x| {
                            let sel = chosen.as_ref().map(|c| c.exercise == x.name).unwrap_or(false);
                            view! {
                                <option value=x.id.to_string() selected=sel>
                                    {format!("{} · {} ({})", x.name, x.group, x.sessions)}
                                </option>
                            }
                        })
                        .collect_view()}
                </select>
            </label>
        </div>
        {match chosen {
            Some(s) => {
                let points: Vec<(i64, f64)> = s.points.iter().map(|p| (p.day, p.e1rm)).collect();
                view! {
                    <div class=ROW>
                        <span class=OK>{format!("{} · {}", s.exercise, s.group)}</span>
                        <span class=DIM>{format!("{} sessions", s.points.len())}</span>
                    </div>
                    <charts::Line points=points unit="kg" />
                }
                    .into_any()
            }
            None => view! { <Hint text="No exercise has two sessions yet, so there is no trend to draw." /> }.into_any(),
        }}
    }
}

#[component]
fn Volume(v: GymAnalysis) -> impl IntoView {
    view! {
        <Section title="Sets per muscle group per week">
            {if v.volume.is_empty() {
                view! { <Hint text="No exercise has a muscle group yet, so there is no volume to show." /> }
                    .into_any()
            } else {
                view! {
                    <table>
                        <thead>
                            <tr>
                                <th>"muscle group"</th>
                                <th class=RIGHT>"weeks"</th>
                                <th class=RIGHT>"sets/week (last 4)"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {v.volume
                                .iter()
                                .map(|s| {
                                    let last: f64 = s
                                        .points
                                        .iter()
                                        .rev()
                                        .take(4)
                                        .map(|p| p.sets)
                                        .sum::<f64>()
                                        / s.points.len().min(4).max(1) as f64;
                                    view! {
                                        <tr>
                                            <td>{s.group.clone()}</td>
                                            <td class="text-right text-gray-500">{s.points.len()}</td>
                                            <td class=RIGHT>{format!("{last:.1}")}</td>
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
        {if v.ungrouped.is_empty() {
            ().into_any()
        } else {
            view! {
                <Section title="Ungrouped exercises">
                    <Hint text=format!(
                        "{} exercise(s) have no muscle group and appear in no body-part trend.",
                        v.ungrouped.len(),
                    ) />
                    <div class=ROW>
                        {v.ungrouped
                            .iter()
                            .map(|e| view! { <span class=DIM>{e.clone()}</span> })
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
