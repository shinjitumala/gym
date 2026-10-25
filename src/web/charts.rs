//! Charts, as inline SVG.
//!
//! No chart library: the shapes are a polyline and a stack of rectangles, and
//! writing them directly keeps the bundle small and the axes honest — every
//! chart here says what its range is, and the ones that must not draw a line
//! (too few weigh-ins) simply do not.

use crate::web::ui::day_label;
use leptos::prelude::*;

pub struct Point {
    pub x: i64,
    pub y: f64,
}

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

/// A line chart of `points`, with optional dashed goal lines at `goals`
/// (kg/week) drawn around the series' own range.
#[component]
pub fn Line(
    #[prop(into)] points: Vec<(i64, f64)>,
    /// A flat band either side of the trend, in the same unit as the points.
    #[prop(optional)]
    band: Option<f64>,
    #[prop(into)] unit: String,
) -> impl IntoView {
    let xs: Vec<i64> = points.iter().map(|(x, _)| *x).collect();
    let ys: Vec<f64> = points.iter().map(|(_, y)| *y).collect();
    let x0 = xs.iter().copied().min().unwrap_or(0) as f64;
    let x1 = xs.iter().copied().max().unwrap_or(1) as f64;
    let pad = band.unwrap_or(0.0).max(
        (ys.iter().copied().fold(f64::MIN, f64::max) - ys.iter().copied().fold(f64::MAX, f64::min))
            * 0.1,
    );
    let y0 = ys.iter().copied().fold(f64::MAX, f64::min) - pad;
    let y1 = ys.iter().copied().fold(f64::MIN, f64::max) + pad;
    let line: Vec<String> = points
        .iter()
        .map(|(x, y)| format!("{:.1},{:.1}", px(*x as f64, x0, x1), py(*y, y0, y1)))
        .collect();
    let dots: Vec<(f64, f64)> = points
        .iter()
        .map(|(x, y)| (px(*x as f64, x0, x1), py(*y, y0, y1)))
        .collect();
    let first = xs.first().copied().unwrap_or(0);
    let last = xs.last().copied().unwrap_or(0);
    view! {
        <svg class="w-full h-auto block border border-green-900 bg-black text-gray-500 text-[9px]" viewBox=format!("0 0 {W} {H}") preserveAspectRatio="none">
            <line stroke="#14532d" stroke-width="1" x1=PAD y1=H - PAD x2=W - PAD y2=H - PAD />
            <line stroke="#14532d" stroke-width="1" x1=PAD y1=PAD x2=PAD y2=H - PAD />
            <text x=PAD y=14.0>
                {format!("{:.1} {unit}", ys.iter().copied().fold(f64::MIN, f64::max))}
            </text>
            <text x=PAD y=H - 8.0>
                {format!("{:.1} {unit}", ys.iter().copied().fold(f64::MAX, f64::min))}
            </text>
            <text x=W / 2.0 - 20.0 y=H - 8.0>{day_label(first)}</text>
            <text x=W - PAD - 30.0 y=H - 8.0>{day_label(last)}</text>
            <polyline fill="none" stroke="#4ade80" stroke-width="1.5" points=line.join(" ") />
            {dots
                .iter()
                .map(|(x, y)| view! { <circle fill="#4ade80" cx=*x cy=*y r=1.6 /> })
                .collect_view()}
        </svg>
    }
}

/// One vertical bar per day, each bar stacked by the foods that supplied it.
#[component]
pub fn Stacked(
    #[prop(into)] days: Vec<Vec<(String, f64)>>,
    #[prop(into)] labels: Vec<String>,
    #[prop(into)] unit: String,
) -> impl IntoView {
    // A bar never fills the whole width: one day should look like one day.
    let n = days.len().max(12);
    let totals: Vec<f64> = days
        .iter()
        .map(|d| d.iter().map(|(_, v)| *v).sum::<f64>())
        .collect();
    let top = totals.iter().copied().fold(0.0, f64::max).max(1.0);
    let bw = (W - 2.0 * PAD) / n as f64;
    let offset = (W - 2.0 * PAD - bw * days.len() as f64) / 2.0;
    let palette = [
        "#4ade80", "#22c55e", "#16a34a", "#15803d", "#166534", "#14532d", "#064e3b",
    ];
    let mut names: Vec<String> = Vec::new();
    for d in &days {
        for (name, _) in d {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
    }
    let bars: Vec<AnyView> = days
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let mut y = H - PAD;
            let mut segs: Vec<AnyView> = Vec::new();
            for (name, v) in d {
                let h = (v / top) * (H - 2.0 * PAD);
                y -= h;
                let colour =
                    palette[names.iter().position(|n| n == name).unwrap_or(0) % palette.len()];
                segs.push(
                    view! {
                        <rect
                            x=PAD + offset + i as f64 * bw
                            y=y
                            width=(bw - 1.0).max(1.0)
                            height=h.max(0.5)
                            fill=colour
                        />
                    }
                    .into_any(),
                );
            }
            view! { <g>{segs.collect_view()}</g> }.into_any()
        })
        .collect();
    let first = labels.first().cloned().unwrap_or_default();
    let last = labels.last().cloned().unwrap_or_default();
    view! {
        <svg class="w-full h-auto block border border-green-900 bg-black text-gray-500 text-[9px]" viewBox=format!("0 0 {W} {H}") preserveAspectRatio="none">
            <line stroke="#14532d" stroke-width="1" x1=PAD y1=H - PAD x2=W - PAD y2=H - PAD />
            <text x=PAD y=14.0>{format!("{:.0} {unit}", top)}</text>
            <text x=W / 2.0 - 20.0 y=H - 8.0>{first}</text>
            <text x=W - PAD - 30.0 y=H - 8.0>{last}</text>
            {bars}
        </svg>
        <div class="row">
            {names
                .iter()
                .take(7)
                .enumerate()
                .map(|(i, n)| {
                    view! {
                        <span class="dim">
                            <span style=format!("color: {}", palette[i % palette.len()])>"■ "</span>
                            {n.clone()}
                        </span>
                    }
                })
                .collect_view()}
        </div>
    }
}
