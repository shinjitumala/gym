//! The pieces every page is built from: the header, the three tabs, the bottom
//! action row, a section, a key figure, a segmented switch, and the form
//! helpers.
//!
//! The look is the one on [the overhaul](overhaul) page: black background, a
//! green ramp, blocky 2px borders and no rounded corners. It lives in
//! `style/main.scss`; these components only supply the class names.
//!
//! The tab bar has exactly three tabs — **gym**, **food**, **weight** — because
//! those are the three things the log records. A mode's **edit** and **analyse**
//! pages are not tabs: they are buttons at the bottom of the mode's daily page,
//! so the bar never grows a second row and a phone never gets a menu.

use leptos::prelude::*;

/// A flexible row of fields or figures.
pub const ROW: &str = "flex flex-wrap items-end gap-2";
/// A row that starts a new record.
pub const LINE: &str =
    "flex flex-wrap items-end gap-2 border-t border-dashed border-green-900 pt-1";
/// The date/time row of a form.
pub const WHEN: &str = "flex flex-wrap items-end gap-2 pb-1 [&>label]:basis-24 [&>label]:grow";
pub const GROW: &str = "grow min-w-0 basis-full sm:basis-32";
pub const NARROW: &str = "shrink-0 basis-16";
pub const CARD: &str = "border border-green-900 p-2 flex flex-col gap-1";
pub const HINT: &str = "text-gray-500 text-xs";
pub const KPIS: &str = "flex flex-wrap gap-4";
pub const KPI: &str = "flex flex-col min-w-18";
pub const CHIPS: &str = "flex flex-wrap gap-1 max-[380px]:flex-col max-[380px]:items-stretch";
pub const CHIP: &str = "flex items-center gap-1";
pub const RECORD: &str = "flex flex-col gap-1 border-b border-green-900 py-1";
pub const PANEL: &str = "border border-green-900 p-1 flex flex-col gap-0.5";
pub const FOLD: &str = "flex flex-col gap-1";
pub const SMALL: &str = "text-xs font-normal px-1.5";
pub const SMALL_BAD: &str = "text-xs font-normal px-1.5 text-red-400 border-red-500";
pub const RIGHT: &str = "text-right";
pub const DIM: &str = "text-gray-500";
pub const OK: &str = "text-green-400";
pub const BAD: &str = "text-red-400";
pub const WARN: &str = "text-amber-400";
pub const NOW: &str = "text-green-500";
pub const SEGMENTED: &str = "flex flex-wrap [&_button]:grow [&_button]:font-normal [&_button]:text-gray-500 [&_button]:border-r-0 [&_button:last-child]:border-r-2 [&_button.on]:font-bold [&_button.on]:text-green-500 [&_button.on]:bg-green-950";
pub const ERROR: &str = "border-2 border-red-500 bg-red-950 text-gray-200 px-2 py-1";
/// The list a picker shows: one button per row, full width, newest first.
pub const PICKER: &str = "flex flex-col gap-0.5 max-h-64 overflow-auto";
pub const PICK: &str =
    "flex items-baseline gap-2 text-left font-normal [&>span:last-child]:ml-auto [&>span:last-child]:text-gray-500";

/// A date that a chart or a list can show, from its day number.
pub fn day_label(day: i64) -> String {
    chrono::NaiveDate::from_num_days_from_ce_opt(day as i32)
        .map(|d| d.format("%m-%d").to_string())
        .unwrap_or_default()
}

/// The same day with its year, for an axis whose labels are far apart.
pub fn day_full(day: i64) -> String {
    chrono::NaiveDate::from_num_days_from_ce_opt(day as i32)
        .map(|d| d.format("%Y-%m-%d").to_string())
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

/// How long ago a `YYYY-MM-DD` was, in words, or `None` for a row never used.
pub fn ago(day: &str, today: &str) -> Option<String> {
    let a = chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()?;
    let b = chrono::NaiveDate::parse_from_str(today, "%Y-%m-%d").ok()?;
    let n = (b - a).num_days();
    Some(match n {
        0 => "today".to_string(),
        1 => "yesterday".to_string(),
        n if n < 7 => format!("{n} days ago"),
        n if n < 30 => format!("{}w ago", n / 7),
        _ => format!("{}mo ago", n / 30),
    })
}

#[component]
pub fn Section(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <section class=CARD>
            <h2>{title}</h2>
            {children()}
        </section>
    }
}

