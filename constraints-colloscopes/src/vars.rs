use crate::ids::{GlobalWeek, GroupNum};
use crate::tools::*;
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::ids::{GroupListId, SlotId, StudentId};

#[derive(Debug, Clone)]
pub struct VarEnv(pub Parameters);

impl std::ops::Deref for VarEnv {
    type Target = Parameters;
    fn deref(&self) -> &Parameters {
        &self.0
    }
}

impl VarEnv {
    pub fn new(params: Parameters) -> VarEnv {
        VarEnv(params)
    }
}

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, collomatique_ilp_modeler::DescribeVar,
)]
#[env(VarEnv)]
pub enum Var {
    #[defer_fix(Self::fix_group_in_interrogation(env, slot, week, group))]
    GroupInInterrogation {
        #[range(Self::compute_slot_range(env))]
        slot: SlotId,
        #[range(Self::compute_week_range(env, slot))]
        week: GlobalWeek,
        #[range(Self::compute_group_range(env, slot, week))]
        group: GroupNum,
    },
    #[defer_fix(Self::fix_student_group(env, student, group_list))]
    #[var(Variable::integer().min(-1.).max(Self::compute_max_group_num(env, group_list)))]
    StudentGroup {
        #[range(Self::compute_group_list_range(env))]
        group_list: GroupListId,
        #[range(Self::compute_student_range(env, group_list))]
        student: StudentId,
    },
}

impl Var {
    fn compute_max_group_num(env: &VarEnv, group_list: &GroupListId) -> f64 {
        let group_list_data = match env.group_lists.group_list_map.get(group_list) {
            Some(data) => data,
            None => return 0.,
        };
        (group_list_data.params.group_names.len() as i32 - 1) as f64
    }

    pub fn compute_slot_range(env: &VarEnv) -> Vec<SlotId> {
        env.slots.all_slots().map(|(id, _)| *id).collect()
    }

    pub fn enumerate_weeks_for_slot(env: &VarEnv, slot: &SlotId) -> Vec<GlobalWeek> {
        crate::tools::enumerate_weeks_for_slot_id(env, *slot)
            .into_iter()
            .map(GlobalWeek)
            .collect()
    }

    pub fn compute_week_range(env: &VarEnv, slot: &SlotId) -> Vec<GlobalWeek> {
        Self::enumerate_weeks_for_slot(env, slot)
    }

    pub fn compute_group_range(env: &VarEnv, slot: &SlotId, week: &GlobalWeek) -> Vec<GroupNum> {
        let default = vec![];
        let subject_id = match env.slots.find_slot_subject_and_position(*slot) {
            Some((subject_id, _pos)) => subject_id,
            None => return default,
        };
        let period_id = match week_to_period_id(env, week.0) {
            Some((id, _)) => id,
            None => return default,
        };
        let group_list_id = match env
            .group_lists
            .subjects_associations
            .get(&(period_id, subject_id))
        {
            Some(id) => id,
            None => return default,
        };
        if !env.group_lists.group_list_map.contains(group_list_id) {
            return default;
        }
        GroupNum::enumerate(env, *group_list_id).collect()
    }

    pub fn compute_group_list_range(env: &VarEnv) -> Vec<GroupListId> {
        env.group_lists.group_list_map.keys().collect()
    }

    pub fn compute_student_ids(env: &VarEnv, group_list: &GroupListId) -> Vec<StudentId> {
        let group_list_data = match env.group_lists.group_list_map.get(group_list) {
            Some(group_list) => group_list,
            None => return Vec::new(),
        };
        match &group_list_data.filling {
            collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                excluded_students,
            } => env
                .students
                .student_map
                .keys()
                .filter(|student_id| !excluded_students.contains(student_id))
                .collect(),
            collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups } => {
                groups
                    .iter()
                    .flat_map(|g| g.students.iter().copied())
                    .collect()
            }
        }
    }

    pub fn compute_student_range(env: &VarEnv, group_list: &GroupListId) -> Vec<StudentId> {
        Self::compute_student_ids(env, group_list)
    }

    fn fix_student_group(
        env: &VarEnv,
        student: &StudentId,
        group_list: &GroupListId,
    ) -> Option<f64> {
        let group_list_data = match env.group_lists.group_list_map.get(group_list) {
            Some(data) => data,
            None => return Some(-1.),
        };

        if group_list_data
            .filling
            .excluded_students()
            .contains(student)
        {
            return Some(-1.);
        }

        let collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { .. } =
            &group_list_data.filling
        else {
            return None;
        };

        let Some(num) = group_list_data.filling.find_student_group(*student) else {
            return Some(-1.0);
        };

        Some(num as f64)
    }

    fn fix_group_in_interrogation(
        env: &VarEnv,
        slot: &SlotId,
        week: &GlobalWeek,
        group: &GroupNum,
    ) -> Option<f64> {
        if env.slots.find_slot(*slot).is_none() {
            return Some(0.0);
        };

        let weeks = Self::enumerate_weeks_for_slot(env, slot);
        if !weeks.contains(week) {
            return Some(0.0);
        }

        let groups = Self::compute_group_range(env, slot, week);
        if !groups.contains(group) {
            return Some(0.0);
        }

        None
    }
}
