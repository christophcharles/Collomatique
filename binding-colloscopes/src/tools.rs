use collomatique_state_colloscopes::{
    colloscope_params::Parameters, Data, PeriodId, WeekPatternId,
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

pub fn extract_week_pattern(env: &Data, week_pattern_id: Option<WeekPatternId>) -> Vec<bool> {
    let mut output = vec![];

    let week_pattern = match week_pattern_id {
        Some(id) => env
            .get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .get(&id)
            .expect("WeekPatternId should be valid")
            .weeks
            .clone(),
        None => vec![true; env.get_inner_data().params.periods.count_weeks()],
    };

    let mut current_first_week = 0usize;
    for (_period_id, period_desc) in &env.get_inner_data().params.periods.ordered_period_list {
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
