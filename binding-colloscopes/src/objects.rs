use collomatique_state_colloscopes::{GroupListId, SlotId};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct WeekId(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct InterrogationData {
    pub slot: SlotId,
    pub week: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct GroupId {
    pub group_list: GroupListId,
    pub num: i32,
}
