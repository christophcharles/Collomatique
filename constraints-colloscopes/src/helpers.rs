use crate::extras::{MyBundle, V, extra_var, weeks_for_slot};
use crate::ids::GlobalWeek;
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::{PeriodId, SlotId, StudentId, SubjectId};
use std::collections::BTreeSet;

pub(crate) fn slot_week_pairs_for_subject(
    env: &VarEnv,
    subject_id: SubjectId,
    excluded_periods: &BTreeSet<PeriodId>,
) -> Vec<(SlotId, GlobalWeek)> {
    let Some(subject_slots) = env.slots.subject_map.get(&subject_id) else {
        return vec![];
    };
    subject_slots
        .ordered_slots
        .iter()
        .flat_map(|(slot_id, slot_data)| {
            weeks_for_slot(env, slot_data, excluded_periods)
                .into_iter()
                .map(move |week| (*slot_id, week))
        })
        .collect()
}

pub(crate) fn count_interrogations_expr(
    slot_week_pairs: &[(SlotId, GlobalWeek)],
    student: StudentId,
    first_week: GlobalWeek,
    last_week: GlobalWeek,
) -> IntLinExpr<V> {
    slot_week_pairs
        .iter()
        .filter(|(_, week)| *week >= first_week && *week <= last_week)
        .map(|&(slot, week)| {
            IntLinExpr::var(extra_var(ExtraVarName::StudentAtInterrogation {
                student,
                slot,
                week,
            }))
        })
        .sum()
}

pub(crate) fn enrolled_students_for_subject(
    env: &VarEnv,
    subject_id: SubjectId,
) -> BTreeSet<StudentId> {
    env.assignments
        .period_map
        .values()
        .filter_map(|pa| pa.subject_map.get(&subject_id))
        .flat_map(|students| students.iter().copied())
        .collect()
}

pub(crate) fn all_active_global_weeks(env: &VarEnv) -> Vec<GlobalWeek> {
    let mut result = Vec::new();
    let mut global_week = 0usize;
    for (_period_id, period_desc) in &env.periods.ordered_period_list {
        for week_desc in period_desc {
            if week_desc.interrogations {
                result.push(GlobalWeek(global_week));
            }
            global_week += 1;
        }
    }
    result
}

pub(crate) fn last_global_week(env: &VarEnv) -> GlobalWeek {
    let total: usize = env
        .periods
        .ordered_period_list
        .iter()
        .map(|(_, desc)| desc.len())
        .sum();
    GlobalWeek(total.saturating_sub(1))
}

/// Objectify a soft bundle as a plain weighted sum and merge it in.
///
/// Emits `Σ wᵢ·λᵢ` where each `λᵢ` bounds one constraint's violation and `wᵢ`
/// is `weight_fn` applied to that constraint's [`ConstraintDesc`]. There is no
/// `1/n` normalization and no global `L∞` bound: the penalty's footprint stays
/// confined to each constraint's own variables, which is what lets the
/// incremental strategy pick the terms up epoch by epoch (a global `L∞` bound
/// would span every constraint and only enter at the final epoch). Every soft
/// family uses this — the balancing terms weight each `λᵢ` by `BASE/n`, the
/// limits/pairings terms by a flat `BASE` per violation. An empty soft bundle
/// contributes nothing (`Err → bundle unchanged`).
pub(crate) fn merge_objectified_weighted(
    bundle: MyBundle,
    soft_bundle: MyBundle,
    penalty_var: ExtraVarName,
    weight_fn: impl Fn(&ConstraintDesc) -> f64,
) -> MyBundle {
    match soft_bundle.objectify_weighted_sum(penalty_var, weight_fn) {
        Ok(objectified) => bundle
            .merge(objectified)
            .expect("no duplicate extras from objectification"),
        Err(_) => bundle,
    }
}
