use super::objects::InterrogationData;
use super::tools::*;
use collo_ml::EvalVar;
use collomatique_state_colloscopes::Data;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, EvalVar)]
#[env(Data)]
pub enum Var {
    GroupInInterrogation {
        interrogation: InterrogationData,
        #[range(Self::compute_group_range(env, interrogation))]
        group: i32,
    },
}

impl Var {
    fn compute_group_range(env: &Data, interrogation: &InterrogationData) -> std::ops::Range<i32> {
        let default_range = 0..0;
        let subject_id = match env
            .get_inner_data()
            .params
            .slots
            .find_slot_subject_and_position(interrogation.slot)
        {
            Some((subject_id, _pos)) => subject_id,
            None => return default_range,
        };
        let period_id = week_to_period_id(env, interrogation.week);
        let period_associations = match env
            .get_inner_data()
            .params
            .group_lists
            .subjects_associations
            .get(&period_id)
        {
            Some(period_associations) => period_associations,
            None => return default_range,
        };
        let group_list_id = match period_associations.get(&subject_id) {
            Some(id) => id,
            None => return default_range,
        };
        let group_list = match env
            .get_inner_data()
            .params
            .group_lists
            .group_list_map
            .get(group_list_id)
        {
            Some(group_list) => group_list,
            None => return default_range,
        };
        0..*group_list.params.group_count.end() as i32
    }
}
