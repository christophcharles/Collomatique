//! The two constants the collision objective is built out of: how much one
//! meeting weighs, and how many list-uses a student takes part in.
//!
//! Both are read by the greedy's scoring (`greedy::state`), and both come
//! straight from the design (`docs/plans/greedy_algorithm.md` §2). They live
//! apart from the state that uses them because they are pure arithmetic over
//! a plan: nothing here knows about placements.

#[cfg(test)]
mod tests;

use crate::specs::GenerationPlan;
use collomatique_state_colloscopes::StudentId;
use std::collections::BTreeMap;

/// The mass one meeting in a group of `size` students puts on each partner,
/// for a list-use count of `uses` and a student taking part in `n_uses` uses
/// overall: `uses / (n_uses · (size − 1))` (§2.2).
///
/// Zero when the student sits alone there — nobody to put mass on — and zero
/// when `n_uses` is 0, the student whose every list serves no (period,
/// subject) pair: they are placed like anybody else, and score nothing.
///
/// `size` is the *target* size for a rebuilt list and the *actual* group size
/// for a kept one (§2.1): kept lists are user-made and may be unbalanced.
pub(crate) fn pair_mass(uses: usize, n_uses: usize, size: usize) -> f64 {
    if n_uses == 0 || size <= 1 {
        return 0.0;
    }
    uses as f64 / (n_uses as f64 * (size - 1) as f64)
}

/// `N_s` for every student of the plan: all of s's list-uses, rebuilt and kept
/// alike (the fixed-N convention of §2.2).
///
/// A kept list serving no (period, subject) pair is inert and is skipped, so
/// the keys are the plan's student universe: everyone a spec places, plus
/// everyone a *weighing* kept list groups.
pub(crate) fn plan_n_uses(plan: &GenerationPlan) -> BTreeMap<StudentId, usize> {
    let mut n_uses: BTreeMap<StudentId, usize> = BTreeMap::new();
    for (spec, covered) in &plan.specs {
        for &student in spec.students() {
            *n_uses.entry(student).or_default() += covered.len();
        }
    }
    for kept in &plan.kept_lists {
        if kept.use_count == 0 {
            continue;
        }
        for group in &kept.groups {
            for &student in group {
                *n_uses.entry(student).or_default() += kept.use_count;
            }
        }
    }
    n_uses
}
