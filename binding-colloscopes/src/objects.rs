use collomatique_state_colloscopes::SlotId;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct WeekId(pub usize);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct InterrogationData {
    pub slot: SlotId,
    pub week: usize,
}
