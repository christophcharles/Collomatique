use collomatique_binding_colloscopes::tools;
use collomatique_binding_colloscopes::vars::Var;
use collomatique_ilp::ConfigData;
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::colloscopes::Colloscope;

use collomatique_state_colloscopes::ids::Id;

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
                    student: student_id.inner() as i32,
                    group_list: group_list_id.inner() as i32,
                },
                *group as f64,
            );
        }
    }

    let mut first_week_in_period = 0usize;
    for (period_id, period_desc) in &env.periods.ordered_period_list {
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
                    let group = *group_num as i32;
                    config_data = config_data.set(
                        Var::GroupInInterrogationInternal {
                            slot: slot_id.inner() as i32,
                            week: week as i32,
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

    for (group_list_id, group_list) in &env.group_lists.group_list_map {
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
                student: student_id.inner() as i32,
                group_list: group_list_id.inner() as i32,
            };
            if config_data.get(var.clone()).is_some() {
                continue;
            }
            config_data = config_data.set(var, -1.);
        }
    }

    let mut first_week_in_period = 0usize;
    for (period_id, period_desc) in &env.periods.ordered_period_list {
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

                for group_num in 0..group_list.params.group_names.len() as u32 {
                    if interrogation.assigned_groups.contains(&group_num) {
                        continue;
                    }
                    let group = group_num as i32;
                    config_data = config_data.set(
                        Var::GroupInInterrogationInternal {
                            slot: slot_id.inner() as i32,
                            week: week as i32,
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

pub fn build_colloscope(env: &Parameters, config_data: &ConfigData<Var>) -> Option<Colloscope> {
    let mut colloscope = Colloscope::new_empty_from_params(env);

    for (var, value) in config_data.get_values() {
        match var {
            Var::StudentGroup {
                student,
                group_list,
            } => {
                if value >= -0.1 {
                    let group_list_id = unsafe {
                        collomatique_state_colloscopes::ids::GroupListId::new(group_list as u64)
                    };
                    let student_id = unsafe {
                        collomatique_state_colloscopes::ids::StudentId::new(student as u64)
                    };
                    let collo_group_list = colloscope.group_lists.get_mut(&group_list_id)?;
                    collo_group_list
                        .groups_for_students
                        .insert(student_id, value as u32);
                }
            }
            Var::GroupInInterrogationInternal { slot, week, group } => {
                if value < 0.5 {
                    continue;
                }
                let slot_id =
                    unsafe { collomatique_state_colloscopes::ids::SlotId::new(slot as u64) };
                let (period_id, num_in_period) = tools::week_to_period_id(env, week as usize)?;
                let collo_period = colloscope.period_map.get_mut(&period_id)?;
                let collo_slot = collo_period.slot_map.get_mut(&slot_id)?;
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
