use super::*;

pub mod general_planning;
pub use general_planning::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmdMsg {
    Update(UpdateMsg),
    GetData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateMsg {
    GeneralPlanning(GeneralPlanningCmdMsg),
}

impl From<crate::ops::UpdateOp> for UpdateMsg {
    fn from(value: crate::ops::UpdateOp) -> Self {
        match value {
            crate::ops::UpdateOp::GeneralPlanning(op) => UpdateMsg::GeneralPlanning(op.into()),
        }
    }
}

impl UpdateMsg {
    pub fn promote(
        self,
        data: &collomatique_state_colloscopes::Data,
    ) -> Result<crate::ops::UpdateOp, ErrorMsg> {
        Ok(match self {
            UpdateMsg::GeneralPlanning(op) => {
                crate::ops::UpdateOp::GeneralPlanning(op.promote(data)?)
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgPeriodId(pub u64);

impl From<collomatique_state_colloscopes::PeriodId> for MsgPeriodId {
    fn from(value: collomatique_state_colloscopes::PeriodId) -> Self {
        MsgPeriodId(value.inner())
    }
}
