//! What to lift next, from what was lifted last.
//!
//! Effort is part of the comparison, not decoration. A set taken closer to
//! failure at the same load and reps is progress even though neither number
//! moved, so a recommendation that reads only load and reps would hold the bar
//! still for as long as the effort keeps climbing. See
//! [the training rules](../doc/src/features.md#recommendations-compare-effort-first).

/// One set as the comparison sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SetEffort {
    pub load: f64,
    pub rep: f64,
    /// The effort's rank: a smaller number is closer to failure, so a smaller
    /// number is a harder set. `None` is "not recorded", which compares as
    /// unknown rather than as the hardest or the easiest.
    pub effort: Option<i64>,
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
        SetEffort { load, rep, effort }
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
}
