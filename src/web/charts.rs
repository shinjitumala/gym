//! Charts, as inline SVG.
//!
//! No chart library: the shapes are a polyline and a stack of rectangles, and
//! writing them directly keeps the bundle small and the axes honest — every
//! chart here says what its range is, its dates are on the horizontal axis and
//! its numbers on the vertical one, and the ones that must not draw a line (too
//! few weigh-ins) simply do not.

use crate::web::dto::{PhaseSpan, Span};
use crate::web::ui::{day_full, day_label, DIM, OK, ROW, WARN};
use leptos::prelude::*;

/// The label colour. An SVG `text` fills black unless told otherwise, so every
/// axis label says so; the class's `text-gray-500` colours the element, not the
/// glyphs.
const LABEL: &str = "#6b7280";

const W: f64 = 720.0;
const H: f64 = 220.0;
/// Room for the axis numbers on the left, the day labels underneath, and the
/// line's own headroom at the top. The left margin holds a label like
/// `70.0 kg` right-aligned against the axis, so it never crosses the plot.
const LEFT: f64 = 58.0;
const RIGHT: f64 = 14.0;
const TOP: f64 = 10.0;
const BOTTOM: f64 = 24.0;

fn px(x: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        LEFT
    } else {
        LEFT + (x - min) / (max - min) * (W - LEFT - RIGHT)
    }
}

