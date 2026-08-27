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

#[cfg(test)]
mod tests;

use crate::frozen::FrozenPlacements;
use crate::specs::GenerationPlan;
use collomatique_state_colloscopes::group_lists::GroupList;
use collomatique_state_colloscopes::{PeriodId, StudentId, SubjectId};
use std::collections::BTreeSet;
use std::time::Instant;

/// What one greedy run produced.
#[derive(Debug)]
pub struct GreedyOutcome {
    /// One prefilled `GroupList` per spec, in plan order, paired with the
    /// (period, subject) pairs it must be associated to — exactly the payload
    /// of `GroupListsUpdateOp::AddGeneratedGroupLists`.
    pub lists: Vec<(GroupList, BTreeSet<(PeriodId, SubjectId)>)>,
    /// The seats phase one froze, in the shape
    /// [`build_model`](crate::build_model) pins them from. Empty when prefill
    /// claimed nothing.
    pub frozen: FrozenPlacements,
}

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
pub fn greedy_group_lists(plan: &GenerationPlan, names: &[String]) -> GreedyOutcome {
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
) -> GreedyOutcome {
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
    // global order that drove prefill (§7.1). A student prefill placed in
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

    // The §9 diagnostic: what the two phases actually scored. It is the number
    // an optional ILP polish over the same plan has to beat — or tie, since
    // the greedy solution is its warm start — so the two runs can be compared
    // line to line in the same log.
    log(&format!(
        "[greedy] Objective value: {:.6}",
        state.objective_value(),
    ));

    // Read before `into_group_lists` consumes the state.
    let frozen = state.frozen_placements();
    let lists = state.into_group_lists(names);
    log(&format!("[greedy] Done ({:.2?})", total.elapsed()));
    GreedyOutcome { lists, frozen }
}

/// The collision probability a finished placement reaches: `Σ_s Σ_t P_s(t)²`
/// (§2.3), kept-list mass included — the very quantity
/// [`greedy_group_lists`] maximizes, evaluated on lists it did not
/// necessarily produce.
///
/// `lists` is one prefilled list per spec, in plan order: a
/// [`GreedyOutcome::lists`] or a
/// [`build_group_lists`](crate::build_group_lists) output for the same plan.
/// That is what makes it the ground truth of the model's objective — the two
/// numbers must agree on the same placement — and the reason it is public:
/// the equality is asserted outside the crate's unit tests too.
///
/// Panics on internal inconsistency, like
/// [`group_lists_to_warm_start`](crate::group_lists_to_warm_start): a list
/// count differing from the plan's, a student of a spec sitting in no group of
/// its list, or a group index beyond the ones the plan gives the list.
pub fn placement_objective(
    plan: &GenerationPlan,
    lists: &[(GroupList, BTreeSet<(PeriodId, SubjectId)>)],
) -> f64 {
    assert_eq!(
        lists.len(),
        plan.specs.len(),
        "one list per spec is required"
    );

    let mut state = state::State::new(plan);
    for (list, ((spec, _covered), (group_list, _pairs))) in
        plan.specs.iter().zip(lists.iter()).enumerate()
    {
        for &student in spec.students() {
            let group = group_list
                .filling()
                .find_student_group(student)
                .expect("every student of a spec sits in a group of its list");
            assert!(
                group < state.targets(list).len(),
                "the list has more groups than the plan gives it",
            );
            state.place(student, list, group);
        }
    }
    state.objective_value()
}
