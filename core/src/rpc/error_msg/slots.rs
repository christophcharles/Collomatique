use crate::rpc::cmd_msg::{MsgSlotId, MsgSubjectId, MsgTeacherId, MsgWeekPatternId};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotsError {
    AddNewSlot(AddNewSlotError),
    UpdateSlot(UpdateSlotError),
    DeleteSlot(DeleteSlotError),
    MoveSlotUp(MoveSlotUpError),
    MoveSlotDown(MoveSlotDownError),
}

impl std::fmt::Display for SlotsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SlotsError::AddNewSlot(e) => e.fmt(f),
            SlotsError::UpdateSlot(e) => e.fmt(f),
            SlotsError::DeleteSlot(e) => e.fmt(f),
            SlotsError::MoveSlotUp(e) => e.fmt(f),
            SlotsError::MoveSlotDown(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::SlotsUpdateError> for SlotsError {
    fn from(value: crate::ops::SlotsUpdateError) -> Self {
        use crate::ops::SlotsUpdateError;
        match value {
            SlotsUpdateError::AddNewSlot(e) => SlotsError::AddNewSlot(e.into()),
            SlotsUpdateError::UpdateSlot(e) => SlotsError::UpdateSlot(e.into()),
            SlotsUpdateError::DeleteSlot(e) => SlotsError::DeleteSlot(e.into()),
            SlotsUpdateError::MoveSlotUp(e) => SlotsError::MoveSlotUp(e.into()),
            SlotsUpdateError::MoveSlotDown(e) => SlotsError::MoveSlotDown(e.into()),
        }
    }
}

impl From<AddNewSlotError> for SlotsError {
    fn from(value: AddNewSlotError) -> Self {
        SlotsError::AddNewSlot(value)
    }
}

impl From<UpdateSlotError> for SlotsError {
    fn from(value: UpdateSlotError) -> Self {
        SlotsError::UpdateSlot(value)
    }
}

impl From<DeleteSlotError> for SlotsError {
    fn from(value: DeleteSlotError) -> Self {
        SlotsError::DeleteSlot(value)
    }
}

impl From<MoveSlotUpError> for SlotsError {
    fn from(value: MoveSlotUpError) -> Self {
        SlotsError::MoveSlotUp(value)
    }
}

impl From<MoveSlotDownError> for SlotsError {
    fn from(value: MoveSlotDownError) -> Self {
        SlotsError::MoveSlotDown(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddNewSlotError {
    InvalidSubjectId(MsgSubjectId),
    SubjectHasNoInterrogation(MsgSubjectId),
    InvalidTeacherId(MsgTeacherId),
    InvalidWeekPatternId(MsgWeekPatternId),
    TeacherDoesNotTeachInSubject(MsgTeacherId, MsgSubjectId),
    SlotOverlapsWithNextDay,
}

impl std::fmt::Display for AddNewSlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddNewSlotError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            AddNewSlotError::SubjectHasNoInterrogation(id) => {
                write!(f, "La matière {} ne donne pas lieu à des colles", id.0)
            }
            AddNewSlotError::InvalidTeacherId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun colleur", id.0)
            }
            AddNewSlotError::InvalidWeekPatternId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun modèle de périodicité",
                    id.0
                )
            }
            AddNewSlotError::TeacherDoesNotTeachInSubject(tid, sid) => {
                write!(
                    f,
                    "L'enseignant {} ne colle pas dans la matière {}",
                    tid.0, sid.0
                )
            }
            AddNewSlotError::SlotOverlapsWithNextDay => {
                write!(f, "Le créneau est à cheval sur deux journées")
            }
        }
    }
}