#[component]
pub fn Hint(#[prop(into)] text: String) -> impl IntoView {
    view! { <div class=HINT>{text}</div> }
}

#[component]
pub fn Kpi(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(into)] unit: String,
) -> impl IntoView {
    view! {
        <div class=KPI>
            <span class="text-xs text-gray-500">{label}</span>
            <span>
                <span class="text-green-400 text-xl font-bold tabular-nums">{value}</span>
                <span class="text-xs text-gray-500 ml-0.5">{unit}</span>
            </span>
        </div>
    }
}

/// The base look: black, a green ramp, blocky controls, no rounded corners.
/// Descendant selectors set the defaults once, so markup only says what is
/// different about a given control.
const APP_CLASS: &str = "relative bg-black h-dvh flex flex-col overflow-hidden text-gray-400 text-sm accent-green-500 max-w-[100vw] \
    scrollbar-thin scrollbar-thumb-green-900 scrollbar-track-black \
    [&_button]:rounded-none [&_button]:border-2 [&_button]:border-green-900 [&_button]:bg-black [&_button]:text-green-400 [&_button]:px-2 [&_button]:font-bold \
    [&_button:hover]:border-green-500 [&_button:hover]:bg-green-950 [&_button:disabled]:opacity-30 [&_button:disabled]:cursor-not-allowed \
    [&_input]:rounded-none [&_input]:bg-black [&_input]:border-2 [&_input]:border-green-900 [&_input]:text-green-300 [&_input]:px-1 [&_input]:min-w-0 [&_input]:max-w-full \
    [&_input:focus]:outline-none [&_input:focus]:border-green-500 \
    [&_select]:rounded-none [&_select]:bg-black [&_select]:border-2 [&_select]:border-green-900 [&_select]:text-green-200 [&_select]:px-1 [&_select]:min-w-0 [&_select]:max-w-full \
    [&_label]:flex [&_label]:flex-col [&_label]:gap-0.5 [&_label]:text-xs [&_label]:text-gray-500 [&_label]:min-w-0 \
    [&_table]:w-full [&_table]:tabular-nums [&_th]:text-left [&_th]:text-green-500 [&_th]:font-bold [&_th]:whitespace-nowrap \
    [&_th]:border-b [&_th]:border-green-900 [&_th]:pr-2 [&_td]:text-left [&_td]:border-b [&_td]:border-green-900 [&_td]:pr-2 [&_td]:whitespace-nowrap \
    [&_h2]:text-green-500 [&_h2]:font-bold [&_h2]:text-sm [&_form]:flex [&_form]:flex-col [&_form]:gap-2 [&_main]:flex-1 [&_main]:overflow-auto [&_main]:min-w-0 \
    [&_main]:scrollbar-gutter-stable \
    [&_select]:w-full [&_input]:w-full";

const TABS: &str =
    "flex gap-1 px-2 py-1 border-b border-green-900 overflow-x-auto whitespace-nowrap";
const TAB: &str = "flex-none px-2 py-0.5 border-2 border-green-900 text-green-400 hover:border-green-500 hover:bg-green-950";
const TAB_ON: &str =
    "flex-none px-2 py-0.5 border-2 border-green-500 bg-green-950 text-green-500 font-bold";
const ACTIONS: &str =
    "flex gap-1 px-2 py-1 border-t-2 border-green-900 overflow-x-auto whitespace-nowrap";

/// The three modes, in the order a day uses them.
const MODES: [(&str, &str); 3] = [("/gym", "gym"), ("/food", "food"), ("/weight", "weight")];

