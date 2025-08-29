use crate::rpc::cmd_msg::{MsgIncompatId, MsgSubjectId, MsgWeekPatternId};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncompatibilitiesError {
    AddNewIncompat(AddNewIncompatError),
    UpdateIncompat(UpdateIncompatError),
    DeleteIncompat(DeleteIncompatError),
}

impl std::fmt::Display for IncompatibilitiesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncompatibilitiesError::AddNewIncompat(e) => e.fmt(f),
            IncompatibilitiesError::UpdateIncompat(e) => e.fmt(f),
            IncompatibilitiesError::DeleteIncompat(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::IncompatibilitiesUpdateError> for IncompatibilitiesError {
    fn from(value: crate::ops::IncompatibilitiesUpdateError) -> Self {
        use crate::ops::IncompatibilitiesUpdateError;
        match value {
            IncompatibilitiesUpdateError::AddNewIncompat(e) => {
                IncompatibilitiesError::AddNewIncompat(e.into())
            }
            IncompatibilitiesUpdateError::UpdateIncompat(e) => {
                IncompatibilitiesError::UpdateIncompat(e.into())
            }
            IncompatibilitiesUpdateError::DeleteIncompat(e) => {
                IncompatibilitiesError::DeleteIncompat(e.into())
            }
        }
    }
}

impl From<AddNewIncompatError> for IncompatibilitiesError {
    fn from(value: AddNewIncompatError) -> Self {
        IncompatibilitiesError::AddNewIncompat(value)
    }
}

impl From<UpdateIncompatError> for IncompatibilitiesError {
    fn from(value: UpdateIncompatError) -> Self {
        IncompatibilitiesError::UpdateIncompat(value)
    }
}

impl From<DeleteIncompatError> for IncompatibilitiesError {
    fn from(value: DeleteIncompatError) -> Self {
        IncompatibilitiesError::DeleteIncompat(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddNewIncompatError {
    InvalidSubjectId(MsgSubjectId),
    InvalidWeekPatternId(MsgWeekPatternId),
    SlotOverlapsWithNextDay,
}

impl std::fmt::Display for AddNewIncompatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddNewIncompatError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            AddNewIncompatError::InvalidWeekPatternId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun modèle de périodicité",
                    id.0
                )
            }
            AddNewIncompatError::SlotOverlapsWithNextDay => {
                write!(
                    f,
                    "Le créneau d'incompatibilité dépasse sur le jour suivant",
                )
            }
        }
    }
}

impl From<crate::ops::AddNewIncompatError> for AddNewIncompatError {
    fn from(value: crate::ops::AddNewIncompatError) -> Self {
        match value {
            crate::ops::AddNewIncompatError::InvalidSubjectId(id) => {
                AddNewIncompatError::InvalidSubjectId(id.into())
            }
            crate::ops::AddNewIncompatError::InvalidWeekPatternId(id) => {
                AddNewIncompatError::InvalidWeekPatternId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateIncompatError {
    InvalidIncompatId(MsgIncompatId),
    InvalidSubjectId(MsgSubjectId),
    InvalidWeekPatternId(MsgWeekPatternId),
    SlotOverlapsWithNextDay,
}

impl std::fmt::Display for UpdateIncompatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateIncompatError::InvalidIncompatId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun incompatibilité horaire",
                    id.0
                )
            }
            UpdateIncompatError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            UpdateIncompatError::InvalidWeekPatternId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun modèle de périodicité",
                    id.0
                )
            }
            UpdateIncompatError::SlotOverlapsWithNextDay => {
                write!(
                    f,
                    "Le créneau d'incompatibilité dépasse sur le jour suivant",
                )
            }
        }
    }
}

impl From<crate::ops::UpdateIncompatError> for UpdateIncompatError {
    fn from(value: crate::ops::UpdateIncompatError) -> Self {
        match value {
            crate::ops::UpdateIncompatError::InvalidIncompatId(id) => {
                UpdateIncompatError::InvalidIncompatId(id.into())
            }
            crate::ops::UpdateIncompatError::InvalidSubjectId(id) => {
                UpdateIncompatError::InvalidSubjectId(id.into())
            }
            crate::ops::UpdateIncompatError::InvalidWeekPatternId(id) => {
                UpdateIncompatError::InvalidWeekPatternId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteIncompatError {
    InvalidIncompatId(MsgIncompatId),
}

impl std::fmt::Display for DeleteIncompatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteIncompatError::InvalidIncompatId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune incompatibilité horaire",
                    id.0
                )
            }
        }
    }
}

impl From<crate::ops::DeleteIncompatError> for DeleteIncompatError {
    fn from(value: crate::ops::DeleteIncompatError) -> Self {
        match value {
            crate::ops::DeleteIncompatError::InvalidIncompatId(id) => {
                DeleteIncompatError::InvalidIncompatId(id.into())
            }
        }
    }
}
