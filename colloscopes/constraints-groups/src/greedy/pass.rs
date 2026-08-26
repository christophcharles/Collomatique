//! The greedy pass: one joint placement per remaining student.
//!
//! Joint placement is not optional under this objective (§7.3): a first
//! meeting contributes a negligible squared mass, so a list-by-list decision
//! would settle almost everything on ties — only the joint view makes
//! repetition visible.
//!
//! Deliberately absent (§7.6): lookahead — the student order is the whole
//! lookahead budget — cross-student repair, and any revisiting of students
//! already done. Unhappy with the result: send it to the ILP.

use super::state::State;
use collomatique_state_colloscopes::StudentId;

/// Chooses the student's group in every list of their profile not already
/// fixed by prefill, deterministically, given the current state.
///
/// This is §7.2's isolated subroutine: the boundary is settled so the
/// strategy can be swapped if it ever proves slow; the sweep below is the
/// first implementation.
pub(super) fn place_student(state: &mut State, student: StudentId) {
    // First pass, spec order: the best free-seat group in each list.
    for list in state.unplaced_lists(student) {
        let group = best_group(state, student, list, None);
        state.place(student, list, group);
    }

    // Revision sweeps to a fixpoint: take the student out of a list and
    // re-choose given every other current placement. A move requires a
    // strict increase of a *pure function* of the configuration (§8), so no
    // cycle is possible even in `f64`, and the score takes finitely many
    // values, so this terminates.
    let movable = state.movable_lists(student);
    let mut sweeps = 0usize;
    loop {
        let mut changed = false;
        for &list in &movable {
            let old = state.remove(student, list);
            let group = best_group(state, student, list, Some(old));
            state.place(student, list, group);
            changed |= group != old;
        }
        if !changed {
            break;
        }
        sweeps += 1;
        // A bug detector, not a limiter (§7.3).
        debug_assert!(sweeps < 1_000, "place_student sweep did not settle");
    }
}

/// The best group for a student out of `list`, scanning ascending and
/// requiring strict improvement (§7.5).
///
/// Without an incumbent — a first placement — the lowest-indexed candidate
/// wins exact ties; targets being sorted descending, ties fill the big groups
/// first. With an incumbent, that group is the baseline and a candidate must
/// strictly beat it to cause a move, so ties keep the student where they are.
/// Both readings are the same rule, and together they are what makes the
/// fixpoint argument airtight.
fn best_group(state: &State, student: StudentId, list: usize, incumbent: Option<usize>) -> usize {
    let mut best: Option<(usize, f64)> =
        incumbent.map(|group| (group, state.placement_delta(student, list, group)));
    for group in 0..state.targets(list).len() {
        if !state.has_free_seat(list, group) {
            continue;
        }
        let delta = state.placement_delta(student, list, group);
        match best {
            Some((_, best_delta)) if delta <= best_delta => {}
            _ => best = Some((group, delta)),
        }
    }
    let (group, _delta) =
        best.expect("the targets sum to the student count: a seat is always free");
    group
}
