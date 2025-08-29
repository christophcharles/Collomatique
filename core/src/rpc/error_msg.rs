use super::*;

pub mod general_planning;
pub use general_planning::*;
pub mod subjects;
pub use subjects::*;
pub mod teachers;
pub use teachers::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorMsg {
    GeneralPlanning(GeneralPlanningError),
    Subjects(SubjectsError),
    Teachers(TeachersError),
}

impl From<crate::ops::UpdateError> for ErrorMsg {
    fn from(value: crate::ops::UpdateError) -> Self {
        use crate::ops::UpdateError;
        match value {
            UpdateError::GeneralPlanning(e) => ErrorMsg::GeneralPlanning(e.into()),
            UpdateError::Subjects(e) => ErrorMsg::Subjects(e.into()),
            UpdateError::Teachers(e) => ErrorMsg::Teachers(e.into()),
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

impl std::fmt::Display for ErrorMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorMsg::GeneralPlanning(e) => e.fmt(f),
            ErrorMsg::Subjects(e) => e.fmt(f),
            ErrorMsg::Teachers(e) => e.fmt(f),
        }
    }
}