fn mode_of(here: &str) -> &'static str {
    MODES
        .iter()
        .find(|(href, _)| here.starts_with(href))
        .map(|(href, _)| *href)
        .unwrap_or("/food")
}

/// Which page of a mode this is: `daily`, `edit` or `analyse`.
fn page_of(here: &str) -> &'static str {
    if here.ends_with("/edit") {
        "edit"
    } else if here.ends_with("/analyse") {
        "analyse"
    } else {
        "daily"
    }
}

/// Every page of a mode, and the href for it.
fn href(mode: &str, page: &str) -> String {
    if page == "daily" {
        mode.to_string()
    } else {
        format!("{mode}/{page}")
    }
}

/// The three tabs.
#[component]
pub fn Nav(#[prop(into)] here: String) -> impl IntoView {
    let mode = mode_of(&here);
    view! {
        <nav class=TABS>
            {MODES
                .iter()
                .map(|(href, label)| {
                    let on = *href == mode;
                    let class = if on { TAB_ON } else { TAB };
                    view! {
                        <a href=*href class=class>
                            {*label}
                        </a>
                    }
                })
                .collect_view()}
        </nav>
    }
}

/// The buttons at the bottom: the other pages of this mode.
///
/// Food has no analysis page of its own — the intake chart lives under weight,
/// beside the trend it belongs with — so food's row has one button.
#[component]
pub fn Actions(#[prop(into)] here: String) -> impl IntoView {
    let mode = mode_of(&here);
    let page = page_of(&here);
    let mut links: Vec<(String, &'static str)> = Vec::new();
    if page != "daily" {
        links.push((href(mode, "daily"), "session"));
    }
    if page != "edit" {
        links.push((href(mode, "edit"), "edit"));
    }
    if page != "analyse" && mode != "/food" {
        links.push((href(mode, "analyse"), "analyse"));
    }
    view! {
        <nav class=ACTIONS>
            {links
                .into_iter()
                .map(|(href, label)| view! { <a href=href class=TAB>{label}</a> })
                .collect_view()}
        </nav>
    }
}

/// The page frame: a header with the page it is on, the three tabs, the page,
/// and the way to the mode's other pages.
///
/// The frame is fixed rather than conditional on the page: a conditional at the
/// root of a hydrated view stands for sibling nodes the server renders through
/// a `<template>`, and a browser that rewrites the document on the way out
/// leaves nothing for the client to hydrate against. A page without tabs gets
/// its own frame instead.
#[component]
pub fn Page(
    #[prop(into)] title: String,
    #[prop(into)] here: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=APP_CLASS>
            <Header title=title />
            <Nav here=here.clone() />
            <main class="flex flex-col gap-2 p-2">{children()}</main>
            <Actions here=here.clone() />
        </div>
    }
}

/// A page with no tabs: the header and the page, nothing else. It is what the
/// status page uses.
#[component]
pub fn Bare(#[prop(into)] title: String, children: Children) -> impl IntoView {
    view! {
        <div class=APP_CLASS>
            <Header title=title />
            <main class="flex flex-col gap-2 p-2">{children()}</main>
        </div>
    }
}

#[component]
fn Header(#[prop(into)] title: String) -> impl IntoView {
    view! {
        <header class="flex items-center gap-2 px-2 py-1 border-b-2 border-green-900 bg-black">
            <span class="font-bold text-green-400 tracking-wide">{title}</span>
            <span class="grow"></span>
            <a href="/status" class="text-gray-600 text-xs hover:text-green-500">
                "status"
            </a>
        </header>
    }
}

