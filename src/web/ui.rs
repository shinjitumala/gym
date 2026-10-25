//! The pieces every page is built from: the header, the two-row tab bar, a
//! section, a key figure, a segmented switch, and the form helpers.
//!
//! The look is the one on [the overhaul](overhaul) page: black background, a
//! green ramp, blocky 2px borders and no rounded corners. It lives in
//! `style/main.css`; these components only supply the class names.
//!
//! The tab bar is two rows on purpose. The first row picks the **domain**
//! (food or gym), the second picks the **intent** (daily, edit, analysis), so
//! the six pages read as a grid rather than a list of six, and the row that
//! wraps on a phone is the short one.

use leptos::prelude::*;

// Tailwind classes as named constants. They are literal strings in the source,
// which is what the scanner reads: a class that only ever appeared after string
// concatenation would silently not be generated.

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
    [&_button]:rounded-none [&_button]:border-2 [&_button]:border-green-900 [&_button]:bg-black [&_button]:text-green-400 [&_button]:px-2 [&_button]:font-bold \
    [&_button:hover]:border-green-500 [&_button:hover]:bg-green-950 [&_button:disabled]:opacity-30 [&_button:disabled]:cursor-not-allowed \
    [&_input]:rounded-none [&_input]:bg-black [&_input]:border-2 [&_input]:border-green-900 [&_input]:text-green-300 [&_input]:px-1 [&_input]:min-w-0 [&_input]:max-w-full \
    [&_input:focus]:outline-none [&_input:focus]:border-green-500 \
    [&_select]:rounded-none [&_select]:bg-black [&_select]:border-2 [&_select]:border-green-900 [&_select]:text-green-200 [&_select]:px-1 [&_select]:min-w-0 [&_select]:max-w-full \
    [&_label]:flex [&_label]:flex-col [&_label]:gap-0.5 [&_label]:text-xs [&_label]:text-gray-500 [&_label]:min-w-0 \
    [&_table]:w-full [&_table]:tabular-nums [&_th]:text-left [&_th]:text-green-500 [&_th]:font-bold [&_th]:whitespace-nowrap \
    [&_th]:border-b [&_th]:border-green-900 [&_th]:pr-2 [&_td]:text-left [&_td]:border-b [&_td]:border-green-900 [&_td]:pr-2 [&_td]:whitespace-nowrap \
    [&_h2]:text-green-500 [&_h2]:font-bold [&_h2]:text-sm [&_form]:flex [&_form]:flex-col [&_form]:gap-2 [&_main]:flex-1 [&_main]:overflow-auto [&_main]:min-w-0 \
    [&_select]:w-full [&_input]:w-full";

const TABS: &str =
    "flex gap-1 px-2 py-1 border-b border-green-900 overflow-x-auto whitespace-nowrap";
const TAB: &str = "flex-none px-2 py-0.5 border-2 border-green-900 text-green-400 hover:border-green-500 hover:bg-green-950";
const TAB_ON: &str =
    "flex-none px-2 py-0.5 border-2 border-green-500 bg-green-950 text-green-500 font-bold";

fn domain_of(here: &str) -> &'static str {
    if here.starts_with("/gym") {
        "/gym"
    } else if here.starts_with("/status") {
        "/status"
    } else {
        "/food"
    }
}

fn intent_of(here: &str) -> &'static str {
    if here.ends_with("/edit") {
        "edit"
    } else if here.ends_with("/analysis") {
        "analysis"
    } else {
        "daily"
    }
}

/// The two rows: domain, then intent.
#[component]
pub fn Nav(#[prop(into)] here: String) -> impl IntoView {
    let domain = domain_of(&here);
    let intent = intent_of(&here);
    let domains = [("/food", "food"), ("/gym", "gym"), ("/status", "status")];
    let intents = [
        ("daily", "daily"),
        ("edit", "edit"),
        ("analysis", "analysis"),
    ];
    view! {
        <nav class=TABS>
            {domains
                .iter()
                .map(|(href, label)| {
                    let on = *href == domain;
                    view! {
                        <a href=*href class=if on { TAB_ON } else { TAB }>
                            {*label}
                        </a>
                    }
                })
                .collect_view()}
        </nav>
        {if domain == "/status" {
            ().into_any()
        } else {
            view! {
                <nav class="flex gap-1 px-2 py-1 pt-0 border-b-2 border-green-900 overflow-x-auto">
                    {intents
                        .iter()
                        .map(|(i, label)| {
                            let href = if *i == "daily" {
                                domain.to_string()
                            } else {
                                format!("{domain}/{i}")
                            };
                            let on = *i == intent;
                            view! {
                                <a href=href class=if on { TAB_ON } else { TAB }>
                                    {*label}
                                </a>
                            }
                        })
                        .collect_view()}
                </nav>
            }
                .into_any()
        }}
    }
}

/// The page frame: a header with the page it is on, then the tab rows.
#[component]
pub fn Page(
    #[prop(into)] title: String,
    #[prop(into)] here: String,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=APP_CLASS>
            <header class="flex items-center gap-2 px-2 py-1 border-b-2 border-green-900 bg-black">
                <span class="font-bold text-green-400 tracking-wide">{title}</span>
                <span class="grow"></span>
                <span class="text-gray-600 text-xs max-sm:hidden">{here.clone()}</span>
            </header>
            <Nav here=here />
            <main>{children()}</main>
        </div>
    }
}

/// A switch between the views of one page (meals and weight, sets and
/// exercises). It is client state, not a route: the six routes stay the six
/// routes, and a second axis of navigation would make the phone bar a menu.
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

pub fn checked(r: NodeRef<leptos::html::Input>) -> bool {
    read_check(r).unwrap_or(false)
}

/// Fill a date field, and optionally a time field, with the server's clock.
///
/// `now` is what the server said when it rendered the page. Reading the
/// browser's own clock would be a second source of truth for "now" and would
/// disagree with the server the moment the two are in different zones.
#[component]
pub fn NowButton(
    #[prop(optional)] date: Option<NodeRef<leptos::html::Input>>,
    #[prop(optional)] time: Option<NodeRef<leptos::html::Input>>,
    #[prop(into)] today: String,
    #[prop(into)] now: String,
) -> impl IntoView {
    let label = if time.is_some() { "now" } else { "today" };
    view! {
        <button type="button" class=NOW on:click=move |_| {
            #[cfg(feature = "hydrate")]
            {
                if let Some(r) = date {
                    if let Some(e) = r.get() {
                        e.set_value(&today);
                    }
                }
                if let Some(r) = time {
                    if let Some(e) = r.get() {
                        e.set_value(&now);
                    }
                }
            }
            #[cfg(not(feature = "hydrate"))]
            {
                let _ = (&date, &time, &today, &now);
            }
        }>
            {label}
        </button>
    }
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
