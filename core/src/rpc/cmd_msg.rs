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
pub mod incompatibilities;
pub use incompatibilities::*;
pub mod group_lists;
pub use group_lists::*;
pub mod rules;
pub use rules::*;
pub mod settings;
pub use settings::*;
pub mod colloscopes;
pub use colloscopes::*;

use collomatique_state_colloscopes::ids::Id;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmdMsg {
    Update(crate::ops::UpdateOp),
    GuiRequest(GuiMsg),
    GetData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuiMsg {
    OpenFileDialog(OpenFileDialogMsg),
    OkDialog(String),
    ConfirmDialog(String),
    InputDialog(String, String),
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
    Incompats(IncompatibilitiesCmdMsg),
    GroupLists(GroupListsCmdMsg),
    Rules(RulesCmdMsg),
    Settings(SettingsCmdMsg),
    Colloscopes(ColloscopesCmdMsg),
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
            UpdateMsg::Incompats(op) => crate::ops::UpdateOp::Incompatibilities(op.promote(data)?),
            UpdateMsg::GroupLists(op) => crate::ops::UpdateOp::GroupLists(op.promote(data)?),
            UpdateMsg::Rules(op) => crate::ops::UpdateOp::Rules(op.promote(data)?),
            UpdateMsg::Settings(op) => crate::ops::UpdateOp::Settings(op.promote(data)),
            UpdateMsg::Colloscopes(op) => crate::ops::UpdateOp::Colloscopes(op.promote(data)?),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgGroupListId(pub u64);

impl From<collomatique_state_colloscopes::GroupListId> for MsgGroupListId {
    fn from(value: collomatique_state_colloscopes::GroupListId) -> Self {
        MsgGroupListId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgRuleId(pub u64);

impl From<collomatique_state_colloscopes::RuleId> for MsgRuleId {
    fn from(value: collomatique_state_colloscopes::RuleId) -> Self {
        MsgRuleId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgColloscopeId(pub u64);

impl From<collomatique_state_colloscopes::ColloscopeId> for MsgColloscopeId {
    fn from(value: collomatique_state_colloscopes::ColloscopeId) -> Self {
        MsgColloscopeId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgColloscopePeriodId(pub u64);

impl From<collomatique_state_colloscopes::ColloscopePeriodId> for MsgColloscopePeriodId {
    fn from(value: collomatique_state_colloscopes::ColloscopePeriodId) -> Self {
        MsgColloscopePeriodId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgColloscopeStudentId(pub u64);

impl From<collomatique_state_colloscopes::ColloscopeStudentId> for MsgColloscopeStudentId {
    fn from(value: collomatique_state_colloscopes::ColloscopeStudentId) -> Self {
        MsgColloscopeStudentId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgColloscopeSubjectId(pub u64);

impl From<collomatique_state_colloscopes::ColloscopeSubjectId> for MsgColloscopeSubjectId {
    fn from(value: collomatique_state_colloscopes::ColloscopeSubjectId) -> Self {
        MsgColloscopeSubjectId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgColloscopeTeacherId(pub u64);

impl From<collomatique_state_colloscopes::ColloscopeTeacherId> for MsgColloscopeTeacherId {
    fn from(value: collomatique_state_colloscopes::ColloscopeTeacherId) -> Self {
        MsgColloscopeTeacherId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgColloscopeWeekPatternId(pub u64);

impl From<collomatique_state_colloscopes::ColloscopeWeekPatternId> for MsgColloscopeWeekPatternId {
    fn from(value: collomatique_state_colloscopes::ColloscopeWeekPatternId) -> Self {
        MsgColloscopeWeekPatternId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgColloscopeSlotId(pub u64);

impl From<collomatique_state_colloscopes::ColloscopeSlotId> for MsgColloscopeSlotId {
    fn from(value: collomatique_state_colloscopes::ColloscopeSlotId) -> Self {
        MsgColloscopeSlotId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgColloscopeIncompatId(pub u64);

impl From<collomatique_state_colloscopes::ColloscopeIncompatId> for MsgColloscopeIncompatId {
    fn from(value: collomatique_state_colloscopes::ColloscopeIncompatId) -> Self {
        MsgColloscopeIncompatId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgColloscopeGroupListId(pub u64);

impl From<collomatique_state_colloscopes::ColloscopeGroupListId> for MsgColloscopeGroupListId {
    fn from(value: collomatique_state_colloscopes::ColloscopeGroupListId) -> Self {
        MsgColloscopeGroupListId(value.inner())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MsgColloscopeRuleId(pub u64);

impl From<collomatique_state_colloscopes::ColloscopeRuleId> for MsgColloscopeRuleId {
    fn from(value: collomatique_state_colloscopes::ColloscopeRuleId) -> Self {
        MsgColloscopeRuleId(value.inner())
    }
}
