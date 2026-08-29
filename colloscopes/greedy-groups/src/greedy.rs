//! Greedy group-list generation — the primary generator.
//!
//! One pass of prefill (whole groups tiled from single cohorts), then
//! one joint placement per remaining student, maximizing the total partner
//! **collision probability** — the chance that two of a student's grouping
//! decisions point at the same person, each meeting weighted by
//! `1 / (group size − 1)` so a meeting in a twelve-seat tutorial cannot buy
//! the right to scatter someone's colle partners.
//!
//! The greedy reads the whole plan: `plan.specs` (with their covered pairs)
//! and `plan.kept_lists`, which is all a plan holds.

mod cohorts;
mod pass;
mod prefill;
mod state;

#[cfg(test)]
mod tests;

use crate::specs::GenerationPlan;
use collomatique_state_colloscopes::group_lists::GroupList;
use collomatique_state_colloscopes::{PeriodId, StudentId, SubjectId};
use std::collections::BTreeSet;
use std::time::Instant;

/// Builds one prefilled `GroupList` per spec of the plan, in plan order,
/// paired with the (period, subject) pairs it must be associated to — exactly
/// the payload of `GroupListsUpdateOp::AddGeneratedGroupLists`.
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
    greedy_group_lists_with_log(plan, names, &mut |_: &str| {})
}

/// [`greedy_group_lists`], reporting on `log` as it goes.
///
/// The two phases are told apart and timed separately: they answer different
/// questions when a result is surprising — how much of the class prefill froze
/// as whole groups, and what the joint placement then had left to decide. The
/// pass reports coarse progress, at most ten lines whatever the class size,
/// since it is the only part whose cost grows with the student count.
pub fn greedy_group_lists_with_log(
    plan: &GenerationPlan,
    names: &[String],
    log: &mut (dyn FnMut(&str) + Send),
) -> Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)> {
    assert_eq!(
        names.len(),
        plan.specs.len(),
        "one name per spec is required"
    );

    let total = Instant::now();
    let mut state = state::State::new(plan);
    let cohorts = cohorts::ordered_cohorts(&state);
    let students: usize = cohorts.iter().map(|cohort| cohort.members.len()).sum();
    log(&format!(
        "[greedy] {} student(s) over {} list(s), in {} cohort(s)",
        students,
        plan.specs.len(),
        cohorts.len(),
    ));

    let t = Instant::now();
    prefill::prefill(&mut state, &cohorts);
    // The cohorts are rarest first and their members ascending: the same
    // global order that drove prefill. A student prefill placed in
    // every list of their profile is done; one whose profile also holds
    // non-claiming lists still enters, and only the missing groups are
    // chosen. Placing one student never places another, so who is left can
    // be read once, here, instead of at every turn of the pass.
    let remaining: Vec<StudentId> = cohorts
        .iter()
        .flat_map(|cohort| cohort.members.iter().copied())
        .filter(|&student| !state.fully_placed(student))
        .collect();
    log(&format!(
        "[greedy] Prefill: {} student(s) seated, {} left to the pass ({:.2?})",
        students - remaining.len(),
        remaining.len(),
        t.elapsed(),
    ));

    let t = Instant::now();
    let step = remaining.len().div_ceil(10).max(1);
    for (done, &student) in remaining.iter().enumerate() {
        pass::place_student(&mut state, student);
        // The last one is the summary line below, not a progress line.
        if (done + 1) % step == 0 && done + 1 != remaining.len() {
            log(&format!(
                "[greedy] Pass: {}/{} student(s) placed ({:.2?})",
                done + 1,
                remaining.len(),
                t.elapsed(),
            ));
        }
    }
    log(&format!(
        "[greedy] Pass: {} student(s) placed ({:.2?})",
        remaining.len(),
        t.elapsed(),
    ));

    // What the two phases actually scored, so two runs over the same plan
    // can be compared line to line in the same log.
    log(&format!(
        "[greedy] Objective value: {:.6}",
        state.objective_value(),
    ));

    let lists = state.into_group_lists(names);
    log(&format!("[greedy] Done ({:.2?})", total.elapsed()));
    lists
}
