use super::*;

pub mod common;
pub use common::*;
pub mod general_planning;
pub use general_planning::*;
pub mod subjects;
pub use subjects::*;
pub mod teachers;
pub use teachers::*;
pub mod students;
pub use students::*;
pub mod assignments;
pub use assignments::*;
pub mod open_file_dialog;
pub use open_file_dialog::*;
pub mod week_patterns;
pub use week_patterns::*;
pub mod slots;
pub use slots::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmdMsg {
    Update(UpdateMsg),
    GuiRequest(GuiMsg),
    GetData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuiMsg {
    OpenFileDialog(OpenFileDialogMsg),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateMsg {
    GeneralPlanning(GeneralPlanningCmdMsg),
    Subjects(SubjectsCmdMsg),
    Teachers(TeachersCmdMsg),
    Students(StudentsCmdMsg),
    Assignments(AssignmentsCmdMsg),
    WeekPatterns(WeekPatternsCmdMsg),
    Slots(SlotsCmdMsg),
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
            UpdateMsg::Subjects(op) => crate::ops::UpdateOp::Subjects(op.promote(data)?),
            UpdateMsg::Teachers(op) => crate::ops::UpdateOp::Teachers(op.promote(data)?),
            UpdateMsg::Students(op) => crate::ops::UpdateOp::Students(op.promote(data)?),
            UpdateMsg::Assignments(op) => crate::ops::UpdateOp::Assignments(op.promote(data)?),
            UpdateMsg::WeekPatterns(op) => crate::ops::UpdateOp::WeekPatterns(op.promote(data)?),
            UpdateMsg::Slots(op) => crate::ops::UpdateOp::Slots(op.promote(data)?),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgTeacherId(pub u64);

impl From<collomatique_state_colloscopes::TeacherId> for MsgTeacherId {
    fn from(value: collomatique_state_colloscopes::TeacherId) -> Self {
        MsgTeacherId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgWeekPatternId(pub u64);

impl From<collomatique_state_colloscopes::WeekPatternId> for MsgWeekPatternId {
    fn from(value: collomatique_state_colloscopes::WeekPatternId) -> Self {
        MsgWeekPatternId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgSlotId(pub u64);

impl From<collomatique_state_colloscopes::SlotId> for MsgSlotId {
    fn from(value: collomatique_state_colloscopes::SlotId) -> Self {
        MsgSlotId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgIncompatId(pub u64);

impl From<collomatique_state_colloscopes::IncompatId> for MsgIncompatId {
    fn from(value: collomatique_state_colloscopes::IncompatId) -> Self {
        MsgIncompatId(value.inner())
    }
}
