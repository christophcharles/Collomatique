use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PairingsUpdateWarning {}

impl PairingsUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PairingsUpdateOp {
    AddNewPairingRule(collomatique_state_colloscopes::pairings::PairingRule),
    DeletePairingRule(collomatique_state_colloscopes::PairingRuleId),
    UpdatePairingRule(
        collomatique_state_colloscopes::PairingRuleId,
        collomatique_state_colloscopes::pairings::PairingRule,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum PairingsUpdateError {
    #[error(transparent)]
    AddNewPairingRule(#[from] AddNewPairingRuleError),
    #[error(transparent)]
    DeletePairingRule(#[from] DeletePairingRuleError),
    #[error(transparent)]
    UpdatePairingRule(#[from] UpdatePairingRuleError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewPairingRuleError {
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeletePairingRuleError {
    #[error("invalid pairing rule id ({0:?})")]
    InvalidPairingRuleId(collomatique_state_colloscopes::PairingRuleId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdatePairingRuleError {
    #[error("invalid pairing rule id ({0:?})")]
    InvalidPairingRuleId(collomatique_state_colloscopes::PairingRuleId),
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

impl PairingsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<PairingsUpdateWarning>> {
        match self {
            Self::AddNewPairingRule(_) => None,
            Self::DeletePairingRule(_) => None,
            Self::UpdatePairingRule(_, _) => None,
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::PairingRuleId>, PairingsUpdateError> {
        match self {
            Self::AddNewPairingRule(rule) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Pairing(
                            collomatique_state_colloscopes::PairingOp::Add(rule.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Pairing(pe) = e {
                            match pe {
                                collomatique_state_colloscopes::PairingError::InvalidSubjectId(
                                    id,
                                ) => AddNewPairingRuleError::InvalidSubjectId(id),
                                collomatique_state_colloscopes::PairingError::InvalidPeriodId(
                                    id,
                                ) => AddNewPairingRuleError::InvalidPeriodId(id),
                                _ => panic!(
                                    "Unexpected pairing error during AddNewPairingRule: {:?}",
                                    pe
                                ),
                            }
                        } else {
                            panic!("Unexpected error during AddNewPairingRule: {:?}", e);
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::PairingRuleId(new_id)) = result
                else {
                    panic!("Unexpected result from PairingOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::DeletePairingRule(rule_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Pairing(
                            collomatique_state_colloscopes::PairingOp::Remove(*rule_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Pairing(pe) = e {
                            match pe {
                                collomatique_state_colloscopes::PairingError::InvalidPairingRuleId(id) => {
                                    DeletePairingRuleError::InvalidPairingRuleId(id)
                                }
                                _ => panic!(
                                    "Unexpected pairing error during DeletePairingRule: {:?}",
                                    pe
                                ),
                            }
                        } else {
                            panic!("Unexpected error during DeletePairingRule: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdatePairingRule(rule_id, rule) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Pairing(
                            collomatique_state_colloscopes::PairingOp::Update(
                                *rule_id,
                                rule.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Pairing(pe) = e {
                            match pe {
                                collomatique_state_colloscopes::PairingError::InvalidPairingRuleId(id) => {
                                    UpdatePairingRuleError::InvalidPairingRuleId(id)
                                }
                                collomatique_state_colloscopes::PairingError::InvalidSubjectId(
                                    id,
                                ) => UpdatePairingRuleError::InvalidSubjectId(id),
                                collomatique_state_colloscopes::PairingError::InvalidPeriodId(
                                    id,
                                ) => UpdatePairingRuleError::InvalidPeriodId(id),
                                _ => panic!(
                                    "Unexpected pairing error during UpdatePairingRule: {:?}",
                                    pe
                                ),
                            }
                        } else {
                            panic!("Unexpected error during UpdatePairingRule: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Pairings,
            match self {
                PairingsUpdateOp::AddNewPairingRule(_) => "Ajouter un appariement".into(),
                PairingsUpdateOp::DeletePairingRule(_) => "Supprimer un appariement".into(),
                PairingsUpdateOp::UpdatePairingRule(_, _) => "Modifier un appariement".into(),
            },
        )
    }
}
