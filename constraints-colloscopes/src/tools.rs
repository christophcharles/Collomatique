use collomatique_state_colloscopes::slots::Slot;
use collomatique_state_colloscopes::{
    GroupListId, PeriodId, SlotId, WeekPatternId, colloscope_params::Parameters,
};
use std::collections::BTreeSet;

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
        let period_len = params
            .periods
            .week_count_of(period_id)
            .expect("period id from period_ids is valid");
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

    enumerate_weeks_for_slot(params, slot_desc, &subject_desc.excluded_periods)
}

pub fn enumerate_weeks_for_slot(
    params: &Parameters,
    slot: &Slot,
    excluded_periods: &BTreeSet<PeriodId>,
) -> Vec<usize> {
    let week_pattern = extract_week_pattern(params, slot.week_pattern);
    let mut output = vec![];
    for (week, status) in week_pattern.into_iter().enumerate() {
        if !status {
            continue;
        }
        let (period, _) =
            week_to_period_id(params, week).expect("Week should correspond to some period");
        if excluded_periods.contains(&period) {
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
    let mut output = vec![];

    let week_pattern = match week_pattern_id {
        Some(id) => params
            .week_patterns
            .week_pattern_map
            .get(&id)
            .expect("WeekPatternId should be valid")
            .weeks
            .clone(),
        None => vec![true; params.periods.count_weeks()],
    };

    for (week_num, (_period_id, week_desc)) in params.periods.walk().enumerate() {
        if !week_desc.interrogations {
            output.push(false);
            continue;
        }

        let week_status = week_pattern
            .get(week_num)
            .expect("Week number should be valid");
        output.push(*week_status);
    }

    output
}
