use crate::extras::{V, extra_var, subject_interrogation_params};
use crate::ids::GlobalWeek;
use crate::types::ExtraVarName;
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::balancing::BalancingOptions;
use collomatique_state_colloscopes::ids::{SlotId, StudentId, SubjectId, TeacherId};
use collomatique_state_colloscopes::soft_param::SoftParam;
use collomatique_state_colloscopes::subjects::SubjectPeriodicity;
use std::collections::BTreeSet;

pub(super) fn effective_balancing_flag(
    env: &VarEnv,
    subject_id: SubjectId,
    extract: impl Fn(&BalancingOptions) -> bool,
) -> bool {
    extract(env.balancing.options_for(subject_id))
}

/// Same as [`effective_balancing_flag`] for the three-state goals: `None` means
/// the goal is not pursued at all, `Some { soft }` says how it is pursued.
pub(super) fn effective_balancing_option<'a>(
    env: &'a VarEnv,
    subject_id: SubjectId,
    extract: impl Fn(&BalancingOptions) -> &Option<SoftParam<()>>,
) -> Option<&'a SoftParam<()>> {
    extract(env.balancing.options_for(subject_id)).as_ref()
}

pub(super) fn teachers_for_subject(env: &VarEnv, subject_id: SubjectId) -> BTreeSet<TeacherId> {
    let Some(subject_slots) = env.slots.slots_for_subject(subject_id) else {
        return BTreeSet::new();
    };
    subject_slots
        .map(|(_, slot_data)| slot_data.teacher_id)
        .collect()
}

pub(super) fn slot_week_pairs_for_teacher(
    all_pairs: &[(SlotId, GlobalWeek)],
    env: &VarEnv,
    subject_id: SubjectId,
    teacher_id: TeacherId,
) -> Vec<(SlotId, GlobalWeek)> {
    let Some(subject_slots) = env.slots.slots_for_subject(subject_id) else {
        return vec![];
    };
    let teacher_slots: BTreeSet<SlotId> = subject_slots
        .filter(|(_, slot_data)| slot_data.teacher_id == teacher_id)
        .map(|(slot_id, _)| *slot_id)
        .collect();
    all_pairs
        .iter()
        .filter(|(slot_id, _)| teacher_slots.contains(slot_id))
        .copied()
        .collect()
}

pub(super) fn count_student_teacher_expr(
    teacher_slot_week_pairs: &[(SlotId, GlobalWeek)],
    student: StudentId,
    first_week: GlobalWeek,
    last_week: GlobalWeek,
) -> IntLinExpr<V> {
    teacher_slot_week_pairs
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

pub(super) fn subject_active_weeks(slot_week_pairs: &[(SlotId, GlobalWeek)]) -> Vec<GlobalWeek> {
    let weeks: BTreeSet<GlobalWeek> = slot_week_pairs.iter().map(|(_, week)| *week).collect();
    weeks.into_iter().collect()
}

pub(super) fn rolling_windows(
    active_weeks: &[GlobalWeek],
    window_size: usize,
    step_size: usize,
) -> Vec<(GlobalWeek, GlobalWeek)> {
    let mut windows = Vec::new();
    let mut i = 0;
    while i + window_size <= active_weeks.len() {
        windows.push((active_weeks[i], active_weeks[i + window_size - 1]));
        i += step_size;
    }
    windows
}

pub(super) fn year_interrogation_count(env: &VarEnv, subject_id: SubjectId) -> Option<u32> {
    let params = subject_interrogation_params(env, subject_id)?;
    match &params.periodicity {
        SubjectPeriodicity::AmountInYear {
            interrogation_count_in_year,
            ..
        } => Some(*interrogation_count_in_year.end()),
        SubjectPeriodicity::AmountForEveryArbitraryBlock { blocks, .. } => Some(
            blocks
                .iter()
                .map(|b| *b.interrogation_count_in_block.end())
                .sum(),
        ),
        SubjectPeriodicity::ExactlyPeriodic {
            periodicity_in_weeks,
        } => {
            let subject = env.subjects.find_subject(subject_id)?;
            let slot_week_pairs = crate::helpers::slot_week_pairs_for_subject(
                env,
                subject_id,
                &subject.excluded_periods,
            );
            let active_weeks = subject_active_weeks(&slot_week_pairs);
            let n = active_weeks.len() as u32;
            let p = periodicity_in_weeks.get();
            Some((n + p - 1) / p)
        }
        SubjectPeriodicity::OnceForEveryBlockOfWeeks {
            weeks_per_block, ..
        } => {
            let subject = env.subjects.find_subject(subject_id)?;
            let slot_week_pairs = crate::helpers::slot_week_pairs_for_subject(
                env,
                subject_id,
                &subject.excluded_periods,
            );
            let active_weeks = subject_active_weeks(&slot_week_pairs);
            let n = active_weeks.len() as u32;
            let b = weeks_per_block.get();
            Some((n + b - 1) / b)
        }
    }
}

pub(super) fn slot_weeks_in_range(
    pairs: &[(SlotId, GlobalWeek)],
    first_week: GlobalWeek,
    last_week: GlobalWeek,
) -> usize {
    pairs
        .iter()
        .filter(|(_, week)| *week >= first_week && *week <= last_week)
        .count()
}
