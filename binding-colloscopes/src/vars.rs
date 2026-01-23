use super::objects::InterrogationData;
use super::tools::*;
use super::views::Env;
use collo_ml::EvalVar;
use collomatique_state_colloscopes::{GroupListId, StudentId};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(Env)]
pub enum Var {
    GroupInInterrogation {
        interrogation: InterrogationData,
        #[range(Self::compute_group_range(env, interrogation))]
        group: i32,
    },
    #[defer_fix(Self::fix_student_group(env, student, group_list))]
    #[var(Variable::integer().min(-1.).max(Self::compute_max_group_num(env, group_list)))]
    StudentGroup {
        student: StudentId,
        group_list: GroupListId,
    },
}

impl Var {
    fn compute_max_group_num(env: &Env, group_list: &GroupListId) -> f64 {
        let group_list_data = match env.params.group_lists.group_list_map.get(group_list) {
            Some(data) => data,
            None => return 0.,
        };
        (group_list_data.params.max_group_count - 1) as f64
    }

    fn compute_group_range(env: &Env, interrogation: &InterrogationData) -> std::ops::Range<i32> {
        let default_range = 0..0;
        let subject_id = match env
            .params
            .slots
            .find_slot_subject_and_position(interrogation.slot)
        {
            Some((subject_id, _pos)) => subject_id,
            None => return default_range,
        };
        let period_id = match week_to_period_id(&env.params, interrogation.week) {
            Some((id, _)) => id,
            None => return default_range,
        };
        let period_associations = match env.params.group_lists.subjects_associations.get(&period_id)
        {
            Some(period_associations) => period_associations,
            None => return default_range,
        };
        let group_list_id = match period_associations.get(&subject_id) {
            Some(id) => id,
            None => return default_range,
        };
        let group_list = match env.params.group_lists.group_list_map.get(group_list_id) {
            Some(group_list) => group_list,
            None => return default_range,
        };
        0..group_list.params.max_group_count as i32
    }

    fn fix_student_group(env: &Env, student: &StudentId, group_list: &GroupListId) -> Option<f64> {
        let group_list_data = match env.params.group_lists.group_list_map.get(group_list) {
            Some(data) => data,
            None => return Some(-1.),
        };

        if group_list_data.params.excluded_students.contains(student) {
            return Some(-1.);
        }

        if env.ignore_prefill_for_group_lists.contains(group_list) {
            return None;
        }

        let Some(num) = group_list_data
            .prefilled_groups
            .find_student_group(*student)
        else {
            return None;
        };

        Some(num as f64)
    }
}
