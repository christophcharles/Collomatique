use super::tools::*;
use collo_ml::EvalVar;
use collomatique_state_colloscopes::colloscope_params::Parameters;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(Parameters)]
pub enum Var {
    #[defer_fix(Self::fix_group_in_interrogation(env, slot, week, group))]
    GroupInInterrogationInternal {
        #[range(Self::compute_slot_range(env))]
        slot: i32,
        #[range(Self::compute_week_range(env, slot))]
        week: i32,
        #[range(Self::compute_group_range(env, slot, week))]
        group: i32,
    },
    #[defer_fix(Self::fix_student_group(env, student, group_list))]
    #[var(Variable::integer().min(-1.).max(Self::compute_max_group_num(env, group_list)))]
    StudentGroup {
        #[range(Self::compute_group_list_range(env))]
        group_list: i32,
        #[range(Self::compute_student_range(env, group_list))]
        student: i32,
    },
}

impl Var {
    fn compute_max_group_num(env: &Parameters, group_list: &i32) -> f64 {
        use collomatique_state_colloscopes::ids::Id;
        let group_list_id =
            unsafe { collomatique_state_colloscopes::ids::GroupListId::new(*group_list as u64) };
        let group_list_data = match env.group_lists.group_list_map.get(&group_list_id) {
            Some(data) => data,
            None => return 0.,
        };
        (group_list_data.params.group_names.len() as i32 - 1) as f64
    }

    fn compute_slot_range(env: &Parameters) -> std::ops::Range<i32> {
        use collomatique_state_colloscopes::ids::Id;
        let ids = env
            .slots
            .subject_map
            .iter()
            .flat_map(|(_subject_id, subject_slots)| {
                subject_slots
                    .ordered_slots
                    .iter()
                    .map(|(id, _)| id.inner() as i32)
            });
        Self::compute_range_from_iter(ids)
    }

    fn enumerate_weeks_for_slot(env: &Parameters, slot: &i32) -> Vec<i32> {
        use collomatique_state_colloscopes::ids::Id;
        let slot_id = unsafe { collomatique_state_colloscopes::ids::SlotId::new(*slot as u64) };
        let Some((subject_id, pos)) = env.slots.find_slot_subject_and_position(slot_id) else {
            return vec![];
        };
        let slot_desc = &env.slots.subject_map[&subject_id].ordered_slots[pos].1;
        let subject_desc = env
            .subjects
            .find_subject(subject_id)
            .expect("Subject ID should be valid");

        let week_pattern = crate::tools::extract_week_pattern(env, slot_desc.week_pattern);
        let mut output = vec![];
        for (week, status) in week_pattern.into_iter().enumerate() {
            if !status {
                continue;
            }
            let (period, _) = crate::tools::week_to_period_id(env, week)
                .expect("Week should correspond to some period");
            if subject_desc.excluded_periods.contains(&period) {
                continue;
            }
            output.push(week as i32);
        }

        output
    }

    fn compute_week_range(env: &Parameters, slot: &i32) -> std::ops::Range<i32> {
        let weeks = Self::enumerate_weeks_for_slot(env, slot);
        Self::compute_range_from_iter(weeks.into_iter())
    }

    fn compute_group_range(env: &Parameters, slot: &i32, week: &i32) -> std::ops::Range<i32> {
        use collomatique_state_colloscopes::ids::Id;
        let slot_id = unsafe { collomatique_state_colloscopes::ids::SlotId::new(*slot as u64) };
        let week_num = *week as usize;
        let default_range = 0..0;
        let subject_id = match env.slots.find_slot_subject_and_position(slot_id) {
            Some((subject_id, _pos)) => subject_id,
            None => return default_range,
        };
        let period_id = match week_to_period_id(env, week_num) {
            Some((id, _)) => id,
            None => return default_range,
        };
        let period_associations = match env.group_lists.subjects_associations.get(&period_id) {
            Some(period_associations) => period_associations,
            None => return default_range,
        };
        let group_list_id = match period_associations.get(&subject_id) {
            Some(id) => id,
            None => return default_range,
        };
        let group_list = match env.group_lists.group_list_map.get(group_list_id) {
            Some(group_list) => group_list,
            None => return default_range,
        };
        0..group_list.params.group_names.len() as i32
    }

