use crate::rpc::cmd_msg::{MsgPeriodId, MsgRuleId, MsgSlotId};

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RulesError {
    AddNewRule(AddNewRuleError),
    UpdateRule(UpdateRuleError),
    DeleteRule(DeleteRuleError),
    UpdatePeriodStatusForRule(UpdatePeriodStatusForRuleError),
}

impl std::fmt::Display for RulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RulesError::AddNewRule(e) => e.fmt(f),
            RulesError::UpdateRule(e) => e.fmt(f),
            RulesError::DeleteRule(e) => e.fmt(f),
            RulesError::UpdatePeriodStatusForRule(e) => e.fmt(f),
        }
    }
}

impl From<crate::ops::RulesUpdateError> for RulesError {
    fn from(value: crate::ops::RulesUpdateError) -> Self {
        use crate::ops::RulesUpdateError;
        match value {
            RulesUpdateError::AddNewRule(e) => RulesError::AddNewRule(e.into()),
            RulesUpdateError::UpdateRule(e) => RulesError::UpdateRule(e.into()),
            RulesUpdateError::DeleteRule(e) => RulesError::DeleteRule(e.into()),
            RulesUpdateError::UpdatePeriodStatusForRule(e) => {
                RulesError::UpdatePeriodStatusForRule(e.into())
            }
        }
    }
}

impl From<AddNewRuleError> for RulesError {
    fn from(value: AddNewRuleError) -> Self {
        RulesError::AddNewRule(value)
    }
}

impl From<UpdateRuleError> for RulesError {
    fn from(value: UpdateRuleError) -> Self {
        RulesError::UpdateRule(value)
    }
}

impl From<DeleteRuleError> for RulesError {
    fn from(value: DeleteRuleError) -> Self {
        RulesError::DeleteRule(value)
    }
}

impl From<UpdatePeriodStatusForRuleError> for RulesError {
    fn from(value: UpdatePeriodStatusForRuleError) -> Self {
        RulesError::UpdatePeriodStatusForRule(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddNewRuleError {
    InvalidSlotId(MsgSlotId),
}

impl std::fmt::Display for AddNewRuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AddNewRuleError::InvalidSlotId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun créneau", id.0)
            }
        }
    }
}

impl From<crate::ops::AddNewRuleError> for AddNewRuleError {
    fn from(value: crate::ops::AddNewRuleError) -> Self {
        match value {
            crate::ops::AddNewRuleError::InvalidSlotId(id) => {
                AddNewRuleError::InvalidSlotId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateRuleError {
    InvalidRuleId(MsgRuleId),
    InvalidSlotId(MsgSlotId),
}

impl std::fmt::Display for UpdateRuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateRuleError::InvalidRuleId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune règle", id.0)
            }
            UpdateRuleError::InvalidSlotId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun créneau", id.0)
            }
        }
    }
}

impl From<crate::ops::UpdateRuleError> for UpdateRuleError {
    fn from(value: crate::ops::UpdateRuleError) -> Self {
        match value {
            crate::ops::UpdateRuleError::InvalidRuleId(id) => {
                UpdateRuleError::InvalidRuleId(id.into())
            }
            crate::ops::UpdateRuleError::InvalidSlotId(id) => {
                UpdateRuleError::InvalidSlotId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeleteRuleError {
    InvalidRuleId(MsgRuleId),
}

impl std::fmt::Display for DeleteRuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeleteRuleError::InvalidRuleId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune règle", id.0)
            }
        }
    }
}

impl From<crate::ops::DeleteRuleError> for DeleteRuleError {
    fn from(value: crate::ops::DeleteRuleError) -> Self {
        match value {
            crate::ops::DeleteRuleError::InvalidRuleId(id) => {
                DeleteRuleError::InvalidRuleId(id.into())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdatePeriodStatusForRuleError {
    InvalidRuleId(MsgRuleId),
    InvalidPeriodId(MsgPeriodId),
}

impl std::fmt::Display for UpdatePeriodStatusForRuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdatePeriodStatusForRuleError::InvalidRuleId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune règle", id.0)
            }
            UpdatePeriodStatusForRuleError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
        }
    }
}

impl From<crate::ops::UpdatePeriodStatusForRuleError> for UpdatePeriodStatusForRuleError {
    fn from(value: crate::ops::UpdatePeriodStatusForRuleError) -> Self {
        match value {
            crate::ops::UpdatePeriodStatusForRuleError::InvalidRuleId(id) => {
                UpdatePeriodStatusForRuleError::InvalidRuleId(id.into())
            }
            crate::ops::UpdatePeriodStatusForRuleError::InvalidPeriodId(id) => {
                UpdatePeriodStatusForRuleError::InvalidPeriodId(id.into())
            }
        }
    }
}
