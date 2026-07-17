use crate::ids::{GlobalWeek, GroupNum};
use crate::tools;
use crate::vars::Var;
use collomatique_ilp::ConfigData;
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::colloscopes::Colloscope;
use collomatique_state_colloscopes::ids::{GroupListId, SlotId, StudentId, WeekId};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_config(env: &Parameters, colloscope: &Colloscope) -> ConfigData<Var> {
    let mut config_data = ConfigData::new();

    // The colloscope only ever holds non-prefilled group lists (validated), so
    // the historical prefilled skip is dead here.
    for (group_list_id, placements) in colloscope.group_lists_iter() {
        for (student_id, group) in placements {
            config_data = config_data.set(
                Var::StudentGroup {
                    student: *student_id,
                    group_list: group_list_id,
                },
                *group as f64,
            );
        }
    }

    for ((slot_id, week_id), assigned_groups) in colloscope.iter() {
        let (period_id, _pos) = env
            .periods
            .week_position(week_id)
            .expect("week id from a live colloscope row is valid");
        let week = env
            .periods
            .global_week_position(week_id)
            .expect("week id from a live colloscope row is valid");

        for group_num in assigned_groups {
            let group = GroupNum::new(
                env,
                tools::group_list_for_slot(env, period_id, slot_id)
                    .expect("slot should have a group list"),
                *group_num as usize,
            )
            .expect("group number should be valid");
            config_data = config_data.set(
                Var::GroupInInterrogation {
                    slot: slot_id,
                    week: GlobalWeek(week),
                    group,
                },
                1.0,
            );
        }
    }

    config_data
}

pub fn build_complete_config(env: &Parameters, colloscope: &Colloscope) -> ConfigData<Var> {
    let mut config_data = build_config(env, colloscope);

    for (group_list_id, group_list) in env.group_lists.group_list_map.iter() {
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
            if group_list.filling.excluded_students().contains(&student_id) {
                continue;
            }
            let var = Var::StudentGroup {
                student: student_id,
                group_list: *group_list_id,
            };
            if config_data.get(var.clone()).is_some() {
                continue;
            }
            config_data = config_data.set(var, -1.);
        }
    }

    // Zero-fill the unassigned group slots on every *possible* interrogation
    // cell. The set of possible cells is re-derived from the parameters —
    // `is_interrogation_possible` mirrors the dense skeleton's Some-cell rule —
    // rather than walked off the colloscope; on validated data the two coincide.
    // `ConfigData` is a map, so the enumeration order is invisible.
    for period_id in env.periods.period_ids() {
        let week_ids: Vec<WeekId> = env
            .periods
            .find_period(period_id)
            .expect("period id from period_ids is valid")
            .clone();
        for (slot_id, _slot) in env.slots.all_slots() {
            let (subject_id, _pos) = env
                .slots
                .find_slot_subject_and_position(*slot_id)
                .expect("Slot ID should be valid");
            let Some(group_list_id) = env
                .group_lists
                .subjects_associations
                .get(&(period_id, subject_id))
            else {
                continue;
            };
            let group_list = env
                .group_lists
                .group_list_map
                .get(group_list_id)
                .expect("Group list ID should be valid");

            for &week_id in &week_ids {
                if !env.is_interrogation_possible(*slot_id, week_id) {
                    continue;
                }
                let week = env
                    .periods
                    .global_week_position(week_id)
                    .expect("week id is valid");
                let assigned = colloscope.interrogation(*slot_id, week_id);

                for group_num in 0..group_list.params.group_names.len() {
                    if assigned.is_some_and(|groups| groups.contains(&(group_num as u32))) {
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
    }

    config_data
}

pub fn build_colloscope(env: &Parameters, config_data: &ConfigData<Var>) -> Option<Colloscope> {
    let mut colloscope = Colloscope::default();

    // Global week index → week id (canonical walk order).
    let week_ids: Vec<WeekId> = env
        .periods
        .walk()
        .map(|(_p, week_id, _w)| week_id)
        .collect();

    // Accumulate rows locally — this is 1d's sparse storage shape — then commit
    // them through the surface writers once each coordinate has been validated
    // against the parameters (the writers panic on an impossible coordinate).
    let mut interrogations: BTreeMap<(SlotId, WeekId), BTreeSet<u32>> = BTreeMap::new();
    let mut group_lists: BTreeMap<GroupListId, BTreeMap<StudentId, u32>> = BTreeMap::new();

    for (var, value) in config_data.get_values() {
        match var {
            Var::StudentGroup {
                student,
                group_list,
            } => {
                if value >= -0.1 {
                    // A colloscope row exists only for a valid, non-prefilled
                    // group list; anything else is a malformed config.
                    let data_group_list = env.group_lists.group_list_map.get(&group_list)?;
                    if data_group_list.is_prefilled() {
                        return None;
                    }
                    group_lists
                        .entry(group_list)
                        .or_default()
                        .insert(student, value as u32);
                }
            }
            Var::GroupInInterrogation { slot, week, group } => {
                if value < 0.5 {
                    continue;
                }
                let &week_id = week_ids.get(week.0)?;
                // Reject a group assigned on an impossible interrogation cell.
                if !env.is_interrogation_possible(slot, week_id) {
                    return None;
                }
                interrogations
                    .entry((slot, week_id))
                    .or_default()
                    .insert(group.index() as u32);
            }
        }
    }

    for ((slot_id, week_id), groups) in interrogations {
        colloscope.set_interrogation(slot_id, week_id, groups);
    }
    for (group_list_id, placements) in group_lists {
        colloscope.set_group_list(group_list_id, placements);
    }

    Some(colloscope)
}
