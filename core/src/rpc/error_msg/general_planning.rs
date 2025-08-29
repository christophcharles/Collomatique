use crate::rpc::cmd_msg::MsgPeriodId;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralPlanningError {
    UpdatePeriodWeekCount(UpdatePeriodWeekCountError),
    DeletePeriod(DeletePeriodError),
    CutPeriod(CutPeriodError),
    MergeWithPreviousPeriod(MergeWithPreviousPeriodError),
    UpdateWeekStatus(UpdateWeekStatusError),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdatePeriodWeekCountError {
    InvalidPeriodId(MsgPeriodId),
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
