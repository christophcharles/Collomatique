use super::*;

pub mod general_planning;
use general_planning::*;

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
