use crate::rpc::cmd_msg::MsgPeriodId;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralPlanningError {
    UpdateFirstWeek(UpdateFirstWeekError),
    UpdatePeriodWeekCount(UpdatePeriodWeekCountError),
    DeletePeriod(DeletePeriodError),
    CutPeriod(CutPeriodError),
    MergeWithPreviousPeriod(MergeWithPreviousPeriodError),
    UpdateWeekStatus(UpdateWeekStatusError),
}

impl std::fmt::Display for GeneralPlanningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeneralPlanningError::UpdateFirstWeek(e) => e.fmt(f),
            GeneralPlanningError::UpdatePeriodWeekCount(e) => e.fmt(f),
            GeneralPlanningError::DeletePeriod(e) => e.fmt(f),
            GeneralPlanningError::CutPeriod(e) => e.fmt(f),
            GeneralPlanningError::MergeWithPreviousPeriod(e) => e.fmt(f),
            GeneralPlanningError::UpdateWeekStatus(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::GeneralPlanningUpdateError> for GeneralPlanningError {
    fn from(value: crate::ops::GeneralPlanningUpdateError) -> Self {
        use crate::ops::GeneralPlanningUpdateError;
        match value {
            GeneralPlanningUpdateError::UpdatePeriodWeekCount(e) => {
                GeneralPlanningError::UpdatePeriodWeekCount(e.into())
            }
            GeneralPlanningUpdateError::DeletePeriod(e) => {
                GeneralPlanningError::DeletePeriod(e.into())
            }
            GeneralPlanningUpdateError::CutPeriod(e) => GeneralPlanningError::CutPeriod(e.into()),
            GeneralPlanningUpdateError::MergeWithPreviousPeriod(e) => {
                GeneralPlanningError::MergeWithPreviousPeriod(e.into())
            }
            GeneralPlanningUpdateError::UpdateWeekStatus(e) => {
                GeneralPlanningError::UpdateWeekStatus(e.into())
            }
        }
    }
}

impl From<UpdateFirstWeekError> for GeneralPlanningError {
    fn from(value: UpdateFirstWeekError) -> Self {
        GeneralPlanningError::UpdateFirstWeek(value)
    }
}

impl From<UpdatePeriodWeekCountError> for GeneralPlanningError {
    fn from(value: UpdatePeriodWeekCountError) -> Self {
        GeneralPlanningError::UpdatePeriodWeekCount(value)
    }
}

impl From<DeletePeriodError> for GeneralPlanningError {
    fn from(value: DeletePeriodError) -> Self {
        GeneralPlanningError::DeletePeriod(value)
    }
}

impl From<CutPeriodError> for GeneralPlanningError {
    fn from(value: CutPeriodError) -> Self {
        GeneralPlanningError::CutPeriod(value)
    }
}

impl From<MergeWithPreviousPeriodError> for GeneralPlanningError {
    fn from(value: MergeWithPreviousPeriodError) -> Self {
        GeneralPlanningError::MergeWithPreviousPeriod(value)
    }
}

impl From<UpdateWeekStatusError> for GeneralPlanningError {
    fn from(value: UpdateWeekStatusError) -> Self {
        GeneralPlanningError::UpdateWeekStatus(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateFirstWeekError {
    DateIsNotAMonday,
}

impl std::fmt::Display for UpdateFirstWeekError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Semaine ne commençant pas sur un lundi")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdatePeriodWeekCountError {
    InvalidPeriodId(MsgPeriodId),
}

impl std::fmt::Display for UpdatePeriodWeekCountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdatePeriodWeekCountError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
        }
    }
}

impl From<crate::ops::UpdatePeriodWeekCountError> for UpdatePeriodWeekCountError {
    fn from(value: crate::ops::UpdatePeriodWeekCountError) -> Self {
        match value {
            crate::ops::UpdatePeriodWeekCountError::InvalidPeriodId(id) => {
                UpdatePeriodWeekCountError::InvalidPeriodId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeletePeriodError {
    InvalidPeriodId(MsgPeriodId),
}

impl std::fmt::Display for DeletePeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeletePeriodError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
        }
    }
}

impl From<crate::ops::DeletePeriodError> for DeletePeriodError {
    fn from(value: crate::ops::DeletePeriodError) -> Self {
        match value {
            crate::ops::DeletePeriodError::InvalidPeriodId(id) => {
                DeletePeriodError::InvalidPeriodId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CutPeriodError {
    InvalidPeriodId(MsgPeriodId),
    RemainingWeekCountTooBig(usize, usize),
}

impl std::fmt::Display for CutPeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CutPeriodError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
            CutPeriodError::RemainingWeekCountTooBig(r, t) => {
                write!(f, "Ne peut conserver {} semaines (max = {})", r, t)
            }
        }
    }
}

impl From<crate::ops::CutPeriodError> for CutPeriodError {
    fn from(value: crate::ops::CutPeriodError) -> Self {
        match value {
            crate::ops::CutPeriodError::InvalidPeriodId(id) => {
                CutPeriodError::InvalidPeriodId(id.into())
            }
            crate::ops::CutPeriodError::RemainingWeekCountTooBig(r, t) => {
                CutPeriodError::RemainingWeekCountTooBig(r, t)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeWithPreviousPeriodError {
    InvalidPeriodId(MsgPeriodId),
    NoPreviousPeriodToMergeWith,
}

impl std::fmt::Display for MergeWithPreviousPeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeWithPreviousPeriodError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
            MergeWithPreviousPeriodError::NoPreviousPeriodToMergeWith => {
                write!(
                    f,
                    "Ne peut fusionner avec la période précédente : c'est la première période"
                )
            }
        }
    }
}

impl From<crate::ops::MergeWithPreviousPeriodError> for MergeWithPreviousPeriodError {
    fn from(value: crate::ops::MergeWithPreviousPeriodError) -> Self {
        match value {
            crate::ops::MergeWithPreviousPeriodError::InvalidPeriodId(id) => {
                MergeWithPreviousPeriodError::InvalidPeriodId(id.into())
            }
            crate::ops::MergeWithPreviousPeriodError::NoPreviousPeriodToMergeWith => {
                MergeWithPreviousPeriodError::NoPreviousPeriodToMergeWith
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateWeekStatusError {
    InvalidPeriodId(MsgPeriodId),
    InvalidWeekNumber(usize, usize),
}

impl std::fmt::Display for UpdateWeekStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateWeekStatusError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
            UpdateWeekStatusError::InvalidWeekNumber(w, t) => {
                write!(f, "Numéro de semaine {} trop grand (max = {})", w, t)
            }
        }
    }
}

impl From<crate::ops::UpdateWeekStatusError> for UpdateWeekStatusError {
    fn from(value: crate::ops::UpdateWeekStatusError) -> Self {
        match value {
            crate::ops::UpdateWeekStatusError::InvalidPeriodId(id) => {
                UpdateWeekStatusError::InvalidPeriodId(id.into())
            }
            crate::ops::UpdateWeekStatusError::InvalidWeekNumber(n, t) => {
                UpdateWeekStatusError::InvalidWeekNumber(n, t)
            }
        }
    }
}
