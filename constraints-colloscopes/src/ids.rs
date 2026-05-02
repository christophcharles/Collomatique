pub use collomatique_state_colloscopes::ids::{
    GroupListId, PeriodId, SlotId, StudentId, SubjectId,
};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlobalWeek(pub usize);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupNum(pub usize);
