use crate::{objects::InterrogationData, tools};
use collomatique_ilp::ConfigData;
use collomatique_state_colloscopes::{
    colloscope_params::Parameters, colloscopes::Colloscope, InnerData,
};

use super::vars::Var;

pub fn build_config(inner_data: &InnerData) -> ConfigData<Var> {
    let mut config_data = ConfigData::new();

    for (group_list_id, group_list) in &inner_data.colloscope.group_lists {
        for (student_id, group) in &group_list.groups_for_students {
            config_data = config_data.set(
                Var::StudentGroup {
                    student: *student_id,
                    group_list: *group_list_id,
                },
                *group as f64,
            );
        }
    }

    let mut first_week_in_period = 0usize;
    for (period_id, period_desc) in &inner_data.params.periods.ordered_period_list {
        let period = inner_data
            .colloscope
            .period_map
            .get(period_id)
            .expect("Period ID should be valid");

        for (slot_id, slot) in &period.slot_map {
            for (week_num, interrogation_opt) in slot.interrogations.iter().enumerate() {
                let Some(interrogation) = interrogation_opt else {
                    continue;
                };

                let week = first_week_in_period + week_num;

                for group_num in &interrogation.assigned_groups {
                    let interrogation = InterrogationData {
                        slot: *slot_id,
                        week,
                    };
                    let group = *group_num as i32;
                    config_data = config_data.set(
                        Var::GroupInInterrogation {
                            interrogation,
                            group,
                        },
                        1.0,
                    );
                }
            }
        }

        first_week_in_period += period_desc.len();
    }

    config_data
}

pub fn build_colloscope(params: &Parameters, config_data: &ConfigData<Var>) -> Option<Colloscope> {
    let mut colloscope = Colloscope::new_empty_from_params(params);

    for (var, value) in config_data.get_values() {
        match var {
            Var::StudentGroup {
                student,
                group_list,
            } => {
                let collo_group_list = colloscope.group_lists.get_mut(&group_list)?;
                collo_group_list
                    .groups_for_students
                    .insert(student, value as u32);
            }
            Var::GroupInInterrogation {
                interrogation,
                group,
            } => {
                if value < 0.5 {
                    continue;
                }
                let (period_id, num_in_period) =
                    tools::week_to_period_id(params, interrogation.week)?;
                let collo_period = colloscope.period_map.get_mut(&period_id)?;
                let collo_slot = collo_period.slot_map.get_mut(&interrogation.slot)?;
                let collo_interrogation_opt = collo_slot.interrogations.get_mut(num_in_period)?;

                let Some(collo_interrogation) = collo_interrogation_opt else {
                    return None;
                };
                collo_interrogation.assigned_groups.insert(group as u32);
            }
        }
    }

    Some(colloscope)
}
