//! The numbers behind the analysis tabs.
//!
//! Everything here is pure: a slice of observations in, a number or `None` out.
//! `None` is how the UI says "not enough data" — the plan is explicit that the
//! app withholds a number it cannot support rather than printing a confident
//! slope from four mornings of water weight.

use chrono::NaiveDate;

/// One observation: a date and a value in kilograms.
pub type Point = (NaiveDate, f64);

/// A least-squares fit through a series of weigh-ins.
#[derive(Clone, Copy, Debug)]
pub struct Fit {
    /// Kilograms per day. Positive is gaining.
    pub slope: f64,
    /// Standard error of `slope`, the same unit.
    pub se: f64,
    /// Fitted kilograms on the first day of the series.
    pub intercept: f64,
    /// Observations the fit was made from.
    pub n: usize,
}

impl Fit {
    pub fn per_week(&self) -> f64 {
        self.slope * 7.0
    }

    /// One standard error either side of the slope, in kilograms per week.
    pub fn band_per_week(&self) -> f64 {
        self.se * 7.0
    }

    /// The fitted value on a given day, for drawing the line.
    pub fn at(&self, first: NaiveDate, day: NaiveDate) -> f64 {
        self.intercept + self.slope * (day - first).num_days() as f64
    }
}

/// A least-squares slope through `points`, or `None` when there are too few.
///
/// Three points is the minimum for a standard error at all; the caller decides
/// how many it wants before it will show a slope.
pub fn fit(points: &[Point]) -> Option<Fit> {
    if points.len() < 3 {
        return None;
    }
    let first = points.iter().map(|(d, _)| *d).min()?;
    let xs: Vec<f64> = points
        .iter()
        .map(|(d, _)| (*d - first).num_days() as f64)
        .collect();
    let ys: Vec<f64> = points.iter().map(|(_, v)| *v).collect();
    let n = points.len() as f64;
    let mx = xs.iter().sum::<f64>() / n;
    let my = ys.iter().sum::<f64>() / n;
    let sxx: f64 = xs.iter().map(|x| (x - mx).powi(2)).sum();
    if sxx <= f64::EPSILON {
        return None;
    }
    let sxy: f64 = xs.iter().zip(&ys).map(|(x, y)| (x - mx) * (y - my)).sum();
    let slope = sxy / sxx;
    let intercept = my - slope * mx;
    let sse: f64 = xs
        .iter()
        .zip(&ys)
        .map(|(x, y)| (y - (intercept + slope * x)).powi(2))
        .sum();
    let dof = n - 2.0;
    let se = if dof > 0.0 {
        (sse / dof / sxx).sqrt()
    } else {
        0.0
    };
    Some(Fit {
        slope,
        se,
        intercept,
        n: points.len(),
    })
}

/// What the app will show for a trend: a fit, plus how much of the minimum
/// number of weigh-ins was present.
#[derive(Clone, Debug)]
pub struct Trend {
    pub fit: Option<Fit>,
    /// Weigh-ins inside the window.
    pub have: usize,
    /// Weigh-ins the window needs before a slope is shown.
    pub need: usize,
    pub points: Vec<Point>,
}

impl Trend {
    pub fn shown(&self) -> Option<Fit> {
        self.fit
    }
}

/// Fit a window of weigh-ins, requiring `min_points` observations.
pub fn trend(points: &[Point], min_points: usize) -> Trend {
    let fit = fit(points).filter(|_| points.len() >= min_points);
    Trend {
        fit,
        have: points.len(),
        need: min_points,
        points: points.to_vec(),
    }
}

/// Which phase the log is in, from the `phase` row that was active.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Goal {
    /// Gaining: kilograms per month, positive.
    Bulk(f64),
    /// Losing or holding: on goal is a band at or below zero.
    Cut,
}

/// The verdict on the trend against the phase's goal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    /// The whole uncertainty band is inside the goal range.
    On,
    /// The whole band is outside it.
    Off,
    /// The band straddles the boundary, so the honest answer is "can't say yet".
    Uncertain,
}

/// A goal as an inclusive range in kilograms per week. `None` is unbounded
/// above (a cut).
pub fn goal_range(goal: Goal) -> (f64, Option<f64>) {
    match goal {
        Goal::Bulk(per_month) => {
            let per_week = per_month / 30.4375 * 7.0;
            (per_week * 0.5, Some(per_week * 1.5))
        }
        Goal::Cut => (f64::NEG_INFINITY, Some(0.0)),
    }
}

