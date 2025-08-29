use crate::rpc::cmd_msg::{MsgPeriodId, MsgStudentId, MsgSubjectId};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignmentsError {
    Assign(AssignError),
}

impl std::fmt::Display for AssignmentsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignmentsError::Assign(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::AssignmentsUpdateError> for AssignmentsError {
    fn from(value: crate::ops::AssignmentsUpdateError) -> Self {
        use crate::ops::AssignmentsUpdateError;
        match value {
            AssignmentsUpdateError::Assign(e) => AssignmentsError::Assign(e.into()),
        }
    }
}

impl From<AssignError> for AssignmentsError {
    fn from(value: AssignError) -> Self {
        AssignmentsError::Assign(value)
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
