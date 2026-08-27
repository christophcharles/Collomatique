//! Cohorts and the global processing order.
//!
//! A **cohort** is a maximal set of interchangeable students (§6.1). The key
//! is not just the profile — the set of lists containing the student — but
//! also the student's kept-list group memberships: two students in the same
//! lists but in *different* groups of a kept list have different frozen
//! partners, so swapping them changes the score and they are not
//! interchangeable. Zero-use kept lists carry no mass and are already dropped
//! by [`State::new`](super::state::State::new), so they never split a cohort
//! for nothing.
//!
//! A pleasant consequence of the key: `N_s` is uniform inside a cohort, which
//! is what makes the §7.1 tie-break "more list-uses" well defined per cohort.

use super::state::State;
use collomatique_state_colloscopes::StudentId;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

/// A set of interchangeable students, and the lists they must be placed in.
pub(super) struct Cohort {
    /// Ascending `StudentId` — the canonical member order (§6.3). Prefill
    /// fills claimed groups with this same order in every list, which is what
    /// makes the blocks prefix-align across lists.
    pub(super) members: Vec<StudentId>,
    /// Spec indices whose student set contains the members.
    pub(super) profile: BTreeSet<usize>,
}

/// Cohorts in the global processing order of §7.1: rarest first — ascending
/// cohort size, ties toward more list-uses, then ascending first member id.
///
/// Rationale: rare profiles have the fewest options for consistent partners
/// and must commit while the space is empty; the "takes everything standard"
/// students come last and are exactly the flexible ones. The same order
/// drives prefill and the greedy pass. It stays a free function called once
/// from the entry point — cheap insurance if it ever needs swapping, not a
/// configuration knob.
pub(super) fn ordered_cohorts(state: &State) -> Vec<Cohort> {
    let mut by_key: BTreeMap<(BTreeSet<usize>, Vec<(usize, usize)>), Vec<StudentId>> =
        BTreeMap::new();
    for student in state.universe() {
        let profile = state.profile(student);
        // A student known only through a kept list is never placed.
        if profile.is_empty() {
            continue;
        }
        let key = (profile.clone(), state.kept_memberships(student).to_vec());
        by_key.entry(key).or_default().push(student);
    }

    // `universe()` is ascending, so every member vector already is.
    let mut cohorts: Vec<Cohort> = by_key
        .into_iter()
        .map(|((profile, _kept), members)| Cohort { members, profile })
        .collect();
    cohorts.sort_by_key(|cohort| {
        let first = cohort.members[0];
        (cohort.members.len(), Reverse(state.n_uses(first)), first)
    });
    cohorts
}
