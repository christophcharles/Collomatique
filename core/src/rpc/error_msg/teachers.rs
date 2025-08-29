use crate::rpc::cmd_msg::{MsgSubjectId, MsgTeacherId};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeachersError {
    AddNewTeacher(AddNewTeacherError),
    UpdateTeacher(UpdateTeacherError),
    DeleteTeacher(DeleteTeacherError),
}

impl std::fmt::Display for TeachersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TeachersError::AddNewTeacher(e) => e.fmt(f),
            TeachersError::UpdateTeacher(e) => e.fmt(f),
            TeachersError::DeleteTeacher(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::TeachersUpdateError> for TeachersError {
    fn from(value: crate::ops::TeachersUpdateError) -> Self {
        use crate::ops::TeachersUpdateError;
        match value {
            TeachersUpdateError::AddNewTeacher(e) => TeachersError::AddNewTeacher(e.into()),
            TeachersUpdateError::UpdateTeacher(e) => TeachersError::UpdateTeacher(e.into()),
            TeachersUpdateError::DeleteTeacher(e) => TeachersError::DeleteTeacher(e.into()),
        }
    }
}

impl From<AddNewTeacherError> for TeachersError {
    fn from(value: AddNewTeacherError) -> Self {
        TeachersError::AddNewTeacher(value)
    }
}

impl From<UpdateTeacherError> for TeachersError {
    fn from(value: UpdateTeacherError) -> Self {
        TeachersError::UpdateTeacher(value)
    }
}

impl From<DeleteTeacherError> for TeachersError {
    fn from(value: DeleteTeacherError) -> Self {
        TeachersError::DeleteTeacher(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddNewTeacherError {
    InvalidSubjectId(MsgSubjectId),
}

impl std::fmt::Display for AddNewTeacherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddNewTeacherError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
        }
    }
}

impl From<crate::ops::AddNewTeacherError> for AddNewTeacherError {
    fn from(value: crate::ops::AddNewTeacherError) -> Self {
        match value {
            crate::ops::AddNewTeacherError::InvalidSubjectId(id) => {
                AddNewTeacherError::InvalidSubjectId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateTeacherError {
    InvalidTeacherId(MsgTeacherId),
    InvalidSubjectId(MsgSubjectId),
}

impl std::fmt::Display for UpdateTeacherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateTeacherError::InvalidTeacherId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun colleur", id.0)
            }
            UpdateTeacherError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
        }
    }
}

impl From<crate::ops::UpdateTeacherError> for UpdateTeacherError {
    fn from(value: crate::ops::UpdateTeacherError) -> Self {
        match value {
            crate::ops::UpdateTeacherError::InvalidTeacherId(id) => {
                UpdateTeacherError::InvalidTeacherId(id.into())
            }
            crate::ops::UpdateTeacherError::InvalidSubjectId(id) => {
                UpdateTeacherError::InvalidSubjectId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteTeacherError {
    InvalidTeacherId(MsgTeacherId),
}

impl std::fmt::Display for DeleteTeacherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteTeacherError::InvalidTeacherId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun colleur", id.0)
            }
        }
    }
}

impl From<crate::ops::DeleteTeacherError> for DeleteTeacherError {
    fn from(value: crate::ops::DeleteTeacherError) -> Self {
        match value {
            crate::ops::DeleteTeacherError::InvalidTeacherId(id) => {
                DeleteTeacherError::InvalidTeacherId(id.into())
            }
        }
    }
}