/// Compare a fit with a goal. A goal is met only when the whole band is inside
/// the range, not when the point estimate is.
pub fn verdict(fit: Fit, goal: Goal) -> Verdict {
    let (low, high) = goal_range(goal);
    let slope = fit.per_week();
    let lo = slope - fit.band_per_week();
    let hi = slope + fit.band_per_week();
    let above = |v: f64, h: Option<f64>| match h {
        Some(h) => v <= h,
        None => true,
    };
    if lo >= low && above(hi, high) {
        Verdict::On
    } else if hi < low || !above(lo, high) {
        Verdict::Off
    } else {
        Verdict::Uncertain
    }
}

/// Energy in a kilogram of body mass. The constant the plan names.
pub const KCAL_PER_KG: f64 = 7700.0;

/// A day's energy intake, and how much of the day was actually logged.
#[derive(Clone, Copy, Debug)]
pub struct DayIntake {
    pub date: NaiveDate,
    pub kcal: f64,
    /// A day below this is not a day's intake — a single snack — and is left
    /// out of the average that produces the maintenance estimate.
    pub logged: bool,
}

/// The intake below which a day is treated as unlogged.
pub const LOGGED_DAY_KCAL: f64 = 800.0;

impl DayIntake {
    pub fn new(date: NaiveDate, kcal: f64) -> Self {
        Self {
            date,
            kcal,
            logged: kcal >= LOGGED_DAY_KCAL,
        }
    }
}

/// Maintenance calories: the intake that would explain the observed weight
/// change. `intake + 7700 * kg/day`.
#[derive(Clone, Copy, Debug)]
pub struct Maintenance {
    pub kcal: f64,
    /// Logged days the estimate is made from.
    pub days: usize,
    /// Mean intake over those days.
    pub intake: f64,
    /// Weight change per day the estimate uses.
    pub slope: f64,
}

/// Estimate maintenance, or `None` when the window has too few logged days.
pub fn maintenance(days: &[DayIntake], fit: Fit, min_days: usize) -> Option<Maintenance> {
    let logged: Vec<&DayIntake> = days.iter().filter(|d| d.logged).collect();
    if logged.len() < min_days {
        return None;
    }
    let intake = logged.iter().map(|d| d.kcal).sum::<f64>() / logged.len() as f64;
    Some(Maintenance {
        kcal: intake - KCAL_PER_KG * fit.slope,
        days: logged.len(),
        intake,
        slope: fit.slope,
    })
}

/// A day whose intake is far from what the weight trend implies.
#[derive(Clone, Copy, Debug)]
pub struct Outlier {
    pub date: NaiveDate,
    pub kcal: f64,
    pub expected: f64,
    pub delta: f64,
}

/// How far a day's intake may sit from the weight-implied intake before it is
/// worth a human's look.
pub const OUTLIER_KCAL: f64 = 1000.0;

