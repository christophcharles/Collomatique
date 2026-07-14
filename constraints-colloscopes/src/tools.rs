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
    let period_associations = params.group_lists.subjects_associations.get(&period)?;
    period_associations.get(&subject_id).copied()
}

pub fn week_to_period_id(params: &Parameters, week: usize) -> Option<(PeriodId, usize)> {
    let mut current_week = 0usize;
    for (period_id, period_desc) in params.periods.ordered_period_list.iter() {
        let next_period_week = current_week + period_desc.len();
        if week >= current_week && week < next_period_week {
            return Some((period_id, week - current_week));
        }
        current_week = next_period_week;
    }
    None
}

pub(crate) fn enumerate_weeks_for_slot_id(params: &Parameters, slot: SlotId) -> Vec<usize> {
    let Some((subject_id, pos)) = params.slots.find_slot_subject_and_position(slot) else {
        return vec![];
    };
    let slot_desc = &params.slots.subject_map[&subject_id].ordered_slots[pos].1;
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

    let mut current_first_week = 0usize;
    for (_period_id, period_desc) in params.periods.ordered_period_list.iter() {
        for (num, week_desc) in period_desc.iter().enumerate() {
            if !week_desc.interrogations {
                output.push(false);
                continue;
            }

            let week_num = current_first_week + num;
            let week_status = week_pattern
                .get(week_num)
                .expect("Week number should be valid");
            output.push(*week_status);
        }
        current_first_week += period_desc.len();
    }

    output
}
