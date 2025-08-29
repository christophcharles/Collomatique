use crate::rpc::cmd_msg::{MsgPeriodId, MsgStudentId};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StudentsError {
    AddNewStudent(AddNewStudentError),
    UpdateStudent(UpdateStudentError),
    DeleteStudent(DeleteStudentError),
}

impl std::fmt::Display for StudentsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StudentsError::AddNewStudent(e) => e.fmt(f),
            StudentsError::UpdateStudent(e) => e.fmt(f),
            StudentsError::DeleteStudent(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::StudentsUpdateError> for StudentsError {
    fn from(value: crate::ops::StudentsUpdateError) -> Self {
        use crate::ops::StudentsUpdateError;
        match value {
            StudentsUpdateError::AddNewStudent(e) => StudentsError::AddNewStudent(e.into()),
            StudentsUpdateError::UpdateStudent(e) => StudentsError::UpdateStudent(e.into()),
            StudentsUpdateError::DeleteStudent(e) => StudentsError::DeleteStudent(e.into()),
        }
    }
}

impl From<AddNewStudentError> for StudentsError {
    fn from(value: AddNewStudentError) -> Self {
        StudentsError::AddNewStudent(value)
    }
}

impl From<UpdateStudentError> for StudentsError {
    fn from(value: UpdateStudentError) -> Self {
        StudentsError::UpdateStudent(value)
    }
}

impl From<DeleteStudentError> for StudentsError {
    fn from(value: DeleteStudentError) -> Self {
        StudentsError::DeleteStudent(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddNewStudentError {
    InvalidPeriodId(MsgPeriodId),
}

impl std::fmt::Display for AddNewStudentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddNewStudentError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
        }
    }
}

impl From<crate::ops::AddNewStudentError> for AddNewStudentError {
    fn from(value: crate::ops::AddNewStudentError) -> Self {
        match value {
            crate::ops::AddNewStudentError::InvalidPeriodId(id) => {
                AddNewStudentError::InvalidPeriodId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateStudentError {
    InvalidStudentId(MsgStudentId),
    InvalidPeriodId(MsgPeriodId),
}

impl std::fmt::Display for UpdateStudentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateStudentError::InvalidStudentId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun élève", id.0)
            }
            UpdateStudentError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
        }
    }
}

impl From<crate::ops::UpdateStudentError> for UpdateStudentError {
    fn from(value: crate::ops::UpdateStudentError) -> Self {
        match value {
            crate::ops::UpdateStudentError::InvalidStudentId(id) => {
                UpdateStudentError::InvalidStudentId(id.into())
            }
            crate::ops::UpdateStudentError::InvalidPeriodId(id) => {
                UpdateStudentError::InvalidPeriodId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteStudentError {
    InvalidStudentId(MsgStudentId),
}

impl std::fmt::Display for DeleteStudentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteStudentError::InvalidStudentId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun élève", id.0)
            }
        }
    }
}

impl From<crate::ops::DeleteStudentError> for DeleteStudentError {
    fn from(value: crate::ops::DeleteStudentError) -> Self {
        match value {
            crate::ops::DeleteStudentError::InvalidStudentId(id) => {
                DeleteStudentError::InvalidStudentId(id.into())
            }
        }
    }
}
