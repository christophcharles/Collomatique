use crate::rpc::cmd_msg::{MsgPeriodId, MsgSubjectId};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubjectsError {
    AddNewSubject(AddNewSubjectError),
    UpdateSubject(UpdateSubjectError),
    DeleteSubject(DeleteSubjectError),
    MoveUp(MoveUpError),
    MoveDown(MoveDownError),
    UpdatePeriodStatus(UpdatePeriodStatusError),
}

impl std::fmt::Display for SubjectsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubjectsError::AddNewSubject(e) => e.fmt(f),
            SubjectsError::UpdateSubject(e) => e.fmt(f),
            SubjectsError::DeleteSubject(e) => e.fmt(f),
            SubjectsError::MoveUp(e) => e.fmt(f),
            SubjectsError::MoveDown(e) => e.fmt(f),
            SubjectsError::UpdatePeriodStatus(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::SubjectsUpdateError> for SubjectsError {
    fn from(value: crate::ops::SubjectsUpdateError) -> Self {
        use crate::ops::SubjectsUpdateError;
        match value {
            SubjectsUpdateError::AddNewSubject(e) => SubjectsError::AddNewSubject(e.into()),
            SubjectsUpdateError::UpdateSubject(e) => SubjectsError::UpdateSubject(e.into()),
            SubjectsUpdateError::DeleteSubject(e) => SubjectsError::DeleteSubject(e.into()),
            SubjectsUpdateError::MoveUp(e) => SubjectsError::MoveUp(e.into()),
            SubjectsUpdateError::MoveDown(e) => SubjectsError::MoveDown(e.into()),
            SubjectsUpdateError::UpdatePeriodStatus(e) => {
                SubjectsError::UpdatePeriodStatus(e.into())
            }
        }
    }
}

impl From<AddNewSubjectError> for SubjectsError {
    fn from(value: AddNewSubjectError) -> Self {
        SubjectsError::AddNewSubject(value)
    }
}

impl From<UpdateSubjectError> for SubjectsError {
    fn from(value: UpdateSubjectError) -> Self {
        SubjectsError::UpdateSubject(value)
    }
}

impl From<DeleteSubjectError> for SubjectsError {
    fn from(value: DeleteSubjectError) -> Self {
        SubjectsError::DeleteSubject(value)
    }
}

impl From<MoveUpError> for SubjectsError {
    fn from(value: MoveUpError) -> Self {
        SubjectsError::MoveUp(value)
    }
}

impl From<MoveDownError> for SubjectsError {
    fn from(value: MoveDownError) -> Self {
        SubjectsError::MoveDown(value)
    }
}

impl From<UpdatePeriodStatusError> for SubjectsError {
    fn from(value: UpdatePeriodStatusError) -> Self {
        SubjectsError::UpdatePeriodStatus(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddNewSubjectError {
    StudentsPerGroupRangeIsEmpty,
    GroupsPerInterrogationRangeIsEmpty,
    InterrogationCountRangeIsEmpty,
}

impl std::fmt::Display for AddNewSubjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddNewSubjectError::StudentsPerGroupRangeIsEmpty => {
                write!(
                    f,
                    "Aucune valeur autorisée pour le nombre d'élèves par groupe"
                )
            }
            AddNewSubjectError::GroupsPerInterrogationRangeIsEmpty => {
                write!(
                    f,
                    "Aucune valeur autorisée pour le nombre de groupes par colle"
                )
            }
            AddNewSubjectError::InterrogationCountRangeIsEmpty => {
                write!(f, "Aucune valeur autorisée pour le nombre de colles")
            }
        }
    }
}

impl From<crate::ops::AddNewSubjectError> for AddNewSubjectError {
    fn from(value: crate::ops::AddNewSubjectError) -> Self {
        match value {
            crate::ops::AddNewSubjectError::StudentsPerGroupRangeIsEmpty => {
                AddNewSubjectError::StudentsPerGroupRangeIsEmpty
            }
            crate::ops::AddNewSubjectError::GroupsPerInterrogationRangeIsEmpty => {
                AddNewSubjectError::GroupsPerInterrogationRangeIsEmpty
            }
            crate::ops::AddNewSubjectError::InterrogationCountRangeIsEmpty => {
                AddNewSubjectError::InterrogationCountRangeIsEmpty
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateSubjectError {
    InvalidSubjectId(MsgSubjectId),
    StudentsPerGroupRangeIsEmpty,
    GroupsPerInterrogationRangeIsEmpty,
    InterrogationCountRangeIsEmpty,
}

impl std::fmt::Display for UpdateSubjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateSubjectError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            UpdateSubjectError::StudentsPerGroupRangeIsEmpty => {
                write!(
                    f,
                    "Aucune valeur autorisée pour le nombre d'élèves par groupe"
                )
            }
            UpdateSubjectError::GroupsPerInterrogationRangeIsEmpty => {
                write!(
                    f,
                    "Aucune valeur autorisée pour le nombre de groupes par colle"
                )
            }
            UpdateSubjectError::InterrogationCountRangeIsEmpty => {
                write!(f, "Aucune valeur autorisée pour le nombre de colles")
            }
        }
    }
}

impl From<crate::ops::UpdateSubjectError> for UpdateSubjectError {
    fn from(value: crate::ops::UpdateSubjectError) -> Self {
        match value {
            crate::ops::UpdateSubjectError::InvalidSubjectId(id) => {
                UpdateSubjectError::InvalidSubjectId(id.into())
            }
            crate::ops::UpdateSubjectError::StudentsPerGroupRangeIsEmpty => {
                UpdateSubjectError::StudentsPerGroupRangeIsEmpty
            }
            crate::ops::UpdateSubjectError::GroupsPerInterrogationRangeIsEmpty => {
                UpdateSubjectError::GroupsPerInterrogationRangeIsEmpty
            }
            crate::ops::UpdateSubjectError::InterrogationCountRangeIsEmpty => {
                UpdateSubjectError::InterrogationCountRangeIsEmpty
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteSubjectError {
    InvalidSubjectId(MsgSubjectId),
}

impl std::fmt::Display for DeleteSubjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteSubjectError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
        }
    }
}

impl From<crate::ops::DeleteSubjectError> for DeleteSubjectError {
    fn from(value: crate::ops::DeleteSubjectError) -> Self {
        match value {
            crate::ops::DeleteSubjectError::InvalidSubjectId(id) => {
                DeleteSubjectError::InvalidSubjectId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveUpError {
    InvalidSubjectId(MsgSubjectId),
    NoUpperPosition,
}

impl std::fmt::Display for MoveUpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveUpError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            MoveUpError::NoUpperPosition => {
                write!(
                    f,
                    "Impossible de remonter la matière (c'est déjà la première)"
                )
            }
        }
    }
}

impl From<crate::ops::MoveUpError> for MoveUpError {
    fn from(value: crate::ops::MoveUpError) -> Self {
        match value {
            crate::ops::MoveUpError::InvalidSubjectId(id) => {
                MoveUpError::InvalidSubjectId(id.into())
            }
            crate::ops::MoveUpError::NoUpperPosition => MoveUpError::NoUpperPosition,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveDownError {
    InvalidSubjectId(MsgSubjectId),
    NoLowerPosition,
}

impl std::fmt::Display for MoveDownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveDownError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            MoveDownError::NoLowerPosition => {
                write!(
                    f,
                    "Impossible de descendre la matière (c'est déjà la dernière)"
                )
            }
        }
    }
}

impl From<crate::ops::MoveDownError> for MoveDownError {
    fn from(value: crate::ops::MoveDownError) -> Self {
        match value {
            crate::ops::MoveDownError::InvalidSubjectId(id) => {
                MoveDownError::InvalidSubjectId(id.into())
            }
            crate::ops::MoveDownError::NoLowerPosition => MoveDownError::NoLowerPosition,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdatePeriodStatusError {
    InvalidSubjectId(MsgSubjectId),
    InvalidPeriodId(MsgPeriodId),
}

impl std::fmt::Display for UpdatePeriodStatusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdatePeriodStatusError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            UpdatePeriodStatusError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
        }
    }
}

impl From<crate::ops::UpdatePeriodStatusError> for UpdatePeriodStatusError {
    fn from(value: crate::ops::UpdatePeriodStatusError) -> Self {
        match value {
            crate::ops::UpdatePeriodStatusError::InvalidSubjectId(id) => {
                UpdatePeriodStatusError::InvalidSubjectId(id.into())
            }
            crate::ops::UpdatePeriodStatusError::InvalidPeriodId(id) => {
                UpdatePeriodStatusError::InvalidPeriodId(id.into())
            }
        }
    }
}