impl From<crate::ops::AddNewSlotError> for AddNewSlotError {
    fn from(value: crate::ops::AddNewSlotError) -> Self {
        match value {
            crate::ops::AddNewSlotError::InvalidSubjectId(id) => {
                AddNewSlotError::InvalidSubjectId(id.into())
            }
            crate::ops::AddNewSlotError::SubjectHasNoInterrogation(id) => {
                AddNewSlotError::SubjectHasNoInterrogation(id.into())
            }
            crate::ops::AddNewSlotError::InvalidTeacherId(id) => {
                AddNewSlotError::InvalidTeacherId(id.into())
            }
            crate::ops::AddNewSlotError::InvalidWeekPatternId(id) => {
                AddNewSlotError::InvalidWeekPatternId(id.into())
            }
            crate::ops::AddNewSlotError::TeacherDoesNotTeachInSubject(tid, sid) => {
                AddNewSlotError::TeacherDoesNotTeachInSubject(tid.into(), sid.into())
            }
            crate::ops::AddNewSlotError::SlotOverlapsWithNextDay => {
                AddNewSlotError::SlotOverlapsWithNextDay
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateSlotError {
    InvalidSlotId(MsgSlotId),
    InvalidSubjectId(MsgSubjectId),
    SubjectHasNoInterrogation(MsgSubjectId),
    InvalidTeacherId(MsgTeacherId),
    InvalidWeekPatternId(MsgWeekPatternId),
    TeacherDoesNotTeachInSubject(MsgTeacherId, MsgSubjectId),
    SlotOverlapsWithNextDay,
}

impl std::fmt::Display for UpdateSlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateSlotError::InvalidSlotId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun créneau de colle",
                    id.0
                )
            }
            UpdateSlotError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            UpdateSlotError::SubjectHasNoInterrogation(id) => {
                write!(f, "La matière {} ne donne pas lieu à des colles", id.0)
            }
            UpdateSlotError::InvalidTeacherId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun colleur", id.0)
            }
            UpdateSlotError::InvalidWeekPatternId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun modèle de périodicité",
                    id.0
                )
            }
            UpdateSlotError::TeacherDoesNotTeachInSubject(tid, sid) => {
                write!(
                    f,
                    "L'enseignant {} ne colle pas dans la matière {}",
                    tid.0, sid.0
                )
            }
            UpdateSlotError::SlotOverlapsWithNextDay => {
                write!(f, "Le créneau est à cheval sur deux journées")
            }
        }
    }
}

impl From<crate::ops::UpdateSlotError> for UpdateSlotError {
    fn from(value: crate::ops::UpdateSlotError) -> Self {
        match value {
            crate::ops::UpdateSlotError::InvalidSlotId(id) => {
                UpdateSlotError::InvalidSlotId(id.into())
            }
            crate::ops::UpdateSlotError::InvalidSubjectId(id) => {
                UpdateSlotError::InvalidSubjectId(id.into())
            }
            crate::ops::UpdateSlotError::SubjectHasNoInterrogation(id) => {
                UpdateSlotError::SubjectHasNoInterrogation(id.into())
            }
            crate::ops::UpdateSlotError::InvalidTeacherId(id) => {
                UpdateSlotError::InvalidTeacherId(id.into())
            }
            crate::ops::UpdateSlotError::InvalidWeekPatternId(id) => {
                UpdateSlotError::InvalidWeekPatternId(id.into())
            }
            crate::ops::UpdateSlotError::TeacherDoesNotTeachInSubject(tid, sid) => {
                UpdateSlotError::TeacherDoesNotTeachInSubject(tid.into(), sid.into())
            }
            crate::ops::UpdateSlotError::SlotOverlapsWithNextDay => {
                UpdateSlotError::SlotOverlapsWithNextDay
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteSlotError {
    InvalidSlotId(MsgSlotId),
}

impl std::fmt::Display for DeleteSlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteSlotError::InvalidSlotId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun créneau de colle",
                    id.0
                )
            }
        }
    }
}

impl From<crate::ops::DeleteSlotError> for DeleteSlotError {
    fn from(value: crate::ops::DeleteSlotError) -> Self {
        match value {
            crate::ops::DeleteSlotError::InvalidSlotId(id) => {
                DeleteSlotError::InvalidSlotId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveSlotUpError {
    InvalidSlotId(MsgSlotId),
    NoUpperPosition,
}

impl std::fmt::Display for MoveSlotUpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveSlotUpError::InvalidSlotId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun créneau de colle",
                    id.0
                )
            }
            MoveSlotUpError::NoUpperPosition => {
                write!(
                    f,
                    "Impossible de remonter le créneau de colle (c'est déjà le premier)"
                )
            }
        }
    }
}

impl From<crate::ops::MoveSlotUpError> for MoveSlotUpError {
    fn from(value: crate::ops::MoveSlotUpError) -> Self {
        match value {
            crate::ops::MoveSlotUpError::InvalidSlotId(id) => {
                MoveSlotUpError::InvalidSlotId(id.into())
            }
            crate::ops::MoveSlotUpError::NoUpperPosition => MoveSlotUpError::NoUpperPosition,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveSlotDownError {
    InvalidSlotId(MsgSlotId),
    NoLowerPosition,
}

impl std::fmt::Display for MoveSlotDownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveSlotDownError::InvalidSlotId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun créneau de colle",
                    id.0
                )
            }
            MoveSlotDownError::NoLowerPosition => {
                write!(
                    f,
                    "Impossible de descendre le creneau de colle (c'est déjà le dernier)"
                )
            }
        }
    }
}

impl From<crate::ops::MoveSlotDownError> for MoveSlotDownError {
    fn from(value: crate::ops::MoveSlotDownError) -> Self {
        match value {
            crate::ops::MoveSlotDownError::InvalidSlotId(id) => {
                MoveSlotDownError::InvalidSlotId(id.into())
            }
            crate::ops::MoveSlotDownError::NoLowerPosition => MoveSlotDownError::NoLowerPosition,
        }
    }
}
