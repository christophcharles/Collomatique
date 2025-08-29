use crate::rpc::cmd_msg::{MsgGroupListId, MsgStudentId, MsgSubjectId};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupListsError {
    AddNewGroupList(AddNewGroupListError),
    UpdateGroupList(UpdateGroupListError),
    DeleteGroupList(DeleteGroupListError),
    PrefillGroupList(PrefillGroupListError),
    AssignGroupListToSubject(AssignGroupListToSubjectError),
}

impl std::fmt::Display for GroupListsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroupListsError::AddNewGroupList(e) => e.fmt(f),
            GroupListsError::UpdateGroupList(e) => e.fmt(f),
            GroupListsError::DeleteGroupList(e) => e.fmt(f),
            GroupListsError::PrefillGroupList(e) => e.fmt(f),
            GroupListsError::AssignGroupListToSubject(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::GroupListsUpdateError> for GroupListsError {
    fn from(value: crate::ops::GroupListsUpdateError) -> Self {
        use crate::ops::GroupListsUpdateError;
        match value {
            GroupListsUpdateError::AddNewGroupList(e) => GroupListsError::AddNewGroupList(e.into()),
            GroupListsUpdateError::UpdateGroupList(e) => GroupListsError::UpdateGroupList(e.into()),
            GroupListsUpdateError::DeleteGroupList(e) => GroupListsError::DeleteGroupList(e.into()),
            GroupListsUpdateError::PrefillGroupList(e) => {
                GroupListsError::PrefillGroupList(e.into())
            }
            GroupListsUpdateError::AssignGroupListToSubject(e) => {
                GroupListsError::AssignGroupListToSubject(e.into())
            }
        }
    }
}

impl From<AddNewGroupListError> for GroupListsError {
    fn from(value: AddNewGroupListError) -> Self {
        GroupListsError::AddNewGroupList(value)
    }
}

impl From<UpdateGroupListError> for GroupListsError {
    fn from(value: UpdateGroupListError) -> Self {
        GroupListsError::UpdateGroupList(value)
    }
}

impl From<DeleteGroupListError> for GroupListsError {
    fn from(value: DeleteGroupListError) -> Self {
        GroupListsError::DeleteGroupList(value)
    }
}

impl From<PrefillGroupListError> for GroupListsError {
    fn from(value: PrefillGroupListError) -> Self {
        GroupListsError::PrefillGroupList(value)
    }
}

impl From<AssignGroupListToSubjectError> for GroupListsError {
    fn from(value: AssignGroupListToSubjectError) -> Self {
        GroupListsError::AssignGroupListToSubject(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddNewGroupListError {
    InvalidStudentId(MsgStudentId),
    GroupCountRangeIsEmpty,
    StudentsPerGroupRangeIsEmpty,
}

impl std::fmt::Display for AddNewGroupListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddNewGroupListError::InvalidStudentId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun élève", id.0)
            }
            AddNewGroupListError::GroupCountRangeIsEmpty => {
                write!(
                    f,
                    "L'éventail de nombres de groupes autorisés doit être non vide"
                )
            }
            AddNewGroupListError::StudentsPerGroupRangeIsEmpty => {
                write!(
                    f,
                    "L'éventail de nombres d'élèves par groupe autorisés doit être non vide"
                )
            }
        }
    }
}

impl From<crate::ops::AddNewGroupListError> for AddNewGroupListError {
    fn from(value: crate::ops::AddNewGroupListError) -> Self {
        match value {
            crate::ops::AddNewGroupListError::InvalidStudentId(id) => {
                AddNewGroupListError::InvalidStudentId(id.into())
            }
            crate::ops::AddNewGroupListError::GroupCountRangeIsEmpty => {
                AddNewGroupListError::GroupCountRangeIsEmpty
            }
            crate::ops::AddNewGroupListError::StudentsPerGroupRangeIsEmpty => {
                AddNewGroupListError::StudentsPerGroupRangeIsEmpty
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateGroupListError {
    InvalidGroupListId(MsgGroupListId),
    InvalidStudentId(MsgStudentId),
    GroupCountRangeIsEmpty,
    StudentsPerGroupRangeIsEmpty,
}

impl std::fmt::Display for UpdateGroupListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateGroupListError::InvalidGroupListId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune liste de groupes",
                    id.0
                )
            }
            UpdateGroupListError::InvalidStudentId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun élève", id.0)
            }
            UpdateGroupListError::GroupCountRangeIsEmpty => {
                write!(
                    f,
                    "L'éventail de nombres de groupes autorisés doit être non vide"
                )
            }
            UpdateGroupListError::StudentsPerGroupRangeIsEmpty => {
                write!(
                    f,
                    "L'éventail de nombres d'élèves par groupe autorisés doit être non vide"
                )
            }
        }
    }
}

