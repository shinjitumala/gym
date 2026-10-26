//! What to lift next, from what was lifted last.
//!
//! Effort is part of the comparison, not decoration. A set taken closer to
//! failure at the same load and reps is progress even though neither number
//! moved, so a recommendation that reads only load and reps would hold the bar
//! still for as long as the effort keeps climbing. See
//! [the training rules](../doc/src/features.md#recommendations-compare-effort-first).

/// The effort row that marks a set as a warmup.
///
/// A warmup is recorded like any other set and is not a rung on the ladder: it
/// is not taken near failure, so ranking it as an effort would read a limp
/// first set as a regression and a grinder as progress. The comparison skips
/// these sets by this name, so the ladder stays the three rungs — `fail`,
/// `form fail`, `hard` — whatever rank the row happens to carry.
pub const WARMUP: &str = "warmup";

/// Whether an effort name is the warmup marker.
pub fn is_warmup(effort: Option<&str>) -> bool {
    effort == Some(WARMUP)
}

/// The estimated one-rep max of a set: what a single would have to weigh to be
/// worth the same effort, from the rep-factor table.
///
/// The table is bounded at thirty reps: a set of forty is an endurance set, not
/// a set to estimate a maximum from, so it has no estimate rather than a guess.
/// It lives here, beside the rule that reads it, because the rule runs in the
/// browser as well as in the server and the host-only `util` module does not.
pub fn one_rep_max(load: f64, rep: f64) -> Option<f64> {
    // https://strengthlevel.com/one-rep-max-calculator
    let rep = rep.round() as i64;
    let f = match rep {
        1 => 100.0_f64,
        2 => 97.0_f64,
        3 => 94.0_f64,
        4 => 92.0_f64,
        5 => 89.0_f64,
        6 => 86.0_f64,
        7 => 83.0_f64,
        8 => 81.0_f64,
        9 => 78.0_f64,
        10 => 75.0_f64,
        11 => 73.0_f64,
        12 => 71.0_f64,
        13 => 70.0_f64,
        14 => 68.0_f64,
        15 => 67.0_f64,
        16 => 65.0_f64,
        17 => 64.0_f64,
        18 => 63.0_f64,
        19 => 61.0_f64,
        20 => 60.0_f64,
        21 => 59.0_f64,
        22 => 58.0_f64,
        23 => 57.0_f64,
        24 => 56.0_f64,
        25 => 55.0_f64,
        26 => 54.0_f64,
        27 => 53.0_f64,
        28 => 52.0_f64,
        29 => 51.0_f64,
        30 => 50.0_f64,
        _ => return None,
    };
    let r = load * 100.0_f64 / f;
    Some((r * 10.0).round() / 10.0)
}

/// One set as the comparison sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetEffort {
    pub load: f64,
    pub rep: f64,
    /// The effort's rank: a smaller number is closer to failure, so a smaller
    /// number is a harder set. `None` is "not recorded", which compares as
    /// unknown rather than as the hardest or the easiest.
    pub effort: Option<i64>,
    /// Recorded as a warmup, which no comparison and no count includes.
    pub warmup: bool,
}

impl SetEffort {
    fn estimated_max(&self) -> Option<f64> {
        one_rep_max(self.load, self.rep)
    }
}

/// The set that speaks for a session: the highest estimated 1RM among its
/// working sets, which is one number that compares a heavy single with a light
/// set of many.
///
/// A session of warmups alone has none. That is how a caller tells "this
/// session does not count" from "this session was 40 kg": the numbers to beat
/// are a working set, and a warmup never becomes one however many reps it has.
pub fn best(sets: &[SetEffort]) -> Option<SetEffort> {
    sets.iter()
        .filter(|s| !s.warmup)
        .filter_map(|s| Some((s.estimated_max()?, *s)))
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, s)| s)
}

/// What the next set should do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// Heavier or longer: the last set beat the one before it.
    Up,
    /// Repeat the last set: it neither beat nor fell behind.
    Hold,
    /// Lighter: the last set was a regression.
    Down,
}

