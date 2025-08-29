use crate::rpc::cmd_msg::MsgWeekPatternId;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeekPatternsError {
    UpdateWeekPattern(UpdateWeekPatternError),
    DeleteWeekPattern(DeleteWeekPatternError),
}

impl std::fmt::Display for WeekPatternsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeekPatternsError::UpdateWeekPattern(e) => e.fmt(f),
            WeekPatternsError::DeleteWeekPattern(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::WeekPatternsUpdateError> for WeekPatternsError {
    fn from(value: crate::ops::WeekPatternsUpdateError) -> Self {
        use crate::ops::WeekPatternsUpdateError;
        match value {
            WeekPatternsUpdateError::DeleteWeekPattern(e) => {
                WeekPatternsError::DeleteWeekPattern(e.into())
            }
            WeekPatternsUpdateError::UpdateWeekPattern(e) => {
                WeekPatternsError::UpdateWeekPattern(e.into())
            }
        }
    }
}

impl From<UpdateWeekPatternError> for WeekPatternsError {
    fn from(value: UpdateWeekPatternError) -> Self {
        WeekPatternsError::UpdateWeekPattern(value)
    }
}

impl From<DeleteWeekPatternError> for WeekPatternsError {
    fn from(value: DeleteWeekPatternError) -> Self {
        WeekPatternsError::DeleteWeekPattern(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateWeekPatternError {
    InvalidWeekPatternId(MsgWeekPatternId),
}

impl std::fmt::Display for UpdateWeekPatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateWeekPatternError::InvalidWeekPatternId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun modèle de périodicité",
                    id.0
                )
            }
        }
    }
}

impl From<crate::ops::UpdateWeekPatternError> for UpdateWeekPatternError {
    fn from(value: crate::ops::UpdateWeekPatternError) -> Self {
        match value {
            crate::ops::UpdateWeekPatternError::InvalidWeekPatternId(id) => {
                UpdateWeekPatternError::InvalidWeekPatternId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteWeekPatternError {
    InvalidWeekPatternId(MsgWeekPatternId),
}

impl std::fmt::Display for DeleteWeekPatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteWeekPatternError::InvalidWeekPatternId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun modèle de périodicité",
                    id.0
                )
            }
        }
    }
}

impl From<crate::ops::DeleteWeekPatternError> for DeleteWeekPatternError {
    fn from(value: crate::ops::DeleteWeekPatternError) -> Self {
        match value {
            crate::ops::DeleteWeekPatternError::InvalidWeekPatternId(id) => {
                DeleteWeekPatternError::InvalidWeekPatternId(id.into())
            }
        }
    }
}
