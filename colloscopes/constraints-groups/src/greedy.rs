//! Greedy group-list generation — the primary generator.
//!
//! Design: `docs/plans/greedy_roadmap.md` (point 1) and
//! `docs/plans/greedy_algorithm.md`; the `§n` references below point into the
//! latter. One pass of prefill (whole groups tiled from single cohorts), then
//! one joint placement per remaining student, maximizing the total partner
//! **collision probability** — the chance that two of a student's grouping
//! decisions point at the same person, each meeting weighted by
//! `1 / (group size − 1)` so a meeting in a twelve-seat tutorial cannot buy
//! the right to scatter someone's colle partners.
//!
//! The greedy reads only `plan.specs` (with their covered pairs) and
//! `plan.kept_lists`. It ignores `ghost`, `canonical_range` and
//! `pinned_pairs` entirely — ILP-era machinery.

mod cohorts;
mod pass;
mod prefill;
mod state;
mod targets;

#[cfg(test)]
mod tests;

use crate::specs::GenerationPlan;
use collomatique_state_colloscopes::group_lists::GroupList;
use collomatique_state_colloscopes::{PeriodId, SubjectId};
use std::collections::BTreeSet;

/// Builds one prefilled `GroupList` per spec of the plan, in plan order,
/// paired with the (period, subject) pairs it must be associated to — exactly
/// the payload of `GroupListsUpdateOp::AddGeneratedGroupLists`, mirroring
/// [`build_group_lists`](crate::build_group_lists).
///
/// Always succeeds: the group targets are fixed upfront and sum to the
/// student count, so a free seat always exists and the hard constraints hold
/// unconditionally. `group_names` come out all `None`.
///
/// Panics if `names.len()` is not `plan.specs.len()`.
pub fn greedy_group_lists(
    plan: &GenerationPlan,
    names: &[String],
) -> Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)> {
    assert_eq!(
        names.len(),
        plan.specs.len(),
        "one name per spec is required"
    );

    let mut state = state::State::new(plan);
    let cohorts = cohorts::ordered_cohorts(&state);
    prefill::prefill(&mut state, &cohorts);
    // The cohorts are rarest first and their members ascending: the same
    // global order that drove prefill (§7.1). A student prefill placed in
    // every list of their profile is done; one whose profile also holds
    // non-claiming lists still enters, and only the missing groups are
    // chosen.
    for student in cohorts
        .iter()
        .flat_map(|cohort| cohort.members.iter().copied())
    {
        if !state.fully_placed(student) {
            pass::place_student(&mut state, student);
        }
    }
    state.into_group_lists(names)
}
