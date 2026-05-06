pub use collomatique_state_colloscopes::ids::{
    GroupListId, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId,
};

use collomatique_state_colloscopes::colloscope_params::Parameters;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlobalWeek(pub usize);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupNum {
    group_list: GroupListId,
    last_group: usize,
    group: usize,
}

impl GroupNum {
    pub fn new(env: &Parameters, group_list: GroupListId, index: usize) -> Option<GroupNum> {
        let gl = env.group_lists.group_list_map.get(&group_list)?;
        let count = gl.params.group_names.len();
        if index >= count {
            return None;
        }
        Some(GroupNum {
            group_list,
            last_group: count - 1,
            group: index,
        })
    }

    pub fn index(&self) -> usize {
        self.group
    }

    pub fn group_list(&self) -> GroupListId {
        self.group_list
    }

    pub fn next(self) -> Option<GroupNum> {
        if self.group >= self.last_group {
            return None;
        }
        Some(GroupNum {
            group: self.group + 1,
            ..self
        })
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(group_list: GroupListId, index: usize) -> GroupNum {
        GroupNum {
            group_list,
            last_group: index,
            group: index,
        }
    }

    pub fn enumerate(env: &Parameters, group_list: GroupListId) -> impl Iterator<Item = GroupNum> {
        let count = env
            .group_lists
            .group_list_map
            .get(&group_list)
            .map(|gl| gl.params.group_names.len())
            .unwrap_or(0);
        let last_group = count.saturating_sub(1);
        (0..count).map(move |i| GroupNum {
            group_list,
            last_group,
            group: i,
        })
    }
}
