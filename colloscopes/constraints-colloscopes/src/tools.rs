use collomatique_state_colloscopes::slots::Slot;
use collomatique_state_colloscopes::subjects::Subject;
use collomatique_state_colloscopes::{
    GroupListId, PeriodId, SlotId, WeekPatternId, colloscope_params::Parameters,
};

pub fn group_list_for_slot(
    params: &Parameters,
    period: PeriodId,
    slot: SlotId,
) -> Option<GroupListId> {
    let (subject_id, _) = params.slots.find_slot_subject_and_position(slot)?;
    params
        .group_lists
        .subjects_associations
        .get(&(period, subject_id))
        .copied()
}

pub fn week_to_period_id(params: &Parameters, week: usize) -> Option<(PeriodId, usize)> {
    let mut current_week = 0usize;
    for period_id in params.periods.period_ids() {
        let period_len = params.weeks.week_count_for_period(period_id).unwrap_or(0);
        let next_period_week = current_week + period_len;
        if week >= current_week && week < next_period_week {
            return Some((period_id, week - current_week));
        }
        current_week = next_period_week;
    }
    None
}

pub(crate) fn enumerate_weeks_for_slot_id(params: &Parameters, slot: SlotId) -> Vec<usize> {
    let Some((subject_id, slot_desc)) = params.slots.find_slot_with_subject(slot) else {
        return vec![];
    };
    let subject_desc = params
        .subjects
        .find_subject(subject_id)
        .expect("Subject ID should be valid");

    enumerate_weeks_for_slot(params, slot_desc, subject_desc)
}

/// The weeks a slot can carry an interrogation on, as global week indices.
///
/// A week survives only if it is active for the subject's own pattern *and* for
/// the slot's pattern, and its period is not excluded by the subject. This is
/// the global-index twin of [`Parameters::is_interrogation_possible`], and the
/// single place the two patterns are ANDed for the constraints layer: every ILP
/// variable domain flows through it, so a week the subject's pattern disables
/// carries no variable at all.
pub fn enumerate_weeks_for_slot(params: &Parameters, slot: &Slot, subject: &Subject) -> Vec<usize> {
    let slot_weeks = extract_week_pattern(params, slot.week_pattern);
    let subject_weeks = extract_week_pattern(params, subject.week_pattern);
    let mut output = vec![];
    for (week, status) in slot_weeks.into_iter().enumerate() {
        if !status || !subject_weeks[week] {
            continue;
        }
        let (period, _) =
            week_to_period_id(params, week).expect("Week should correspond to some period");
        if subject.excluded_periods.contains(&period) {
            continue;
        }
        output.push(week);
    }
    output
}

pub fn extract_week_pattern(
    params: &Parameters,
    week_pattern_id: Option<WeekPatternId>,
) -> Vec<bool> {
    params
        .walk_weeks()
        .map(|(_period_id, week_id, _week_desc)| params.is_week_active(week_id, week_pattern_id))
        .collect()
}
