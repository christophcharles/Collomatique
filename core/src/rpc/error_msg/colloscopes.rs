use crate::rpc::cmd_msg::{
    MsgColloscopeGroupListId, MsgColloscopeId, MsgColloscopeIncompatId, MsgColloscopePeriodId,
    MsgColloscopeRuleId, MsgColloscopeSlotId, MsgColloscopeStudentId, MsgColloscopeSubjectId,
    MsgColloscopeTeacherId, MsgColloscopeWeekPatternId, MsgGroupListId, MsgIncompatId, MsgPeriodId,
    MsgRuleId, MsgSlotId, MsgStudentId, MsgSubjectId, MsgTeacherId, MsgWeekPatternId,
};

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
    InvalidStudentId(MsgStudentId),
    InvalidPeriodId(MsgPeriodId),
    InvalidSubjectId(MsgSubjectId),
    InvalidTeacherId(MsgTeacherId),
    InvalidWeekPatternId(MsgWeekPatternId),
    InvalidSlotId(MsgSlotId),
    InvalidIncompatId(MsgIncompatId),
    InvalidGroupListId(MsgGroupListId),
    InvalidRuleId(MsgRuleId),
    InvalidColloscopeStudentId(MsgColloscopeStudentId),
    InvalidColloscopePeriodId(MsgColloscopePeriodId),
    InvalidColloscopeSubjectId(MsgColloscopeSubjectId),
    InvalidColloscopeTeacherId(MsgColloscopeTeacherId),
    InvalidColloscopeWeekPatternId(MsgColloscopeWeekPatternId),
    InvalidColloscopeSlotId(MsgColloscopeSlotId),
    InvalidColloscopeIncompatId(MsgColloscopeIncompatId),
    InvalidColloscopeGroupListId(MsgColloscopeGroupListId),
    InvalidColloscopeRuleId(MsgColloscopeRuleId),
    InvariantErrorInParameters,
    WrongPeriodCountInColloscopeData,
    WrongGroupListCountInColloscopeData,
    WrongSubjectCountInPeriodInColloscopeData(MsgColloscopePeriodId),
    WrongSlotCountForSubjectInPeriodInColloscopeData(MsgColloscopePeriodId, MsgColloscopeSubjectId),
    WrongInterrogationCountForSlotInPeriodInColloscopeData(
        MsgColloscopePeriodId,
        MsgColloscopeSlotId,
    ),
    InterrogationOnNonInterrogationWeek(MsgColloscopePeriodId, MsgColloscopeSlotId, usize),
    MissingInterrogationOnInterrogationWeek(MsgColloscopePeriodId, MsgColloscopeSlotId, usize),
    InvalidGroupNumInInterrogation(MsgColloscopePeriodId, MsgColloscopeSlotId, usize),
    ExcludedStudentInGroupList(MsgColloscopeGroupListId, MsgColloscopeStudentId),
    WrongStudentCountInGroupList(MsgColloscopeGroupListId),
    InvalidGroupNumForStudentInGroupList(MsgColloscopeGroupListId, MsgColloscopeStudentId),
}