/// Days whose intake the weight trend does not explain.
///
/// The app flags days, never a food: without a controlled repeat, a mis-valued
/// food is not identifiable, and pointing at one would be a guess dressed as a
/// finding.
pub fn outliers(days: &[DayIntake], m: Maintenance) -> Vec<Outlier> {
    days.iter()
        .filter(|d| d.logged)
        .filter_map(|d| {
            let delta = d.kcal - m.kcal;
            (delta.abs() > OUTLIER_KCAL).then_some(Outlier {
                date: d.date,
                kcal: d.kcal,
                expected: m.kcal,
                delta,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    /// A series with a known slope and noise; 0.1 kg/day is 0.7 kg/week.
    fn synthetic(slope: f64, noise: &[f64]) -> Vec<Point> {
        noise
            .iter()
            .enumerate()
            .map(|(i, n)| {
                (
                    d("2026-01-01") + chrono::Duration::days(i as i64),
                    70.0 + slope * i as f64 + n,
                )
            })
            .collect()
    }

    #[test]
    fn a_known_slope_is_recovered() {
        let p = synthetic(0.1, &[0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
        let f = fit(&p).unwrap();
        assert!((f.per_week() - 0.7).abs() < 1e-9, "{}", f.per_week());
        assert!(f.se < 1e-9, "a perfect line has no uncertainty");
    }

    #[test]
    fn noise_widens_the_band_and_keeps_the_slope_near() {
        let clean = fit(&synthetic(0.05, &[0.0; 8])).unwrap();
        let noisy = fit(&synthetic(
            0.05,
            &[0.3, -0.3, 0.3, -0.3, 0.3, -0.3, 0.3, -0.3],
        ))
        .unwrap();
        assert!(
            noisy.band_per_week() > clean.band_per_week(),
            "{} vs {}",
            noisy.band_per_week(),
            clean.band_per_week()
        );
        assert!(
            (noisy.per_week() - 0.35).abs() < 0.3,
            "the slope stays in the same direction: {}",
            noisy.per_week()
        );
    }

    #[test]
    fn a_slope_needs_three_points_but_a_trend_needs_its_minimum() {
        let three = synthetic(0.02, &[0.0, 0.0, 0.0]);
        assert!(fit(&three).is_some());
        assert!(trend(&three, 14).shown().is_none());
        assert_eq!(trend(&three, 14).have, 3);
        assert_eq!(trend(&three, 14).need, 14);
        assert!(trend(&three, 3).shown().is_some());
        assert!(fit(&three[..2]).is_none());
    }

    #[test]
    fn the_fitted_line_goes_through_the_observations() {
        let p = synthetic(0.1, &[0.0, 0.0, 0.0]);
        let f = fit(&p).unwrap();
        let first = p[0].0;
        for (day, kg) in &p {
            assert!((f.at(first, *day) - kg).abs() < 1e-9);
        }
    }

    #[test]
    fn a_bulk_is_on_goal_only_when_the_band_is_inside_it() {
        let on = fit(&synthetic(0.033, &[0.0, 0.0, 0.0, 0.0])).unwrap();
        assert_eq!(verdict(on, Goal::Bulk(1.0)), Verdict::On);
        let off = fit(&synthetic(0.0, &[0.0, 0.0, 0.0, 0.0])).unwrap();
        assert_eq!(verdict(off, Goal::Bulk(1.0)), Verdict::Off);
    }

    #[test]
    fn a_band_that_straddles_the_goal_is_uncertain_rather_than_optimistic() {
        let p = synthetic(0.03, &[0.6, -0.6, 0.6, -0.6, 0.5, -0.5]);
        let f = fit(&p).unwrap();
        let v = verdict(f, Goal::Bulk(1.0));
        assert_eq!(v, Verdict::Uncertain, "{:?} {:?}", f, v);
    }

    #[test]
    fn a_cut_is_on_goal_at_or_below_zero() {
        let losing = fit(&synthetic(-0.02, &[0.0, 0.0, 0.0])).unwrap();
        assert_eq!(verdict(losing, Goal::Cut), Verdict::On);
        let gaining = fit(&synthetic(0.02, &[0.0, 0.0, 0.0])).unwrap();
        assert_eq!(verdict(gaining, Goal::Cut), Verdict::Off);
        let (_, high) = goal_range(Goal::Cut);
        assert_eq!(high, Some(0.0));
    }

    #[test]
    fn maintenance_is_withheld_until_enough_days_are_logged() {
        let fit = fit(&synthetic(0.0, &[0.0, 0.0, 0.0])).unwrap();
        let days = vec![
            DayIntake::new(d("2026-01-01"), 2200.0),
            DayIntake::new(d("2026-01-02"), 2100.0),
            DayIntake::new(d("2026-01-03"), 300.0), // a snack, not a day
        ];
        assert!(maintenance(&days, fit, 3).is_none(), "two logged days");
        assert!(maintenance(&days, fit, 2).is_some());
    }

    #[test]
    fn maintenance_accounts_for_the_weight_change() {
        let fit = fit(&synthetic(0.1, &[0.0, 0.0, 0.0])).unwrap();
        let days: Vec<DayIntake> = (0..5)
            .map(|i| DayIntake::new(d("2026-01-01") + chrono::Duration::days(i), 3000.0))
            .collect();
        let m = maintenance(&days, fit, 5).unwrap();
        assert!((m.kcal - (3000.0 - 770.0)).abs() < 1e-6, "{}", m.kcal);
        assert_eq!(m.days, 5);
    }

    #[test]
    fn a_day_far_from_the_trend_is_reported_and_a_normal_day_is_not() {
        let fit = fit(&synthetic(0.0, &[0.0, 0.0, 0.0])).unwrap();
        let days = vec![
            DayIntake::new(d("2026-01-01"), 2500.0),
            DayIntake::new(d("2026-01-02"), 2600.0),
            DayIntake::new(d("2026-01-03"), 4200.0),
        ];
        let m = maintenance(&days, fit, 3).unwrap();
        let o = outliers(&days, m);
        assert_eq!(o.len(), 1, "{o:?}");
        assert_eq!(o[0].date, d("2026-01-03"));
        assert!(o[0].delta > 1000.0);
    }
}