    fn compute_range_from_iter(ids: impl Iterator<Item = i32>) -> std::ops::Range<i32> {
        let mut group_list_min = i32::MAX;
        let mut group_list_max = 0;
        for id in ids {
            if id < group_list_min {
                group_list_min = id;
            }
            if id > group_list_max {
                group_list_max = id;
            }
        }
        if group_list_max < group_list_min {
            return 0..0;
        }
        group_list_min..group_list_max + 1
    }

    fn compute_group_list_range(env: &Parameters) -> std::ops::Range<i32> {
        use collomatique_state_colloscopes::ids::Id;
        let ids = env
            .group_lists
            .group_list_map
            .keys()
            .map(|id| id.inner() as i32);
        Self::compute_range_from_iter(ids)
    }

    fn compute_student_ids(env: &Parameters, group_list: &i32) -> Vec<i32> {
        use collomatique_state_colloscopes::ids::Id;
        let group_list_id =
            unsafe { collomatique_state_colloscopes::ids::GroupListId::new(*group_list as u64) };
        let group_list = match env.group_lists.group_list_map.get(&group_list_id) {
            Some(group_list) => group_list,
            None => return Vec::new(),
        };
        match &group_list.filling {
            collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                excluded_students,
            } => env
                .students
                .student_map
                .keys()
                .filter_map(|student_id| {
                    if excluded_students.contains(student_id) {
                        return None;
                    }
                    Some(student_id.inner() as i32)
                })
                .collect::<Vec<_>>(),
            collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups } => {
                groups
                    .iter()
                    .flat_map(|g| g.students.iter().map(|x| x.inner() as i32))
                    .collect::<Vec<_>>()
            }
        }
    }

    fn compute_student_range(env: &Parameters, group_list: &i32) -> std::ops::Range<i32> {
        let ids = Self::compute_student_ids(env, group_list);
        Self::compute_range_from_iter(ids.into_iter())
    }

    fn fix_student_group(env: &Parameters, student: &i32, group_list: &i32) -> Option<f64> {
        use collomatique_state_colloscopes::ids::Id;
        let group_list_id =
            unsafe { collomatique_state_colloscopes::ids::GroupListId::new(*group_list as u64) };
        let group_list_data = match env.group_lists.group_list_map.get(&group_list_id) {
            Some(data) => data,
            None => return Some(-1.),
        };

        let student_id =
            unsafe { collomatique_state_colloscopes::ids::StudentId::new(*student as u64) };
        if group_list_data
            .filling
            .excluded_students()
            .contains(&student_id)
        {
            return Some(-1.);
        }

        let collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { .. } =
            &group_list_data.filling
        else {
            return None;
        };

        let Some(num) = group_list_data.filling.find_student_group(student_id) else {
            return Some(-1.0);
        };

        Some(num as f64)
    }

    fn fix_group_in_interrogation(
        env: &Parameters,
        slot: &i32,
        week: &i32,
        group: &i32,
    ) -> Option<f64> {
        use collomatique_state_colloscopes::ids::Id;
        let slot_id = unsafe { collomatique_state_colloscopes::ids::SlotId::new(*slot as u64) };
        if env.slots.find_slot(slot_id).is_none() {
            return Some(0.0); // Non existent slot, so obviously not in it
        };

        let weeks = Self::enumerate_weeks_for_slot(env, slot);
        if !weeks.contains(week) {
            return Some(0.0); // Invalid week
        }

        let group_range = Self::compute_group_range(env, slot, week);
        if !group_range.contains(group) {
            return Some(0.0); // Invalid group
        }

        None
    }
}