impl Change {
    pub fn word(&self) -> &'static str {
        match self {
            Change::Up => "up",
            Change::Hold => "hold",
            Change::Down => "down",
        }
    }
}

/// The next set to attempt.
#[derive(Clone, Debug, PartialEq)]
pub struct Advice {
    pub change: Change,
    pub load: f64,
    pub rep: f64,
    /// The sentence the exercise window shows, so the reader does not have to
    /// reverse-engineer the numbers.
    pub why: String,
}

/// The smallest load change worth making, in kilograms.
pub const STEP: f64 = 2.5;

const EPS: f64 = 0.01;

/// How the last set compares with the one before it.
///
/// The order is effort, then load, then reps, with one guard: a lighter bar is
/// never progress, however hard the set felt. Effort comes first because a
/// harder set at the same load and reps is the progress a numbers-only
/// comparison cannot see.
///
/// A warmup never reaches this function: it is not a set taken near failure, so
/// [`best`] drops it before a session is compared. What is left is the three
/// rungs, and the four rules hold among them alone.
pub fn change(prev: SetEffort, last: SetEffort) -> Change {
    if last.load + EPS < prev.load {
        return Change::Down;
    }
    if let (Some(a), Some(b)) = (prev.effort, last.effort) {
        if b < a {
            return Change::Up;
        }
    }
    if last.load > prev.load + EPS {
        return Change::Up;
    }
    if last.rep > prev.rep + EPS {
        return Change::Up;
    }
    if let (Some(a), Some(b)) = (prev.effort, last.effort) {
        if b > a {
            return Change::Down;
        }
    }
    Change::Hold
}

