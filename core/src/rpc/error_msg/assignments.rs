use crate::rpc::cmd_msg::{MsgPeriodId, MsgStudentId, MsgSubjectId};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentsError {
    Assign(AssignError),
    DuplicatePreviousPeriod(DuplicatePreviousPeriodError),
}

impl std::fmt::Display for AssignmentsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignmentsError::Assign(e) => e.fmt(f),
            AssignmentsError::DuplicatePreviousPeriod(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::AssignmentsUpdateError> for AssignmentsError {
    fn from(value: crate::ops::AssignmentsUpdateError) -> Self {
        use crate::ops::AssignmentsUpdateError;
        match value {
            AssignmentsUpdateError::Assign(e) => AssignmentsError::Assign(e.into()),
            AssignmentsUpdateError::DuplicatePreviousPeriod(e) => {
                AssignmentsError::DuplicatePreviousPeriod(e.into())
            }
        }
    }
}

impl From<AssignError> for AssignmentsError {
    fn from(value: AssignError) -> Self {
        AssignmentsError::Assign(value)
    }
}

impl From<DuplicatePreviousPeriodError> for AssignmentsError {
    fn from(value: DuplicatePreviousPeriodError) -> Self {
        AssignmentsError::DuplicatePreviousPeriod(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignError {
    InvalidPeriodId(MsgPeriodId),
    InvalidSubjectId(MsgSubjectId),
    InvalidStudentId(MsgStudentId),
    SubjectDoesNotRunOnPeriod(MsgSubjectId, MsgPeriodId),
    StudentIsNotPresentOnPeriod(MsgStudentId, MsgPeriodId),
}

impl std::fmt::Display for AssignError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
            AssignError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            AssignError::InvalidStudentId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun élève", id.0)
            }
            AssignError::StudentIsNotPresentOnPeriod(student_id, period_id) => {
                write!(
                    f,
                    "L'élève {} n'a pas cours sur la période {}",
                    student_id.0, period_id.0
                )
            }
            AssignError::SubjectDoesNotRunOnPeriod(subject_id, period_id) => {
                write!(
                    f,
                    "La matière {} n'est pas dispensée sur la période {}",
                    subject_id.0, period_id.0
                )
            }
        }
    }
}

impl From<crate::ops::AssignError> for AssignError {
    fn from(value: crate::ops::AssignError) -> Self {
        match value {
            crate::ops::AssignError::InvalidPeriodId(id) => AssignError::InvalidPeriodId(id.into()),
            crate::ops::AssignError::InvalidSubjectId(id) => {
                AssignError::InvalidSubjectId(id.into())
            }
            crate::ops::AssignError::InvalidStudentId(id) => {
                AssignError::InvalidStudentId(id.into())
            }
            crate::ops::AssignError::StudentIsNotPresentOnPeriod(student_id, period_id) => {
                AssignError::StudentIsNotPresentOnPeriod(student_id.into(), period_id.into())
            }
            crate::ops::AssignError::SubjectDoesNotRunOnPeriod(subject_id, period_id) => {
                AssignError::SubjectDoesNotRunOnPeriod(subject_id.into(), period_id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DuplicatePreviousPeriodError {
    InvalidPeriodId(MsgPeriodId),
    FirstPeriodHasNoPreviousPeriod(MsgPeriodId),
}

impl std::fmt::Display for DuplicatePreviousPeriodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DuplicatePreviousPeriodError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
            DuplicatePreviousPeriodError::FirstPeriodHasNoPreviousPeriod(_id) => {
                write!(f, "Il n'y a pas de période précédent la première période")
            }
        }
    }
}

impl From<crate::ops::DuplicatePreviousPeriodError> for DuplicatePreviousPeriodError {
    fn from(value: crate::ops::DuplicatePreviousPeriodError) -> Self {
        match value {
            crate::ops::DuplicatePreviousPeriodError::InvalidPeriodId(id) => {
                DuplicatePreviousPeriodError::InvalidPeriodId(id.into())
            }
            crate::ops::DuplicatePreviousPeriodError::FirstPeriodHasNoPreviousPeriod(id) => {
                DuplicatePreviousPeriodError::FirstPeriodHasNoPreviousPeriod(id.into())
            }
        }
    }
}
