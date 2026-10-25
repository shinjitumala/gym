//! The pieces every page is built from: the header, the tab bar, a section, a
//! key figure, and the number formatters.
//!
//! The look is the one on [the overhaul](overhaul) page: black background, a
//! green ramp, blocky 2px borders and no rounded corners. It lives in
//! `style/main.css`; these components only supply the class names.

use leptos::prelude::*;

/// A date that a chart or a list can show, from its day number.
pub fn day_label(day: i64) -> String {
    chrono::NaiveDate::from_num_days_from_ce_opt(day as i32)
        .map(|d| d.format("%m-%d").to_string())
        .unwrap_or_default()
}

pub fn kg(v: f64) -> String {
    format!("{v:.1}")
}

pub fn kcal(v: f64) -> String {
    format!("{v:.0}")
}

pub fn grams(v: f64) -> String {
    format!("{v:.0}")
}

pub fn one(v: f64) -> String {
    format!("{v:.1}")
}

/// `None` prints as an em dash, which is the page's way of saying "withheld".
pub fn opt(v: Option<f64>, f: fn(f64) -> String) -> String {
    match v {
        Some(v) => f(v),
        None => "—".to_string(),
    }
}

#[component]
pub fn Section(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <section class="card">
            <h2>{title}</h2>
            {children()}
        </section>
    }
}

#[component]
pub fn Hint(#[prop(into)] text: String) -> impl IntoView {
    view! { <div class="hint">{text}</div> }
}

#[component]
pub fn Kpi(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(into)] unit: String,
) -> impl IntoView {
    view! {
        <div class="kpi">
            <span class="label">{label}</span>
            <span>
                <span class="value">{value}</span>
                <span class="unit">{unit}</span>
            </span>
        </div>
    }
}

/// The tab bar. `here` is the route the page is on, so the tab can light up.
#[component]
pub fn Nav(#[prop(into)] here: String) -> impl IntoView {
    let tabs = [
        ("/food", "food"),
        ("/food/edit", "food edit"),
        ("/food/analysis", "food analysis"),
        ("/gym", "gym"),
        ("/gym/edit", "gym edit"),
        ("/gym/analysis", "gym analysis"),
    ];
    let sep = |lead: &'static str| {
        let lead = lead.to_string();
        view! { <span class="sep">{lead}</span> }
    };
    view! {
        <nav class="tabs">
            {tabs
                .iter()
                .map(|(href, label)| {
                    let on = *href == here;
                    view! {
                        <a href=*href class=if on { "on" } else { "" }>
                            {*label}
                        </a>
                    }
                })
                .collect_view()}
            {sep("·")}
            <a href="/status">"status"</a>
        </nav>
    }
}

/// The page frame: a header with the database it is reading, then the content.
#[component]
pub fn Page(
    #[prop(into)] title: String,
    #[prop(into)] here: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="app">
            <header class="bar">
                <span class="title">{title}</span>
                <span class="grow"></span>
                <span class="meta">{here.clone()}</span>
            </header>
            <Nav here=here />
            <main>{children()}</main>
        </div>
    }
}

/// The one way a page renders a failed server function.
pub fn failure(e: ServerFnError) -> impl IntoView {
    view! {
        <div class="error">
            <span class="label">"ERROR"</span>
            {e.to_string()}
        </div>
    }
}

pub fn loading() -> impl IntoView {
    view! { <p class="spinner">"Loading…"</p> }
}