/// The next set for an exercise, given the best set of the last session
/// (`last`) and of the session before it (`prev`).
///
/// `target_rep` is the phase's rep target. At or above it a successful set is
/// answered with load; below it, with one more rep, because a load jump before
/// the rep target is reached is how a session stops being repeatable.
pub fn recommend(
    prev: Option<SetEffort>,
    last: Option<SetEffort>,
    target_rep: i64,
) -> Option<Advice> {
    let last = last?;
    let target = target_rep.max(1) as f64;
    let prev = match prev {
        Some(p) => p,
        None => {
            return Some(Advice {
                change: Change::Hold,
                load: last.load,
                rep: last.rep,
                why: format!(
                    "First recorded session: repeat {:.1} kg × {:.0} and the next one has something to compare.",
                    last.load, last.rep
                ),
            })
        }
    };
    let change = change(prev, last);
    let (load, rep, why) = match change {
        Change::Up if last.rep >= target => (
            last.load + STEP,
            target,
            format!(
                "Last set {:.1} kg × {:.0} beat {:.1} kg × {:.0}: add {STEP:.1} kg and go back to the rep target.",
                last.load, last.rep, prev.load, prev.rep
            ),
        ),
        Change::Up => (
            last.load,
            last.rep + 1.0,
            format!(
                "Last set {:.1} kg × {:.0} beat {:.1} kg × {:.0}: one more rep at the same load.",
                last.load, last.rep, prev.load, prev.rep
            ),
        ),
        Change::Hold => (
            last.load,
            last.rep,
            format!(
                "Last set {:.1} kg × {:.0} matched the one before it: repeat it.",
                last.load, last.rep
            ),
        ),
        Change::Down => (
            (last.load - STEP).max(0.0),
            last.rep,
            format!(
                "Last set {:.1} kg × {:.0} fell behind {:.1} kg × {:.0}: drop {STEP:.1} kg and rebuild.",
                last.load, last.rep, prev.load, prev.rep
            ),
        ),
    };
    Some(Advice {
        change,
        load,
        rep,
        why,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(load: f64, rep: f64, effort: Option<i64>) -> SetEffort {
        SetEffort {
            load,
            rep,
            effort,
            warmup: false,
        }
    }

    fn warmup(load: f64, rep: f64) -> SetEffort {
        SetEffort {
            load,
            rep,
            effort: Some(7),
            warmup: true,
        }
    }

    #[test]
    fn effort_is_compared_before_the_numbers_it_shares() {
        // Same load, same reps, one step closer to failure: hard (3) to
        // form fail (1). Nothing but the effort changed, and that is progress.
        let a = recommend(
            Some(set(60.0, 8.0, Some(3))),
            Some(set(60.0, 8.0, Some(1))),
            8,
        );
        assert_eq!(a.expect("advice").change, Change::Up);
    }

    #[test]
    fn a_repeated_set_with_the_same_effort_holds() {
        let a = recommend(
            Some(set(60.0, 8.0, Some(3))),
            Some(set(60.0, 8.0, Some(3))),
            8,
        );
        assert_eq!(a.expect("advice").change, Change::Hold);
    }

    #[test]
    fn a_lighter_bar_is_not_progress_even_when_it_hurt_more() {
        let a = recommend(
            Some(set(60.0, 8.0, Some(3))),
            Some(set(57.5, 8.0, Some(0))),
            8,
        );
        assert_ne!(a.expect("advice").change, Change::Up);
    }

    #[test]
    fn at_the_rep_target_progress_becomes_load() {
        let a = recommend(
            Some(set(60.0, 8.0, Some(3))),
            Some(set(60.0, 8.0, Some(1))),
            8,
        )
        .expect("advice");
        assert_eq!(a.load, 62.5);
        assert_eq!(a.rep, 8.0);
    }

    #[test]
    fn below_the_rep_target_progress_becomes_a_rep() {
        let a = recommend(
            Some(set(60.0, 5.0, Some(3))),
            Some(set(60.0, 6.0, Some(3))),
            8,
        )
        .expect("advice");
        assert_eq!(a.load, 60.0);
        assert_eq!(a.rep, 7.0);
    }

    #[test]
    fn an_easier_set_at_the_same_numbers_is_a_regression() {
        let a = recommend(
            Some(set(60.0, 8.0, Some(3))),
            Some(set(60.0, 8.0, Some(5))),
            8,
        );
        assert_eq!(a.expect("advice").change, Change::Down);
    }

    #[test]
    fn a_first_session_sets_the_baseline_instead_of_advising_blind() {
        let a = recommend(None, Some(set(60.0, 8.0, Some(3))), 8).expect("advice");
        assert_eq!(a.change, Change::Hold);
        assert_eq!(a.load, 60.0);
    }

    #[test]
    fn no_set_means_no_advice() {
        assert_eq!(recommend(Some(set(60.0, 8.0, Some(3))), None, 8), None);
    }

    #[test]
    fn an_unrecorded_effort_is_not_treated_as_a_hard_set() {
        let a = recommend(Some(set(60.0, 8.0, None)), Some(set(60.0, 8.0, None)), 8);
        assert_eq!(a.expect("advice").change, Change::Hold);
    }

    #[test]
    fn a_warmup_never_becomes_the_set_a_session_is_compared_by() {
        // The warmup is the last set of the session and the highest estimated
        // 1RM in it — 60 kg × 20 is a 100 kg single, against the working set's
        // 86.4 — and it is still the working set that is compared.
        let session = [set(70.0, 8.0, Some(3)), warmup(60.0, 20.0)];
        let best = best(&session).expect("a working set");
        assert_eq!(best.load, 70.0);
        assert!(is_warmup(Some(WARMUP)));
    }

    #[test]
    fn a_session_of_warmups_alone_has_no_numbers_to_beat() {
        // Not "a session at 40 kg": nothing about it counts, so the comparison
        // reaches back to the last session that did some work.
        assert_eq!(best(&[warmup(40.0, 10.0), warmup(50.0, 5.0)]), None);
    }

    #[test]
    fn the_ladder_is_unchanged_when_a_warmup_sits_either_side_of_it() {
        let prev = best(&[warmup(40.0, 10.0), set(60.0, 8.0, Some(3))]).expect("a working set");
        let last = best(&[set(60.0, 8.0, Some(1)), warmup(40.0, 10.0)]).expect("a working set");
        assert_eq!(
            recommend(Some(prev), Some(last), 8).expect("advice").change,
            Change::Up,
            "a warmup must not make a harder working set read as a regression"
        );
    }
}
