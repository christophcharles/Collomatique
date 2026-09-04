//! Permanent soft objective discouraging the same teacher twice in a row.
//!
//! Always on — no configuration surface. Skipped per subject when the hard
//! `avoid_twice_in_a_row` balancing flag is set (the hard constraint subsumes
//! it, and both recursive forms would otherwise build the same
//! `IsLastTeacherSeen` extras) and when the subject has fewer than two distinct
//! teachers (nothing to rotate).
//!
//! For `ExactlyPeriodic { p }`, the hard periodicity constraint forces
//! interrogations exactly `p` active weeks apart, so rolling `2p`-windows
//! (step 1) plus edge windows of sizes `p+1..2p-1` anchored at both ends make
//! every consecutive same-teacher pair fall in exactly `p` windows; with
//! per-window weight `BASE / p`, one violation costs exactly `BASE`. For
//! `OnceForEveryBlockOfWeeks { b }`, `2b`-windows with step `b` cover each
//! consecutive-block pair exactly once (weight `BASE`). For the non-periodic
//! variants, the `IsLastTeacherSeen` chain is reused with its avoid rows
//! objectified at `BASE` per week — gated behind
//! [`ENABLE_RECURSIVE_SOFT_AVOID_TWICE`].

use crate::extras::{MyBundle, is_at_most_once_per_week, subject_interrogation_params};
use crate::helpers::slot_week_pairs_for_subject;
use crate::ids::GlobalWeek;
use crate::vars::VarEnv;
use collomatique_state_colloscopes::subjects::SubjectPeriodicity;
use std::collections::BTreeSet;

use super::avoid_twice_in_a_row::{
    RecursiveMode, WindowMode, build_recursive_constraints, build_window_constraints,
};
use super::helpers::{
    effective_balancing_option, rolling_windows, subject_active_weeks, teachers_for_subject,
};

/// Kill switch for the recursive (`AmountInYear` /
/// `AmountForEveryArbitraryBlock`) part of the objective: the
/// `IsLastTeacherSeen` chain costs one binary and one reified row per
/// (student, teacher, active week), which may prove too expensive on large
/// documents. Hard-coded for now; flip to `false` to drop that part, or promote
/// to a real configuration toggle if the need arises.
const ENABLE_RECURSIVE_SOFT_AVOID_TWICE: bool = true;

/// Rolling `2p`-windows (step 1) plus the `p+1..2p-1` edge windows anchored at
/// both ends, deduplicated (for short years the two ends overlap).
fn exactly_periodic_soft_windows(
    active_weeks: &[GlobalWeek],
    p: usize,
) -> Vec<(GlobalWeek, GlobalWeek)> {
    let n = active_weeks.len();
    let mut windows: BTreeSet<(GlobalWeek, GlobalWeek)> = rolling_windows(active_weeks, 2 * p, 1)
        .into_iter()
        .collect();
    for m in (p + 1)..(2 * p) {
        if m <= n {
            windows.insert((active_weeks[0], active_weeks[m - 1]));
            windows.insert((active_weeks[n - m], active_weeks[n - 1]));
        }
    }
    windows.into_iter().collect()
}

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut output = MyBundle::new();

    for (subject_id, subject) in env.subjects.ordered_subject_list.iter() {
        let subject_id = &subject_id;
        let Some(params) = subject_interrogation_params(env, *subject_id) else {
            continue;
        };
        // Soft path only: the goal must be on *and* soft. `None` (off) builds
        // nothing at all — no objective term either — and `Some { soft: false }`
        // is the hard builder's business.
        let soft = matches!(
            effective_balancing_option(env, *subject_id, |opts| &opts.avoid_twice_in_a_row),
            Some(param) if param.soft
        );
        if !soft {
            continue;
        }
        let teachers = teachers_for_subject(env, *subject_id);
        if teachers.len() < 2 {
            continue;
        }

        let slot_week_pairs = slot_week_pairs_for_subject(env, *subject_id, subject);
        let active_weeks = subject_active_weeks(&slot_week_pairs);

        let mut bundle = MyBundle::new();

        match &params.periodicity {
            SubjectPeriodicity::ExactlyPeriodic {
                periodicity_in_weeks,
            } => {
                let p = periodicity_in_weeks.get() as usize;
                let windows = exactly_periodic_soft_windows(&active_weeks, p);
                let weight = crate::weights::BASE / p as f64;
                build_window_constraints(
                    env,
                    *subject_id,
                    &slot_week_pairs,
                    &windows,
                    WindowMode::Soft { weight },
                    &mut bundle,
                );
            }
            SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                weeks_per_block, ..
            } => {
                let b = weeks_per_block.get() as usize;
                let windows = rolling_windows(&active_weeks, 2 * b, b);
                build_window_constraints(
                    env,
                    *subject_id,
                    &slot_week_pairs,
                    &windows,
                    WindowMode::Soft {
                        weight: crate::weights::BASE,
                    },
                    &mut bundle,
                );
            }
            SubjectPeriodicity::AmountInYear { .. }
            | SubjectPeriodicity::AmountForEveryArbitraryBlock { .. } => {
                if ENABLE_RECURSIVE_SOFT_AVOID_TWICE && is_at_most_once_per_week(env, *subject_id) {
                    build_recursive_constraints(
                        env,
                        *subject_id,
                        &slot_week_pairs,
                        RecursiveMode::Soft,
                        &mut bundle,
                    );
                }
            }
        }

        output = output
            .merge(bundle)
            .expect("no duplicate extras from soft avoid_twice (distinct subjects)");
    }

    output
}
