use super::objects::{InterrogationData, WeekId};
use super::tools::*;
use collo_ml::{EvalObject, ViewBuilder, ViewObject};
use collomatique_state_colloscopes::{Data, GroupListId};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, EvalObject)]
#[env(Data)]
pub enum ObjectId {
    Interrogation(InterrogationData),
    Week(WeekId),
    GroupList(GroupListId),
}

#[derive(ViewObject)]
#[eval_object(ObjectId)]
pub struct Interrogation {
    week: WeekId,
    group_list: GroupListId,
}

#[derive(ViewObject)]
#[eval_object(ObjectId)]
pub struct Week {
    num: i32,
}

#[derive(ViewObject)]
#[eval_object(ObjectId)]
pub struct GroupList {
    groups: Vec<i32>,
}

impl ViewBuilder<Data, InterrogationData> for ObjectId {
    type Object = Interrogation;

    fn enumerate(env: &Data) -> BTreeSet<InterrogationData> {
        let mut output = BTreeSet::new();
        for (_subject_id, subject_slots) in &env.get_inner_data().params.slots.subject_map {
            for (slot_id, _slot_desc) in &subject_slots.ordered_slots {
                for week in 0..env.get_inner_data().params.periods.count_weeks() {
                    output.insert(InterrogationData {
                        slot: *slot_id,
                        week,
                    });
                }
            }
        }
        output
    }

    fn build(env: &Data, id: &InterrogationData) -> Option<Self::Object> {
        let (subject_id, _pos) = env
            .get_inner_data()
            .params
            .slots
            .find_slot_subject_and_position(id.slot)?;
        let period_id = week_to_period_id(env, id.week);
        let period_associations = env
            .get_inner_data()
            .params
            .group_lists
            .subjects_associations
            .get(&period_id)?;
        let group_list_id = period_associations.get(&subject_id)?;

        Some(Interrogation {
            week: WeekId(id.week),
            group_list: *group_list_id,
        })
    }
}

impl ViewBuilder<Data, WeekId> for ObjectId {
    type Object = Week;

    fn enumerate(env: &Data) -> BTreeSet<WeekId> {
        (0..env.get_inner_data().params.periods.count_weeks())
            .into_iter()
            .map(|x| WeekId(x))
            .collect()
    }

    fn build(_env: &Data, id: &WeekId) -> Option<Self::Object> {
        Some(Week { num: id.0 as i32 })
    }
}

impl ViewBuilder<Data, GroupListId> for ObjectId {
    type Object = GroupList;

    fn enumerate(env: &Data) -> BTreeSet<GroupListId> {
        env.get_inner_data()
            .params
            .group_lists
            .group_list_map
            .keys()
            .map(|x| *x)
            .collect()
    }

    fn build(env: &Data, id: &GroupListId) -> Option<Self::Object> {
        let group_list_data = env
            .get_inner_data()
            .params
            .group_lists
            .group_list_map
            .get(id)?;

        Some(GroupList {
            groups: (0..(*group_list_data.params.group_count.end() as i32))
                .into_iter()
                .collect(),
        })
    }
}
