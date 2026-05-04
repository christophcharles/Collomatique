use crate::ids::GlobalWeek;
use crate::native_extras::{V, extra_var, weeks_for_slot};
use crate::types::ExtraVarName;
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::{PeriodId, SlotId, StudentId, SubjectId};
use std::collections::BTreeSet;

pub(super) fn slot_week_pairs_for_subject(
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

pub(super) fn count_interrogations_expr(
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

pub(super) fn enrolled_students_for_subject(
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

pub(super) fn all_active_global_weeks(env: &VarEnv) -> Vec<GlobalWeek> {
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

pub(super) fn last_global_week(env: &VarEnv) -> GlobalWeek {
    let total: usize = env
        .periods
        .ordered_period_list
        .iter()
        .map(|(_, desc)| desc.len())
        .sum();
    GlobalWeek(total.saturating_sub(1))
}
