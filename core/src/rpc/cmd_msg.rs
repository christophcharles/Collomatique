use super::*;

pub mod general_planning;
pub use general_planning::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmdMsg {
    GeneralPlanning(GeneralPlanningCmdMsg),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MsgPeriodId(pub u64);

impl From<collomatique_state_colloscopes::PeriodId> for MsgPeriodId {
    fn from(value: collomatique_state_colloscopes::PeriodId) -> Self {
        MsgPeriodId(value.inner())
    }
}
