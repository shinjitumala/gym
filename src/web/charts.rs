//! Charts, as inline SVG.
//!
//! No chart library: the shapes are a polyline and a stack of rectangles, and
//! writing them directly keeps the bundle small and the axes honest — every
//! chart here says what its range is, and the ones that must not draw a line
//! (too few weigh-ins) simply do not.

use crate::web::dto::PhaseSpan;
use crate::web::ui::{day_label, DIM, OK, ROW};
use leptos::prelude::*;

const W: f64 = 720.0;
const H: f64 = 200.0;
const PAD: f64 = 28.0;

fn px(x: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        PAD
    } else {
        PAD + (x - min) / (max - min) * (W - 2.0 * PAD)
    }
}

fn py(y: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        H / 2.0
    } else {
        H - PAD - (y - min) / (max - min) * (H - 2.0 * PAD)
    }
}

fn low(points: &[(i64, f64)]) -> f64 {
    points.iter().map(|(_, y)| *y).fold(f64::MAX, f64::min)
}

fn high(points: &[(i64, f64)]) -> f64 {
    points.iter().map(|(_, y)| *y).fold(f64::MIN, f64::max)
}

fn x_range(points: &[(i64, f64)]) -> (f64, f64) {
    let x0 = points.iter().map(|(x, _)| *x).fold(i64::MAX, i64::min) as f64;
    let x1 = points.iter().map(|(x, _)| *x).fold(i64::MIN, i64::max) as f64;
    (x0, x1)
}

