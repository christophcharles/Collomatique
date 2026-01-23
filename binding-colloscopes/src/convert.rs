use crate::{objects::InterrogationData, tools, views::Env};
use collomatique_ilp::ConfigData;
use collomatique_state_colloscopes::colloscopes::Colloscope;

use super::vars::Var;

pub fn build_config(env: &Env, colloscope: &Colloscope) -> ConfigData<Var> {
    let mut config_data = ConfigData::new();

    for (group_list_id, group_list) in &colloscope.group_lists {
        let data_group_list = if env.ignore_prefill_for_group_lists.contains(group_list_id) {
            None
        } else {
            Some(
                env.params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                    .expect("Group list ID should be valid"),
            )
        };
        for (student_id, group) in &group_list.groups_for_students {
            if let Some(gl) = data_group_list {
                if gl.prefilled_groups.contains_student(*student_id) {
                    continue;
                }
            }
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
    for (period_id, period_desc) in &env.params.periods.ordered_period_list {
        let period = colloscope
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

pub fn build_complete_config(env: &Env, colloscope: &Colloscope) -> ConfigData<Var> {
    let mut config_data = build_config(env, colloscope);

    for (group_list_id, group_list) in &env.params.group_lists.group_list_map {
        let data_group_list = if env.ignore_prefill_for_group_lists.contains(group_list_id) {
            None
        } else {
            Some(
                env.params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                    .expect("Group list ID should be valid"),
            )
        };
        for (student_id, _student) in &env.params.students.student_map {
            if group_list.params.excluded_students.contains(student_id) {
                continue;
            }
            if let Some(gl) = data_group_list {
                if gl.prefilled_groups.contains_student(*student_id) {
                    continue;
                }
            }
            let var = Var::StudentGroup {
                student: *student_id,
                group_list: *group_list_id,
            };
            if config_data.get(var.clone()).is_some() {
                continue;
            }
            config_data = config_data.set(var, -1.);
        }
    }

    let mut first_week_in_period = 0usize;
    for (period_id, period_desc) in &env.params.periods.ordered_period_list {
        let period = colloscope
            .period_map
            .get(period_id)
            .expect("Period ID should be valid");

        let subject_associations = env
            .params
            .group_lists
            .subjects_associations
            .get(period_id)
            .expect("Period Id should be valid");
        for (slot_id, slot) in &period.slot_map {
            let (subject_id, _pos) = env
                .params
                .slots
                .find_slot_subject_and_position(*slot_id)
                .expect("Slot ID should be valid");
            let Some(group_list_id) = subject_associations.get(&subject_id) else {
                // No interrogations for this period
                continue;
            };
            let group_list = env
                .params
                .group_lists
                .group_list_map
                .get(group_list_id)
                .expect("Group list ID should be valid");

            for (week_num, interrogation_opt) in slot.interrogations.iter().enumerate() {
                let Some(interrogation) = interrogation_opt else {
                    continue;
                };

                let week = first_week_in_period + week_num;

                for group_num in 0..group_list.params.max_group_count {
                    if interrogation.assigned_groups.contains(&group_num) {
                        continue;
                    }
                    let interrogation = InterrogationData {
                        slot: *slot_id,
                        week,
                    };
                    let group = group_num as i32;
                    config_data = config_data.set(
                        Var::GroupInInterrogation {
                            interrogation,
                            group,
                        },
                        0.0,
                    );
                }
            }
        }

        first_week_in_period += period_desc.len();
    }

    config_data
}

pub fn build_empty_colloscope_with_prefilled_groups(env: &Env) -> Colloscope {
    let mut colloscope = Colloscope::new_empty_from_params(&env.params);

    for (group_list_id, group_list) in &env.params.group_lists.group_list_map {
        if env.ignore_prefill_for_group_lists.contains(group_list_id) {
            continue;
        }
        let collo_group_list = colloscope
            .group_lists
            .get_mut(group_list_id)
            .expect("Group list ID should be valid");
        for (num, group) in group_list.prefilled_groups.groups.iter().enumerate() {
            for student_id in &group.students {
                collo_group_list
                    .groups_for_students
                    .insert(*student_id, num as u32);
            }
        }
    }

    colloscope
}

pub fn build_colloscope(env: &Env, config_data: &ConfigData<Var>) -> Option<Colloscope> {
    let mut colloscope = build_empty_colloscope_with_prefilled_groups(env);

    for (var, value) in config_data.get_values() {
        match var {
            Var::StudentGroup {
                student,
                group_list,
            } => {
                if value >= -0.1 {
                    let collo_group_list = colloscope.group_lists.get_mut(&group_list)?;
                    collo_group_list
                        .groups_for_students
                        .insert(student, value as u32);
                }
            }
            Var::GroupInInterrogation {
                interrogation,
                group,
            } => {
                if value < 0.5 {
                    continue;
                }
                let (period_id, num_in_period) =
                    tools::week_to_period_id(&env.params, interrogation.week)?;
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
