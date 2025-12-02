use collomatique_state_colloscopes::{Data, PeriodId};

pub fn week_to_period_id(env: &Data, week: usize) -> PeriodId {
    let mut current_week = 0usize;
    for (period_id, period_desc) in &env.get_inner_data().params.periods.ordered_period_list {
        let next_period_week = current_week + period_desc.len();
        if week >= current_week && week < next_period_week {
            return *period_id;
        }
        current_week = next_period_week;
    }
    panic!("Invalid week")
}
