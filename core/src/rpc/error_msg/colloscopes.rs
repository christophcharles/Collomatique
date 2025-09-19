use crate::rpc::cmd_msg::MsgColloscopeId;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColloscopesError {
    AddEmptyColloscope(AddEmptyColloscopeError),
    CopyColloscope(CopyColloscopeError),
    UpdateColloscope(UpdateColloscopeError),
    DeleteColloscope(DeleteColloscopeError),
}

impl std::fmt::Display for ColloscopesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColloscopesError::AddEmptyColloscope(e) => e.fmt(f),
            ColloscopesError::CopyColloscope(e) => e.fmt(f),
            ColloscopesError::UpdateColloscope(e) => e.fmt(f),
            ColloscopesError::DeleteColloscope(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::ColloscopesUpdateError> for ColloscopesError {
    fn from(value: crate::ops::ColloscopesUpdateError) -> Self {
        use crate::ops::ColloscopesUpdateError;
        match value {
            ColloscopesUpdateError::AddEmptyColloscope(e) => {
                ColloscopesError::AddEmptyColloscope(e.into())
            }
            ColloscopesUpdateError::CopyColloscope(e) => ColloscopesError::CopyColloscope(e.into()),
            ColloscopesUpdateError::UpdateColloscope(e) => {
                ColloscopesError::UpdateColloscope(e.into())
            }
            ColloscopesUpdateError::DeleteColloscope(e) => {
                ColloscopesError::DeleteColloscope(e.into())
            }
        }
    }
}

impl From<AddEmptyColloscopeError> for ColloscopesError {
    fn from(value: AddEmptyColloscopeError) -> Self {
        ColloscopesError::AddEmptyColloscope(value)
    }
}

impl From<CopyColloscopeError> for ColloscopesError {
    fn from(value: CopyColloscopeError) -> Self {
        ColloscopesError::CopyColloscope(value)
    }
}

impl From<UpdateColloscopeError> for ColloscopesError {
    fn from(value: UpdateColloscopeError) -> Self {
        ColloscopesError::UpdateColloscope(value)
    }
}

impl From<DeleteColloscopeError> for ColloscopesError {
    fn from(value: DeleteColloscopeError) -> Self {
        ColloscopesError::DeleteColloscope(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddEmptyColloscopeError {}

impl std::fmt::Display for AddEmptyColloscopeError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl From<crate::ops::AddEmptyColloscopeError> for AddEmptyColloscopeError {
    fn from(value: crate::ops::AddEmptyColloscopeError) -> Self {
        match value {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CopyColloscopeError {
    InvalidColloscopeId(MsgColloscopeId),
}

impl std::fmt::Display for CopyColloscopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CopyColloscopeError::InvalidColloscopeId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun colloscope", id.0)
            }
        }
    }
}

impl From<crate::ops::CopyColloscopeError> for CopyColloscopeError {
    fn from(value: crate::ops::CopyColloscopeError) -> Self {
        match value {
            crate::ops::CopyColloscopeError::InvalidColloscopeId(id) => {
                CopyColloscopeError::InvalidColloscopeId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateColloscopeError {
    InvalidColloscopeId(MsgColloscopeId),
    BadInvariantInColloscope,
}

impl std::fmt::Display for UpdateColloscopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateColloscopeError::InvalidColloscopeId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune règle", id.0)
            }
            UpdateColloscopeError::BadInvariantInColloscope => {
                write!(f, "Les paramètres associés au colloscope sont invalides")
            }
        }
    }
}

impl From<crate::ops::UpdateColloscopeError> for UpdateColloscopeError {
    fn from(value: crate::ops::UpdateColloscopeError) -> Self {
        match value {
            crate::ops::UpdateColloscopeError::InvalidColloscopeId(id) => {
                UpdateColloscopeError::InvalidColloscopeId(id.into())
            }
            crate::ops::UpdateColloscopeError::BadInvariantInColloscope(_) => {
                UpdateColloscopeError::BadInvariantInColloscope
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteColloscopeError {
    InvalidColloscopeId(MsgColloscopeId),
}

impl std::fmt::Display for DeleteColloscopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteColloscopeError::InvalidColloscopeId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun colloscope", id.0)
            }
        }
    }
}

impl From<crate::ops::DeleteColloscopeError> for DeleteColloscopeError {
    fn from(value: crate::ops::DeleteColloscopeError) -> Self {
        match value {
            crate::ops::DeleteColloscopeError::InvalidColloscopeId(id) => {
                DeleteColloscopeError::InvalidColloscopeId(id.into())
            }
        }
    }
}