impl From<crate::ops::UpdateGroupListError> for UpdateGroupListError {
    fn from(value: crate::ops::UpdateGroupListError) -> Self {
        match value {
            crate::ops::UpdateGroupListError::InvalidGroupListId(id) => {
                UpdateGroupListError::InvalidGroupListId(id.into())
            }
            crate::ops::UpdateGroupListError::InvalidStudentId(id) => {
                UpdateGroupListError::InvalidStudentId(id.into())
            }
            crate::ops::UpdateGroupListError::GroupCountRangeIsEmpty => {
                UpdateGroupListError::GroupCountRangeIsEmpty
            }
            crate::ops::UpdateGroupListError::StudentsPerGroupRangeIsEmpty => {
                UpdateGroupListError::StudentsPerGroupRangeIsEmpty
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteGroupListError {
    InvalidGroupListId(MsgGroupListId),
}

impl std::fmt::Display for DeleteGroupListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteGroupListError::InvalidGroupListId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune liste de groupes",
                    id.0
                )
            }
        }
    }
}

impl From<crate::ops::DeleteGroupListError> for DeleteGroupListError {
    fn from(value: crate::ops::DeleteGroupListError) -> Self {
        match value {
            crate::ops::DeleteGroupListError::InvalidGroupListId(id) => {
                DeleteGroupListError::InvalidGroupListId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrefillGroupListError {
    InvalidGroupListId(MsgGroupListId),
    InvalidStudentId(MsgStudentId),
    StudentIsExcluded(MsgGroupListId, MsgStudentId),
}

impl std::fmt::Display for PrefillGroupListError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrefillGroupListError::InvalidGroupListId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune liste de groupes",
                    id.0
                )
            }
            PrefillGroupListError::InvalidStudentId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun élève", id.0)
            }
            PrefillGroupListError::StudentIsExcluded(group_list_id, student_id) => {
                write!(
                    f,
                    "L'élève {} ne peut apparaître dans le préremplissage : il est exclu de la liste {}",
                    student_id.0,
                    group_list_id.0,
                )
            }
        }
    }
}

impl From<crate::ops::PrefillGroupListError> for PrefillGroupListError {
    fn from(value: crate::ops::PrefillGroupListError) -> Self {
        match value {
            crate::ops::PrefillGroupListError::InvalidGroupListId(id) => {
                PrefillGroupListError::InvalidGroupListId(id.into())
            }
            crate::ops::PrefillGroupListError::InvalidStudentId(id) => {
                PrefillGroupListError::InvalidStudentId(id.into())
            }
            crate::ops::PrefillGroupListError::StudentIsExcluded(group_list_id, student_id) => {
                PrefillGroupListError::StudentIsExcluded(group_list_id.into(), student_id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssignGroupListToSubjectError {
    InvalidGroupListId(MsgGroupListId),
    InvalidSubjectId(MsgSubjectId),
    SubjectHasNoInterrogation(MsgSubjectId),
}

impl std::fmt::Display for AssignGroupListToSubjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssignGroupListToSubjectError::InvalidGroupListId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune liste de groupes",
                    id.0
                )
            }
            AssignGroupListToSubjectError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            AssignGroupListToSubjectError::SubjectHasNoInterrogation(id) => {
                write!(
                    f,
                    "La matière {} n'a pas de colles et ne peut donc être associée à une liste de groupes",
                    id.0
                )
            }
        }
    }
}

impl From<crate::ops::AssignGroupListToSubjectError> for AssignGroupListToSubjectError {
    fn from(value: crate::ops::AssignGroupListToSubjectError) -> Self {
        match value {
            crate::ops::AssignGroupListToSubjectError::InvalidGroupListId(id) => {
                AssignGroupListToSubjectError::InvalidGroupListId(id.into())
            }
            crate::ops::AssignGroupListToSubjectError::InvalidSubjectId(id) => {
                AssignGroupListToSubjectError::InvalidSubjectId(id.into())
            }
            crate::ops::AssignGroupListToSubjectError::SubjectHasNoInterrogation(id) => {
                AssignGroupListToSubjectError::SubjectHasNoInterrogation(id.into())
            }
        }
    }
}