fn path(points: &[(i64, f64)], x0: f64, x1: f64, y0: f64, y1: f64) -> String {
    points
        .iter()
        .map(|(x, y)| format!("{:.1},{:.1}", px(*x as f64, x0, x1), py(*y, y0, y1)))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A line chart of `points`, with a dashed fitted line when one was given.
#[component]
pub fn Line(
    #[prop(into)] points: Vec<(i64, f64)>,
    /// The fit the slope came from, drawn dashed over the observations.
    #[prop(optional)]
    fit: Vec<(i64, f64)>,
    #[prop(into)] unit: String,
) -> impl IntoView {
    let x0 = x_range(&points).0;
    let x1 = x_range(&points).1;
    let pad = ((high(&points) - low(&points)) * 0.1).max(0.5);
    let y0 = low(&points) - pad;
    let y1 = high(&points) + pad;
    let line = path(&points, x0, x1, y0, y1);
    let fitted = (!fit.is_empty()).then(|| path(&fit, x0, x1, y0, y1));
    let dots: Vec<(f64, f64)> = points
        .iter()
        .map(|(x, y)| (px(*x as f64, x0, x1), py(*y, y0, y1)))
        .collect();
    let first = x0 as i64;
    let last = x1 as i64;
    view! {
        <svg
            class="w-full h-auto block border border-green-900 bg-black text-gray-500 text-[9px]"
            viewBox=format!("0 0 {W} {H}")
            preserveAspectRatio="none"
        >
            <line stroke="#14532d" stroke-width="1" x1=PAD y1=H - PAD x2=W - PAD y2=H - PAD />
            <line stroke="#14532d" stroke-width="1" x1=PAD y1=PAD x2=PAD y2=H - PAD />
            <text x=PAD y=14.0>{format!("{:.1} {unit}", high(&points))}</text>
            <text x=PAD y=H - 8.0>{format!("{:.1} {unit}", low(&points))}</text>
            <text x=W / 2.0 - 20.0 y=H - 8.0>{day_label(first)}</text>
            <text x=W - PAD - 30.0 y=H - 8.0>{day_label(last)}</text>
            {fitted
                .map(|f| {
                    view! {
                        <polyline
                            fill="none"
                            stroke="#a16207"
                            stroke-width="1"
                            stroke-dasharray="4 3"
                            points=f
                        />
                    }
                })}
            <polyline fill="none" stroke="#4ade80" stroke-width="1.5" points=line />
            {dots
                .iter()
                .map(|(x, y)| view! { <circle fill="#4ade80" cx=*x cy=*y r=1.6 /> })
                .collect_view()}
        </svg>
    }
}

const BULK: &str = "#4ade80";
const CUT: &str = "#fbbf24";
const PLAIN: &str = "#60a5fa";

/// A weight line whose colour follows the phase it was trained in: a bulk in
/// green, a cut in amber, and anything before the first phase in blue.
#[component]
pub fn PhaseLine(
    #[prop(into)] points: Vec<(i64, f64)>,
    #[prop(into)] phases: Vec<PhaseSpan>,
    #[prop(into)] unit: String,
) -> impl IntoView {
    let x0 = x_range(&points).0;
    let x1 = x_range(&points).1;
    let pad = ((high(&points) - low(&points)) * 0.1).max(0.5);
    let y0 = low(&points) - pad;
    let y1 = high(&points) + pad;
    let colour_of = |day: i64| -> &'static str {
        phases
            .iter()
            .find(|p| day >= p.from && day <= p.to)
            .map(|p| if p.kind == "cut" { CUT } else { BULK })
            .unwrap_or(PLAIN)
    };
    // One segment per colour change: consecutive points of the same phase are
    // one polyline, and the boundary point joins both so the line has no gap.
    let mut segments: Vec<(&'static str, Vec<(i64, f64)>)> = Vec::new();
    for p in &points {
        let c = colour_of(p.0);
        match segments.last_mut() {
            Some((colour, pts)) if *colour == c => pts.push(*p),
            Some((_, pts)) => {
                let last = *pts.last().unwrap_or(p);
                segments.push((c, vec![last, *p]));
            }
            None => segments.push((c, vec![*p])),
        }
    }
    let paths: Vec<AnyView> = segments
        .into_iter()
        .map(|(colour, pts)| {
            view! {
                <polyline
                    fill="none"
                    stroke=colour
                    stroke-width="1.8"
                    points=path(&pts, x0, x1, y0, y1)
                />
            }
            .into_any()
        })
        .collect();
    let dots: Vec<AnyView> = points
        .iter()
        .map(|(x, y)| {
            view! {
                <circle
                    fill=colour_of(*x)
                    cx=px(*x as f64, x0, x1)
                    cy=py(*y, y0, y1)
                    r=1.6
                />
            }
            .into_any()
        })
        .collect();
    let first = x0 as i64;
    let last = x1 as i64;
    view! {
        <svg
            class="w-full h-auto block border border-green-900 bg-black text-gray-500 text-[9px]"
            viewBox=format!("0 0 {W} {H}")
            preserveAspectRatio="none"
        >
            <line stroke="#14532d" stroke-width="1" x1=PAD y1=H - PAD x2=W - PAD y2=H - PAD />
            <line stroke="#14532d" stroke-width="1" x1=PAD y1=PAD x2=PAD y2=H - PAD />
            <text x=PAD y=14.0>{format!("{:.1} {unit}", high(&points))}</text>
            <text x=PAD y=H - 8.0>{format!("{:.1} {unit}", low(&points))}</text>
            <text x=W / 2.0 - 20.0 y=H - 8.0>{day_label(first)}</text>
            <text x=W - PAD - 30.0 y=H - 8.0>{day_label(last)}</text>
            {paths}
            {dots}
        </svg>
        <div class=ROW>
            <span class=DIM>
                <span style="color: #4ade80">"■ " </span>
                "bulk"
            </span>
            <span class=DIM>
                <span style="color: #fbbf24">"■ " </span>
                "cut"
            </span>
            <span class=DIM>
                <span style="color: #60a5fa">"■ " </span>
                "before the first phase"
            </span>
        </div>
    }
}

/// One series in the overlay: a name, a colour and the points.
pub struct Series {
    pub name: String,
    pub colour: &'static str,
    pub points: Vec<(i64, f64)>,
}

/// Two scales in one frame: the chosen exercise's estimated 1RM as a thick
/// line on the left scale, and sets per week for it and its muscle-group
/// neighbours as thin lines on the right scale.
///
/// An overlay rather than two charts because the question it answers is
/// comparative: did the strength move when the volume moved.
#[component]
pub fn Overlay(
    #[prop(into)] strength: Vec<(i64, f64)>,
    #[prop(into)] strength_unit: String,
    #[prop(into)] volume: Vec<Series>,
    #[prop(into)] volume_unit: String,
) -> impl IntoView {
    let all: Vec<(i64, f64)> = strength
        .iter()
        .chain(volume.iter().flat_map(|s| s.points.iter()))
        .copied()
        .collect();
    let (x0, x1) = x_range(&all);
    let pad = ((high(&strength) - low(&strength)) * 0.1).max(0.5);
    let sy0 = low(&strength) - pad;
    let sy1 = high(&strength) + pad;
    let vmax = volume
        .iter()
        .flat_map(|s| s.points.iter().map(|(_, v)| *v))
        .fold(0.0, f64::max)
        .max(1.0);
    let vy0 = 0.0;
    let vy1 = vmax * 1.1;
    let first = x0 as i64;
    let last = x1 as i64;
    view! {
        <svg
            class="w-full h-auto block border border-green-900 bg-black text-gray-500 text-[9px]"
            viewBox=format!("0 0 {W} {H}")
            preserveAspectRatio="none"
        >
            <line stroke="#14532d" stroke-width="1" x1=PAD y1=H - PAD x2=W - PAD y2=H - PAD />
            <line stroke="#14532d" stroke-width="1" x1=PAD y1=PAD x2=PAD y2=H - PAD />
            <line stroke="#14532d" stroke-width="1" x1=W - PAD y1=PAD x2=W - PAD y2=H - PAD />
            <text x=PAD y=14.0>{format!("{:.0} {strength_unit}", sy1)}</text>
            <text x=PAD y=H - 8.0>{format!("{:.0} {strength_unit}", sy0)}</text>
            <text x=W - PAD - 44.0 y=14.0>{format!("{vmax:.0} {volume_unit}")}</text>
            <text x=W / 2.0 - 20.0 y=H - 8.0>{day_label(first)}</text>
            <text x=W - PAD - 30.0 y=H - 8.0>{day_label(last)}</text>
            {volume
                .iter()
                .map(|s| {
                    view! {
                        <polyline
                            fill="none"
                            stroke=s.colour
                            stroke-width="1"
                            stroke-dasharray="3 2"
                            points=path(&s.points, x0, x1, vy0, vy1)
                        />
                    }
                })
                .collect_view()}
            <polyline
                fill="none"
                stroke="#4ade80"
                stroke-width="2"
                points=path(&strength, x0, x1, sy0, sy1)
            />
        </svg>
        <div class=ROW>
            <span class=DIM>
                <span class=OK>"— " </span>
                "estimated 1RM"
            </span>
            {volume
                .iter()
                .map(|s| {
                    view! {
                        <span class=DIM>
                            <span style=format!("color: {}", s.colour)>"-- " </span>
                            {s.name.clone()}
                        </span>
                    }
                })
                .collect_view()}
        </div>
    }
}

/// One bar of the intake chart: a day and the foods that filled it.
#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    pub day: i64,
    pub label: String,
    pub parts: Vec<(String, f64)>,
}