impl std::fmt::Display for UpdateColloscopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateColloscopeError::InvalidColloscopeId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun colloscope", id.0)
            }
            UpdateColloscopeError::InvalidStudentId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun élève", id.0)
            }
            UpdateColloscopeError::InvalidPeriodId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune période", id.0)
            }
            UpdateColloscopeError::InvalidSubjectId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune matière", id.0)
            }
            UpdateColloscopeError::InvalidTeacherId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun colleur", id.0)
            }
            UpdateColloscopeError::InvalidWeekPatternId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun modèle de périodicité",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidSlotId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucun créneau", id.0)
            }
            UpdateColloscopeError::InvalidIncompatId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune incompatibilité horaire",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidGroupListId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune liste de groupe",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidRuleId(id) => {
                write!(f, "L'identifiant {} ne correspond à aucune règle", id.0)
            }
            UpdateColloscopeError::InvalidColloscopeStudentId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun élève du colloscope",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidColloscopePeriodId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune période du colloscope",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidColloscopeSubjectId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune matière du colloscope",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidColloscopeTeacherId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun colleur du colloscope",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidColloscopeWeekPatternId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun modèle de périodicité du colloscope",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidColloscopeSlotId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucun créneau du colloscope",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidColloscopeIncompatId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune incompatibilité horaire du colloscope",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidColloscopeGroupListId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune liste de groupe du colloscope",
                    id.0
                )
            }
            UpdateColloscopeError::InvalidColloscopeRuleId(id) => {
                write!(
                    f,
                    "L'identifiant {} ne correspond à aucune règle du colloscope",
                    id.0
                )
            }
            UpdateColloscopeError::InvariantErrorInParameters => {
                write!(f, "Les paramètres associés au colloscope sont invalides")
            }
            UpdateColloscopeError::WrongPeriodCountInColloscopeData => {
                write!(
                    f,
                    "Le nombre de périodes dans le colloscope ne correspond pas aux paramètres"
                )
            }
            UpdateColloscopeError::WrongGroupListCountInColloscopeData => {
                write!(f, "Le nombre de listes de groupes dans le colloscope ne correspond pas aux paramètres")
            }
            UpdateColloscopeError::WrongSubjectCountInPeriodInColloscopeData(period_id) => {
                write!(f, "Le nombre de matières pour la période {} dans le colloscope ne correspond pas aux paramètres", period_id.0)
            }
            UpdateColloscopeError::WrongSlotCountForSubjectInPeriodInColloscopeData(
                period_id,
                subject_id,
            ) => {
                write!(f, "Le nombre de créneaux pour la matière {} durant la période {} dans le colloscope ne correspond pas aux paramètres", subject_id.0, period_id.0)
            }
            UpdateColloscopeError::WrongInterrogationCountForSlotInPeriodInColloscopeData(
                period_id,
                slot_id,
            ) => {
                write!(f, "Le nombre de colles pour le créneau {} durant la période {} dans le colloscope ne correspond pas aux paramètres", slot_id.0, period_id.0)
            }
            UpdateColloscopeError::InterrogationOnNonInterrogationWeek(
                period_id,
                slot_id,
                week,
            ) => {
                write!(f, "Le colloscope a une interrogation sur le créneau {} pendant la période {} semaine {} alors que les paramètres n'en prévoient pas", slot_id.0, period_id.0, week)
            }
            UpdateColloscopeError::MissingInterrogationOnInterrogationWeek(
                period_id,
                slot_id,
                week,
            ) => {
                write!(f, "Le colloscope n'a pas d'interrogation sur le créneau {} pendant la période {} semaine {} alors que les paramètres en prévoient une", slot_id.0, period_id.0, week)
            }
            UpdateColloscopeError::InvalidGroupNumInInterrogation(
                period_id,
                slot_id,
                group_num,
            ) => {
                write!(
                    f,
                    "Le numéro de groupe {} est invalide pour le créneau {} pendant la période {}",
                    group_num, slot_id.0, period_id.0
                )
            }
            UpdateColloscopeError::ExcludedStudentInGroupList(group_list_id, student_id) => {
                write!(
                    f,
                    "L'élève {} est exclu de la liste de groupe {} et ne devrait pas y apparaître",
                    student_id.0, group_list_id.0
                )
            }
            UpdateColloscopeError::WrongStudentCountInGroupList(group_list_id) => {
                write!(f, "Le nombre d'élèves pour la liste de groupe {} dans le colloscope ne correspond pas aux paramètres", group_list_id.0)
            }
            UpdateColloscopeError::InvalidGroupNumForStudentInGroupList(
                group_list_id,
                student_id,
            ) => {
                write!(f, "Le numéro de groupe renseigné pour l'élève {} dans la liste de groupe {} est invalide", student_id.0, group_list_id.0)
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
            crate::ops::UpdateColloscopeError::InvalidStudentId(id) => {
                UpdateColloscopeError::InvalidStudentId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidPeriodId(id) => {
                UpdateColloscopeError::InvalidPeriodId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidSubjectId(id) => {
                UpdateColloscopeError::InvalidSubjectId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidTeacherId(id) => {
                UpdateColloscopeError::InvalidTeacherId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidWeekPatternId(id) => {
                UpdateColloscopeError::InvalidWeekPatternId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidSlotId(id) => {
                UpdateColloscopeError::InvalidSlotId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidIncompatId(id) => {
                UpdateColloscopeError::InvalidIncompatId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidGroupListId(id) => {
                UpdateColloscopeError::InvalidGroupListId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidRuleId(id) => {
                UpdateColloscopeError::InvalidRuleId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidColloscopeStudentId(id) => {
                UpdateColloscopeError::InvalidColloscopeStudentId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidColloscopePeriodId(id) => {
                UpdateColloscopeError::InvalidColloscopePeriodId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidColloscopeSubjectId(id) => {
                UpdateColloscopeError::InvalidColloscopeSubjectId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidColloscopeTeacherId(id) => {
                UpdateColloscopeError::InvalidColloscopeTeacherId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidColloscopeWeekPatternId(id) => {
                UpdateColloscopeError::InvalidColloscopeWeekPatternId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidColloscopeSlotId(id) => {
                UpdateColloscopeError::InvalidColloscopeSlotId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidColloscopeIncompatId(id) => {
                UpdateColloscopeError::InvalidColloscopeIncompatId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidColloscopeGroupListId(id) => {
                UpdateColloscopeError::InvalidColloscopeGroupListId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidColloscopeRuleId(id) => {
                UpdateColloscopeError::InvalidColloscopeRuleId(id.into())
            }
            crate::ops::UpdateColloscopeError::InvariantErrorInParameters(_) => {
                UpdateColloscopeError::InvariantErrorInParameters
            }
            crate::ops::UpdateColloscopeError::WrongPeriodCountInColloscopeData => {
                UpdateColloscopeError::WrongPeriodCountInColloscopeData
            }
            crate::ops::UpdateColloscopeError::WrongGroupListCountInColloscopeData => {
                UpdateColloscopeError::WrongGroupListCountInColloscopeData
            }
            crate::ops::UpdateColloscopeError::WrongSubjectCountInPeriodInColloscopeData(period_id) => {
                UpdateColloscopeError::WrongSubjectCountInPeriodInColloscopeData(period_id.into())
            }
            crate::ops::UpdateColloscopeError::WrongSlotCountForSubjectInPeriodInColloscopeData(period_id, subject_id) => {
                UpdateColloscopeError::WrongSlotCountForSubjectInPeriodInColloscopeData(period_id.into(), subject_id.into())
            }
            crate::ops::UpdateColloscopeError::WrongInterrogationCountForSlotInPeriodInColloscopeData(period_id, slot_id) => {
                UpdateColloscopeError::WrongInterrogationCountForSlotInPeriodInColloscopeData(period_id.into(), slot_id.into())
            }
            crate::ops::UpdateColloscopeError::InterrogationOnNonInterrogationWeek(period_id, slot_id, week) => {
                UpdateColloscopeError::InterrogationOnNonInterrogationWeek(period_id.into(), slot_id.into(), week)
            }
            crate::ops::UpdateColloscopeError::MissingInterrogationOnInterrogationWeek(period_id, slot_id, week) => {
                UpdateColloscopeError::MissingInterrogationOnInterrogationWeek(period_id.into(), slot_id.into(), week)
            }
            crate::ops::UpdateColloscopeError::InvalidGroupNumInInterrogation(period_id, slot_id, group_num) => {
                UpdateColloscopeError::InvalidGroupNumInInterrogation(period_id.into(), slot_id.into(), group_num)
            }
            crate::ops::UpdateColloscopeError::ExcludedStudentInGroupList(group_list_id, student_id) => {
                UpdateColloscopeError::ExcludedStudentInGroupList(group_list_id.into(), student_id.into())
            }
            crate::ops::UpdateColloscopeError::WrongStudentCountInGroupList(group_list_id) => {
                UpdateColloscopeError::WrongStudentCountInGroupList(group_list_id.into())
            }
            crate::ops::UpdateColloscopeError::InvalidGroupNumForStudentInGroupList(group_list_id, student_id) => {
                UpdateColloscopeError::InvalidGroupNumForStudentInGroupList(group_list_id.into(), student_id.into())
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
