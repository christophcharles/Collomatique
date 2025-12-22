use collomatique_state_colloscopes::{colloscope_params::Parameters, PeriodId, WeekPatternId};

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

pub fn compute_time_resolution(params: &Parameters) -> u32 {
    let mut resolution = 60u32; // Enforce at least hour resolution

    for (subject_id, subject_slots) in &params.slots.subject_map {
        let subject_desc = params
            .subjects
            .find_subject(*subject_id)
            .expect("Subject ID should be valid");
        let interrogation_params = subject_desc
            .parameters
            .interrogation_parameters
            .as_ref()
            .expect("There should be parameters when there are slots");
        resolution = gcd(resolution, interrogation_params.duration.get().get());
        for (_slot_id, slot_desc) in &subject_slots.ordered_slots {
            resolution = gcd(
                resolution,
                slot_desc.start_time.start_time.minutes_from_midnight(),
            );
        }
    }

    for (_incompat_id, incompat) in &params.incompats.incompat_map {
        for slot in &incompat.slots {
            resolution = gcd(resolution, slot.start().start_time.minutes_from_midnight());
            resolution = gcd(resolution, slot.duration().get().get());
        }
    }

    resolution
}

fn gcd(mut n1: u32, mut n2: u32) -> u32 {
    while n2 != 0 {
        let r = n1 % n2;
        n1 = n2;
        n2 = r;
    }

    n1
}
