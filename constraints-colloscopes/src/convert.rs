use crate::ids::{GlobalWeek, GroupNum};
use crate::tools;
use crate::vars::Var;
use collomatique_ilp::ConfigData;
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::colloscopes::Colloscope;

pub fn build_config(env: &Parameters, colloscope: &Colloscope) -> ConfigData<Var> {
    let mut config_data = ConfigData::new();

    for (group_list_id, group_list) in &colloscope.group_lists {
        let data_group_list = env
            .group_lists
            .group_list_map
            .get(group_list_id)
            .expect("Group list ID should be valid");
        if data_group_list.is_prefilled() {
            continue;
        }
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
    for (period_id, period_desc) in env.periods.ordered_period_list.entries() {
        let period_id = &period_id;
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
                    let group = GroupNum::new(
                        env,
                        tools::group_list_for_slot(env, *period_id, *slot_id)
                            .expect("slot should have a group list"),
                        *group_num as usize,
                    )
                    .expect("group number should be valid");
                    config_data = config_data.set(
                        Var::GroupInInterrogation {
                            slot: *slot_id,
                            week: GlobalWeek(week),
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

pub fn build_complete_config(env: &Parameters, colloscope: &Colloscope) -> ConfigData<Var> {
    let mut config_data = build_config(env, colloscope);

    for (group_list_id, group_list) in env.group_lists.group_list_map.entries() {
        let group_list_id = &group_list_id;
        let data_group_list = env
            .group_lists
            .group_list_map
            .get(group_list_id)
            .expect("Group list ID should be valid");
        if data_group_list.is_prefilled() {
            continue;
        }
        for student_id in env.students.student_map.keys() {
            if group_list.filling.excluded_students().contains(student_id) {
                continue;
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
    for (period_id, period_desc) in env.periods.ordered_period_list.entries() {
        let period_id = &period_id;
        let period = colloscope
            .period_map
            .get(period_id)
            .expect("Period ID should be valid");

        let subject_associations = env
            .group_lists
            .subjects_associations
            .get(period_id)
            .expect("Period Id should be valid");
        for (slot_id, slot) in &period.slot_map {
            let (subject_id, _pos) = env
                .slots
                .find_slot_subject_and_position(*slot_id)
                .expect("Slot ID should be valid");
            let Some(group_list_id) = subject_associations.get(&subject_id) else {
                continue;
            };
            let group_list = env
                .group_lists
                .group_list_map
                .get(group_list_id)
                .expect("Group list ID should be valid");

            for (week_num, interrogation_opt) in slot.interrogations.iter().enumerate() {
                let Some(interrogation) = interrogation_opt else {
                    continue;
                };

                let week = first_week_in_period + week_num;

                for group_num in 0..group_list.params.group_names.len() {
                    if interrogation.assigned_groups.contains(&(group_num as u32)) {
                        continue;
                    }
                    config_data = config_data.set(
                        Var::GroupInInterrogation {
                            slot: *slot_id,
                            week: GlobalWeek(week),
                            group: GroupNum::new(env, *group_list_id, group_num)
                                .expect("group number should be valid"),
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

pub fn build_colloscope(env: &Parameters, config_data: &ConfigData<Var>) -> Option<Colloscope> {
    let mut colloscope = Colloscope::new_empty_from_params(env);

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
            Var::GroupInInterrogation { slot, week, group } => {
                if value < 0.5 {
                    continue;
                }
                let (period_id, num_in_period) = tools::week_to_period_id(env, week.0)?;
                let collo_period = colloscope.period_map.get_mut(&period_id)?;
                let collo_slot = collo_period.slot_map.get_mut(&slot)?;
                let collo_interrogation_opt = collo_slot.interrogations.get_mut(num_in_period)?;

                let Some(collo_interrogation) = collo_interrogation_opt else {
                    return None;
                };
                collo_interrogation
                    .assigned_groups
                    .insert(group.index() as u32);
            }
        }
    }

    Some(colloscope)
}
