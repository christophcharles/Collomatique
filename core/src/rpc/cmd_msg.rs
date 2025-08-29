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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgStudentId(pub u64);

impl From<collomatique_state_colloscopes::StudentId> for MsgStudentId {
    fn from(value: collomatique_state_colloscopes::StudentId) -> Self {
        MsgStudentId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgSubjectId(pub u64);

impl From<collomatique_state_colloscopes::SubjectId> for MsgSubjectId {
    fn from(value: collomatique_state_colloscopes::SubjectId) -> Self {
        MsgSubjectId(value.inner())
    }
}
