pub use collomatique_state_colloscopes::ids::{
    GroupListId, PeriodId, SlotId, StudentId, SubjectId,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlobalWeek(pub u32);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupNum(pub u32);
