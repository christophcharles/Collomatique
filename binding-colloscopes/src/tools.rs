use collomatique_state_colloscopes::{
    PeriodId, SlotId, WeekPatternId, colloscope_params::Parameters,
};

pub fn week_to_period_id(params: &Parameters, week: usize) -> Option<(PeriodId, usize)> {
    let mut current_week = 0usize;
    for (period_id, period_desc) in &params.periods.ordered_period_list {
        let next_period_week = current_week + period_desc.len();
        if week >= current_week && week < next_period_week {
            return Some((*period_id, week - current_week));
        }
        current_week = next_period_week;
    }
    None
}

pub fn enumerate_weeks_for_slot(params: &Parameters, slot: SlotId) -> Vec<usize> {
    let Some((subject_id, pos)) = params.slots.find_slot_subject_and_position(slot) else {
        return vec![];
    };
    let slot_desc = &params.slots.subject_map[&subject_id].ordered_slots[pos].1;
    let subject_desc = params
        .subjects
        .find_subject(subject_id)
        .expect("Subject ID should be valid");

    let week_pattern = extract_week_pattern(params, slot_desc.week_pattern);
    let mut output = vec![];
    for (week, status) in week_pattern.into_iter().enumerate() {
        if !status {
            continue;
        }
        let (period, _) =
            week_to_period_id(params, week).expect("Week should correspond to some period");
        if subject_desc.excluded_periods.contains(&period) {
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
    for (_period_id, period_desc) in &params.periods.ordered_period_list {
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