/// A switch between the views of one page. It is client state, not a route.
#[component]
pub fn Segmented(
    #[prop(into)] options: Vec<(&'static str, &'static str)>,
    #[prop(into)] chosen: ReadSignal<String>,
    #[prop(into)] on_pick: Callback<String>,
) -> impl IntoView {
    view! {
        <div class=SEGMENTED>
            {options
                .into_iter()
                .map(|(key, label)| {
                    let k = key.to_string();
                    let k2 = k.clone();
                    let on = move || chosen.get() == k2;
                    let pick = k.clone();
                    view! {
                        <button
                            type="button"
                            class:on=on
                            on:click=move |_| on_pick.run(pick.clone())
                        >
                            {label}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// The value a browser currently holds for `r`.
///
/// A form field is DOM state: a submit reads what the user sees rather than a
/// mirrored signal, which can lag behind (autofill, IME composition, a harness
/// setting the value). The fields are therefore uncontrolled — an initial
/// `value` and nothing reactive — so typing is never fought by a re-render.
#[cfg(feature = "hydrate")]
pub fn read(r: NodeRef<leptos::html::Input>) -> Option<String> {
    r.get().map(|e| e.value())
}

#[cfg(not(feature = "hydrate"))]
pub fn read(_r: NodeRef<leptos::html::Input>) -> Option<String> {
    None
}

#[cfg(feature = "hydrate")]
pub fn read_select(r: NodeRef<leptos::html::Select>) -> Option<String> {
    r.get().map(|e| e.value())
}

#[cfg(not(feature = "hydrate"))]
pub fn read_select(_r: NodeRef<leptos::html::Select>) -> Option<String> {
    None
}

#[cfg(feature = "hydrate")]
pub fn read_check(r: NodeRef<leptos::html::Input>) -> Option<bool> {
    r.get().map(|e| e.checked())
}

#[cfg(not(feature = "hydrate"))]
pub fn read_check(_r: NodeRef<leptos::html::Input>) -> Option<bool> {
    None
}

/// A field's value: what the browser holds, else the initial the page rendered.
pub fn value_of(r: NodeRef<leptos::html::Input>, initial: &str) -> String {
    read(r).unwrap_or_else(|| initial.to_string())
}

pub fn selected(r: NodeRef<leptos::html::Select>, initial: i64) -> i64 {
    read_select(r)
        .and_then(|v| v.parse().ok())
        .unwrap_or(initial)
}

pub fn chosen(r: NodeRef<leptos::html::Select>) -> String {
    read_select(r).unwrap_or_default()
}

pub fn checked(r: NodeRef<leptos::html::Input>) -> bool {
    read_check(r).unwrap_or(false)
}

/// Fill a date field, and optionally a time field, with the server's clock.
///
/// The clock is read **when the button is pressed**, not when the page was
/// rendered: a page left open for minutes would otherwise fill the fields with
/// the minute it was opened at. Asking the server for it keeps one clock — the
/// machine that stores the log — so the browser reading its own cannot disagree
/// across a timezone, and a phone whose clock is wrong writes the same minutes
/// the rest of the log is in.
#[component]
pub fn NowButton(
    #[prop(optional)] date: Option<NodeRef<leptos::html::Input>>,
    #[prop(optional)] time: Option<NodeRef<leptos::html::Input>>,
) -> impl IntoView {
    let label = if time.is_some() { "now" } else { "today" };
    view! {
        <button type="button" class=NOW on:click=move |_| {
            #[cfg(feature = "hydrate")]
            fill_from_the_server(date, time);
            #[cfg(not(feature = "hydrate"))]
            {
                let _ = (&date, &time);
            }
        }>
            {label}
        </button>
    }
}

/// Ask the server for its clock, and fill the fields when the answer arrives.
///
/// The press starts the read; the fields are filled a round trip later, not
/// during the click.
#[cfg(feature = "hydrate")]
fn fill_from_the_server(
    date: Option<NodeRef<leptos::html::Input>>,
    time: Option<NodeRef<leptos::html::Input>>,
) {
    leptos::task::spawn_local(async move {
        let Ok(c) = crate::web::api::load_clock().await else {
            return;
        };
        if let Some(e) = date.and_then(|r| r.get()) {
            e.set_value(&c.date);
        }
        if let Some(e) = time.and_then(|r| r.get()) {
            e.set_value(&c.time);
        }
    });
}

/// The one way a page renders a failed server function.
pub fn failure(e: ServerFnError) -> impl IntoView {
    view! {
        <div class=ERROR>
            <span class="text-red-300 font-bold">"ERROR"</span>
            {e.to_string()}
        </div>
    }
}

pub fn loading() -> impl IntoView {
    view! { <p class="text-green-400">"Loading…"</p> }
}

/// A one-line result: the server's message, or its error.
#[component]
pub fn Outcome(outcome: ReadSignal<Option<Result<String, ServerFnError>>>) -> impl IntoView {
    view! {
        {move || {
            outcome
                .get()
                .map(|r| match r {
                    Ok(m) => view! { <span class=OK>{m}</span> }.into_any(),
                    Err(e) => view! { <span class=BAD>{e.to_string()}</span> }.into_any(),
                })
        }}
    }
}

/// Does a row answer what was typed into a search box?
///
/// The query is split on whitespace and every word must be found in the row's
/// text, in any order — so `press chest` finds `bench press · chest`, which a
/// plain substring test misses. A word is found when its letters appear as a
/// run, or, failing that, in order with letters missing, so `bnch` still finds
/// `bench press`. Case is ignored.
pub fn narrows(text: &str, query: &str) -> bool {
    let text = text.to_lowercase();
    query.split_whitespace().all(|word| {
        let word = word.to_lowercase();
        text.contains(&word) || letters_in_order(&text, &word)
    })
}

/// Every letter of `word` in `text`, in order and not necessarily adjacent.
fn letters_in_order(text: &str, word: &str) -> bool {
    let mut letters = text.chars();
    word.chars().all(|w| letters.any(|c| c == w))
}

/// A searchable picker: a filter box over a list of buttons.
///
/// The list arrives already sorted — by recency, for the pickers that are about
/// what you did last — and the filter only narrows it. Nothing re-sorts in the
/// browser, so the order the server chose is the order the reader sees.
#[component]
pub fn Picker(
    #[prop(into)] label: String,
    #[prop(into)] placeholder: String,
    /// `(value, label, note)` per row; `note` is the right-hand hint.
    #[prop(into)]
    rows: Vec<(i64, String, String)>,
    #[prop(into)] on_pick: Callback<i64>,
) -> impl IntoView {
    let (filter, set_filter) = signal(String::new());
    let rows = StoredValue::new(rows);
    view! {
        <label>
            {label}
            <input
                type="text"
                placeholder=placeholder
                on:input=move |e| set_filter.set(event_target_value(&e))
            />
        </label>
        <div class=PICKER>
            {move || {
                let f = filter.get();
                let on_pick = on_pick;
                rows.with_value(|rows| {
                    rows.iter()
                        .filter(|(_, name, _)| narrows(name, &f))
                        .map(|(id, name, note)| {
                            let id = *id;
                            let name = name.clone();
                            let note = note.clone();
                            view! {
                                <button
                                    type="button"
                                    class=PICK
                                    on:click=move |_| on_pick.run(id)
                                >
                                    <span class=OK>{name}</span>
                                    <span>{note}</span>
                                </button>
                            }
                        })
                        .collect_view()
                })
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_query_keeps_every_row() {
        assert!(narrows("bench press", ""));
        assert!(narrows("bench press", "   "));
    }

    #[test]
    fn words_match_in_any_order() {
        assert!(narrows("bench press · chest", "press chest"));
        assert!(narrows("bench press · chest", "chest press"));
        assert!(!narrows("bench press · chest", "press back"));
    }

    #[test]
    fn a_word_may_lose_letters_but_not_their_order() {
        assert!(narrows("bench press", "bnch"));
        assert!(narrows("incline press", "inc prs"));
        assert!(!narrows("bench press", "hcneb"));
    }

    #[test]
    fn case_is_ignored_on_both_sides() {
        assert!(narrows("Bench Press", "ben"));
        assert!(narrows("bench press", "PRESS"));
    }
}
