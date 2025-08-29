use super::*;

pub mod general_planning;
pub use general_planning::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorMsg {
    GeneralPlanning(GeneralPlanningError),
}

impl From<crate::ops::UpdateError> for ErrorMsg {
    fn from(value: crate::ops::UpdateError) -> Self {
        use crate::ops::UpdateError;
        match value {
            UpdateError::GeneralPlanning(e) => ErrorMsg::GeneralPlanning(e.into()),
        }
    }
}

impl From<GeneralPlanningError> for ErrorMsg {
    fn from(value: GeneralPlanningError) -> Self {
        ErrorMsg::GeneralPlanning(value)
    }
}

impl std::fmt::Display for ErrorMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorMsg::GeneralPlanning(e) => e.fmt(f),
        }
    }
}
