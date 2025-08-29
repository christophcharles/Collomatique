use super::*;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorMsg {
    GeneralPlanning(GeneralPlanningError),
    Subjects(SubjectsError),
    Teachers(TeachersError),
    Students(StudentsError),
    Assignments(AssignmentsError),
    WeekPatterns(WeekPatternsError),
    Slots(SlotsError),
    Incompats(IncompatibilitiesError),
    GroupLists(GroupListsError),
    Rules(RulesError),
}

impl From<crate::ops::UpdateError> for ErrorMsg {
    fn from(value: crate::ops::UpdateError) -> Self {
        use crate::ops::UpdateError;
        match value {
            UpdateError::GeneralPlanning(e) => ErrorMsg::GeneralPlanning(e.into()),
            UpdateError::Subjects(e) => ErrorMsg::Subjects(e.into()),
            UpdateError::Teachers(e) => ErrorMsg::Teachers(e.into()),
            UpdateError::Students(e) => ErrorMsg::Students(e.into()),
            UpdateError::Assignments(e) => ErrorMsg::Assignments(e.into()),
            UpdateError::WeekPatterns(e) => ErrorMsg::WeekPatterns(e.into()),
            UpdateError::Slots(e) => ErrorMsg::Slots(e.into()),
            UpdateError::Incompatibilities(e) => ErrorMsg::Incompats(e.into()),
            UpdateError::GroupLists(e) => ErrorMsg::GroupLists(e.into()),
            UpdateError::Rules(e) => ErrorMsg::Rules(e.into()),
        }
    }
}

impl From<GeneralPlanningError> for ErrorMsg {
    fn from(value: GeneralPlanningError) -> Self {
        ErrorMsg::GeneralPlanning(value)
    }
}

impl From<SubjectsError> for ErrorMsg {
    fn from(value: SubjectsError) -> Self {
        ErrorMsg::Subjects(value)
    }
}

impl From<TeachersError> for ErrorMsg {
    fn from(value: TeachersError) -> Self {
        ErrorMsg::Teachers(value)
    }
}

impl From<StudentsError> for ErrorMsg {
    fn from(value: StudentsError) -> Self {
        ErrorMsg::Students(value)
    }
}

impl From<AssignmentsError> for ErrorMsg {
    fn from(value: AssignmentsError) -> Self {
        ErrorMsg::Assignments(value)
    }
}

impl From<WeekPatternsError> for ErrorMsg {
    fn from(value: WeekPatternsError) -> Self {
        ErrorMsg::WeekPatterns(value)
    }
}

impl From<SlotsError> for ErrorMsg {
    fn from(value: SlotsError) -> Self {
        ErrorMsg::Slots(value)
    }
}

impl From<IncompatibilitiesError> for ErrorMsg {
    fn from(value: IncompatibilitiesError) -> Self {
        ErrorMsg::Incompats(value)
    }
}

impl From<GroupListsError> for ErrorMsg {
    fn from(value: GroupListsError) -> Self {
        ErrorMsg::GroupLists(value)
    }
}

impl From<RulesError> for ErrorMsg {
    fn from(value: RulesError) -> Self {
        ErrorMsg::Rules(value)
    }
}

impl std::fmt::Display for ErrorMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorMsg::GeneralPlanning(e) => e.fmt(f),
            ErrorMsg::Subjects(e) => e.fmt(f),
            ErrorMsg::Teachers(e) => e.fmt(f),
            ErrorMsg::Students(e) => e.fmt(f),
            ErrorMsg::Assignments(e) => e.fmt(f),
            ErrorMsg::WeekPatterns(e) => e.fmt(f),
            ErrorMsg::Slots(e) => e.fmt(f),
            ErrorMsg::Incompats(e) => e.fmt(f),
            ErrorMsg::GroupLists(e) => e.fmt(f),
            ErrorMsg::Rules(e) => e.fmt(f),
        }
    }
}