fn py(y: f64, min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        H / 2.0
    } else {
        H - BOTTOM - (y - min) / (max - min) * (H - TOP - BOTTOM)
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

/// Round tick values covering `min..max`, about `want` of them, at a
/// 1/2/5 × 10ⁿ step: a chart whose labels read 80, 82, 84 is read at a glance,
/// one whose labels read 80.37 is not.
fn ticks(min: f64, max: f64, want: usize) -> Vec<f64> {
    let span = max - min;
    if span <= 0.0 || want == 0 {
        return vec![min];
    }
    let raw = span / want as f64;
    let magnitude = 10f64.powf(raw.log10().floor());
    let normal = raw / magnitude;
    let multiplier = if normal <= 1.0 {
        1.0
    } else if normal <= 2.0 {
        2.0
    } else if normal <= 5.0 {
        5.0
    } else {
        10.0
    };
    let step = multiplier * magnitude;
    let mut out = Vec::new();
    let mut v = (min / step).ceil() * step;
    while v <= max + step * 1e-9 {
        out.push(v);
        v += step;
    }
    if out.is_empty() {
        out.push(min);
    }
    out
}

/// The vertical axis: a gridline and its number per tick. `decimals` is what the
/// numbers are rounded to, so a weight axis says `80.5 kg` and a sets axis says
/// `12 sets/wk`.
fn y_axis(y0: f64, y1: f64, unit: &str, decimals: usize, anchor_right: bool) -> Vec<AnyView> {
    // The left scale's numbers sit in the margin, right-aligned against the axis;
    // the right scale's sit inside, left-aligned against its own.
    let (x_text, anchor) = if anchor_right {
        (W - RIGHT - 4.0, "end")
    } else {
        (LEFT - 4.0, "end")
    };
    let label = |v: f64| {
        let number = format!("{v:.decimals$}");
        if unit.is_empty() {
            number
        } else {
            format!("{number} {unit}")
        }
    };
    ticks(y0, y1, 4)
        .into_iter()
        .map(|v| {
            let y = py(v, y0, y1);
            view! {
                <line
                    class="axis-grid"
                    stroke="#14532d"
                    stroke-width="1"
                    stroke-dasharray="2 3"
                    x1=LEFT
                    y1=y
                    x2=W - RIGHT
                    y2=y
                />
                <text
                    class="y-tick"
                    fill=LABEL
                    x=x_text
                    y=y + 3.0
                    text-anchor=anchor
                >
                    {label(v)}
                </text>
            }
            .into_any()
        })
        .collect()
}

/// The horizontal axis: four dates spread across the range, oldest first.
fn x_axis(x0: i64, x1: i64) -> Vec<AnyView> {
    let steps = 3;
    (0..=steps)
        .map(|i| {
            let day = x0 + (x1 - x0) * i / steps;
            let x = px(day as f64, x0 as f64, x1 as f64);
            let anchor = if i == 0 {
                "start"
            } else if i == steps {
                "end"
            } else {
                "middle"
            };
            view! {
                <text class="x-tick" fill=LABEL x=x y=H - 6.0 text-anchor=anchor>
                    {day_full(day)}
                </text>
            }
            .into_any()
        })
        .collect()
}

/// The deload weeks, as bands behind a chart's line.
///
/// Shading rather than dropping: the deload's points stay on the line, and the
/// band says why they are where they are. A week is a stretch of days, so the
/// band runs from the Monday to the Sunday, clipped to the chart's x range.
#[component]
fn Shade(#[prop(into)] spans: Vec<Span>, x0: f64, x1: f64) -> impl IntoView {
    spans
        .into_iter()
        .map(|s| {
            let left = px(s.from as f64, x0, x1).max(LEFT);
            let right = px((s.to + 1) as f64, x0, x1).min(W - RIGHT);
            view! {
                <rect
                    class="deload-week"
                    x=left
                    y=TOP
                    width=(right - left).max(2.0)
                    height=H - TOP - BOTTOM
                    fill="#3f2f0a"
                >
                    <title>{format!("deload week from {}", day_label(s.from))}</title>
                </rect>
            }
        })
        .collect_view()
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
    view! {
        <svg
            class="w-full h-auto block border border-green-900 bg-black text-gray-500 text-[9px]"
            viewBox=format!("0 0 {W} {H}")
            preserveAspectRatio="none"
        >
            <line stroke="#14532d" stroke-width="1" x1=LEFT y1=H - BOTTOM x2=W - RIGHT y2=H - BOTTOM />
            <line stroke="#14532d" stroke-width="1" x1=LEFT y1=TOP x2=LEFT y2=H - BOTTOM />
            {y_axis(y0, y1, &unit, 1, false)}
            {x_axis(x0 as i64, x1 as i64)}
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
    /// The deload weeks to shade behind the line.
    #[prop(optional, into)]
    deloads: Vec<Span>,
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
    let shaded = !deloads.is_empty();
    let spells = deloads.clone();
    view! {
        <svg
            class="w-full h-auto block border border-green-900 bg-black text-gray-500 text-[9px]"
            viewBox=format!("0 0 {W} {H}")
            preserveAspectRatio="none"
        >
            <Shade spans=spells x0=x0 x1=x1 />
            <line stroke="#14532d" stroke-width="1" x1=LEFT y1=H - BOTTOM x2=W - RIGHT y2=H - BOTTOM />
            <line stroke="#14532d" stroke-width="1" x1=LEFT y1=TOP x2=LEFT y2=H - BOTTOM />
            {y_axis(y0, y1, &unit, 1, false)}
            {x_axis(x0 as i64, x1 as i64)}
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
            {shaded
                .then(|| {
                    view! {
                        <span class=DIM>
                            <span style="color: #3f2f0a">"■ " </span>
                            "deload week"
                        </span>
                    }
                })}
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
    /// The deload weeks to shade behind the lines.
    #[prop(optional, into)]
    deloads: Vec<Span>,
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
    let shaded = !deloads.is_empty();
    let spells = deloads.clone();
    view! {
        <svg
            class="w-full h-auto block border border-green-900 bg-black text-gray-500 text-[9px]"
            viewBox=format!("0 0 {W} {H}")
            preserveAspectRatio="none"
        >
            <Shade spans=spells x0=x0 x1=x1 />
            <line stroke="#14532d" stroke-width="1" x1=LEFT y1=H - BOTTOM x2=W - RIGHT y2=H - BOTTOM />
            <line stroke="#14532d" stroke-width="1" x1=LEFT y1=TOP x2=LEFT y2=H - BOTTOM />
            <line stroke="#14532d" stroke-width="1" x1=W - RIGHT y1=TOP x2=W - RIGHT y2=H - BOTTOM />
            {y_axis(sy0, sy1, &strength_unit, 0, false)}
            {y_axis(vy0, vy1, &volume_unit, 0, true)}
            {x_axis(x0 as i64, x1 as i64)}
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
            {shaded
                .then(|| {
                    view! {
                        <span class=DIM>
                            <span class=WARN>"■ " </span>
                            "deload week"
                        </span>
                    }
                })}
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
    let bw = (W - LEFT - RIGHT) / n as f64;
    let offset = (W - LEFT - RIGHT - bw * bars.len() as f64) / 2.0;
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
            let mut y = H - BOTTOM;
            let mut segs: Vec<AnyView> = Vec::new();
            for (name, v) in &bar.parts {
                let h = (v / top) * (H - TOP - BOTTOM);
                y -= h;
                let colour =
                    PALETTE[names.iter().position(|n| n == name).unwrap_or(0) % PALETTE.len()];
                let title = format!("{} · {name} · {v:.0} {unit}", bar.label);
                let day = bar.day;
                segs.push(
                    view! {
                        <rect
                            x=LEFT + offset + i as f64 * bw
                            y=y
                            width=(bw - 1.0).max(1.0)
                            height=h.max(0.5)
                            fill=colour
                            class="intake-bar cursor-crosshair"
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
    // Three day labels spread over the bars, so the chart says when each end is
    // rather than only naming the first and the last.
    let marks: Vec<AnyView> = [0usize, bars.len() / 2, bars.len().saturating_sub(1)]
        .into_iter()
        .filter(|i| *i < bars.len())
        .map(|i| {
            let x = LEFT + offset + (i as f64 + 0.5) * bw;
            let anchor = if i == 0 {
                "start"
            } else if i + 1 == bars.len() {
                "end"
            } else {
                "middle"
            };
            view! {
                <text class="x-tick" fill=LABEL x=x y=H - 6.0 text-anchor=anchor>
                    {bars[i].label.clone()}
                </text>
            }
            .into_any()
        })
        .collect();
    view! {
        <svg
            class="w-full h-auto block border border-green-900 bg-black text-gray-500 text-[9px]"
            viewBox=format!("0 0 {W} {H}")
            preserveAspectRatio="none"
        >
            <line stroke="#14532d" stroke-width="1" x1=LEFT y1=H - BOTTOM x2=W - RIGHT y2=H - BOTTOM />
            {y_axis(0.0, top, &unit, 0, false)}
            {marks}
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