const PALETTE: [&str; 8] = [
    "#4ade80", "#22c55e", "#16a34a", "#15803d", "#166534", "#fbbf24", "#60a5fa", "#a78bfa",
];

/// One bar per day, stacked by the food that supplied the value.
///
/// Hovering or clicking a bar reports its day, which is how the page shows
/// which foods made it: the chart says "what did I eat", not just "how much".
#[component]
pub fn Intake(
    #[prop(into)] bars: Vec<Bar>,
    #[prop(into)] unit: String,
    #[prop(into)] on_pick: Callback<i64>,
) -> impl IntoView {
    // A bar never fills the whole width: one day should look like one day.
    let n = bars.len().max(12);
    let totals: Vec<f64> = bars
        .iter()
        .map(|b| b.parts.iter().map(|(_, v)| *v).sum::<f64>())
        .collect();
    let top = totals.iter().copied().fold(0.0, f64::max).max(1.0);
    let bw = (W - 2.0 * PAD) / n as f64;
    let offset = (W - 2.0 * PAD - bw * bars.len() as f64) / 2.0;
    let mut names: Vec<String> = Vec::new();
    for b in &bars {
        for (name, _) in &b.parts {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    let rects: Vec<AnyView> = bars
        .iter()
        .enumerate()
        .map(|(i, bar)| {
            let mut y = H - PAD;
            let mut segs: Vec<AnyView> = Vec::new();
            for (name, v) in &bar.parts {
                let h = (v / top) * (H - 2.0 * PAD);
                y -= h;
                let colour =
                    PALETTE[names.iter().position(|n| n == name).unwrap_or(0) % PALETTE.len()];
                let title = format!("{} · {name} · {v:.0} {unit}", bar.label);
                let day = bar.day;
                segs.push(
                    view! {
                        <rect
                            x=PAD + offset + i as f64 * bw
                            y=y
                            width=(bw - 1.0).max(1.0)
                            height=h.max(0.5)
                            fill=colour
                            class="cursor-crosshair"
                            on:mouseenter=move |_| on_pick.run(day)
                            on:click=move |_| on_pick.run(day)
                        >
                            <title>{title}</title>
                        </rect>
                    }
                    .into_any(),
                );
            }
            view! { <g>{segs.collect_view()}</g> }.into_any()
        })
        .collect();
    let first = bars.first().map(|b| b.label.clone()).unwrap_or_default();
    let last = bars.last().map(|b| b.label.clone()).unwrap_or_default();
    view! {
        <svg
            class="w-full h-auto block border border-green-900 bg-black text-gray-500 text-[9px]"
            viewBox=format!("0 0 {W} {H}")
            preserveAspectRatio="none"
        >
            <line stroke="#14532d" stroke-width="1" x1=PAD y1=H - PAD x2=W - PAD y2=H - PAD />
            <text x=PAD y=14.0>{format!("{:.0} {unit}", top)}</text>
            <text x=W / 2.0 - 20.0 y=H - 8.0>{first}</text>
            <text x=W - PAD - 30.0 y=H - 8.0>{last}</text>
            {rects}
        </svg>
        <div class=ROW>
            {names
                .iter()
                .take(8)
                .enumerate()
                .map(|(i, n)| {
                    view! {
                        <span class=DIM>
                            <span style=format!("color: {}", PALETTE[i % PALETTE.len()])>"■ " </span>
                            {n.clone()}
                        </span>
                    }
                })
                .collect_view()}
        </div>
    }
}
