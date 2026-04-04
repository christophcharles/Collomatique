use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlotPairingsUpdateWarning {}

impl SlotPairingsUpdateWarning {
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
pub enum SlotPairingsUpdateOp {
    AddNewSlotPairingRule(collomatique_state_colloscopes::slot_pairings::SlotPairingRule),
    DeleteSlotPairingRule(collomatique_state_colloscopes::SlotPairingRuleId),
    UpdateSlotPairingRule(
        collomatique_state_colloscopes::SlotPairingRuleId,
        collomatique_state_colloscopes::slot_pairings::SlotPairingRule,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotPairingsUpdateError {
    #[error(transparent)]
    AddNewSlotPairingRule(#[from] AddNewSlotPairingRuleError),
    #[error(transparent)]
    DeleteSlotPairingRule(#[from] DeleteSlotPairingRuleError),
    #[error(transparent)]
    UpdateSlotPairingRule(#[from] UpdateSlotPairingRuleError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewSlotPairingRuleError {
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("same slot in both parts ({0:?})")]
    SameSlotInBothParts(collomatique_state_colloscopes::SlotId),
    #[error("slots {0:?} and {1:?} do not belong to the same subject")]
    SlotsNotInSameSubject(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::SlotId,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteSlotPairingRuleError {
    #[error("invalid slot pairing rule id ({0:?})")]
    InvalidSlotPairingRuleId(collomatique_state_colloscopes::SlotPairingRuleId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateSlotPairingRuleError {
    #[error("invalid slot pairing rule id ({0:?})")]
    InvalidSlotPairingRuleId(collomatique_state_colloscopes::SlotPairingRuleId),
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("same slot in both parts ({0:?})")]
    SameSlotInBothParts(collomatique_state_colloscopes::SlotId),
    #[error("slots {0:?} and {1:?} do not belong to the same subject")]
    SlotsNotInSameSubject(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::SlotId,
    ),
}

impl SlotPairingsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<SlotPairingsUpdateWarning>> {
        match self {
            Self::AddNewSlotPairingRule(_) => None,
            Self::DeleteSlotPairingRule(_) => None,
            Self::UpdateSlotPairingRule(_, _) => None,
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::SlotPairingRuleId>, SlotPairingsUpdateError>
    {
        match self {
            Self::AddNewSlotPairingRule(rule) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::SlotPairing(
                            collomatique_state_colloscopes::SlotPairingOp::Add(rule.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::SlotPairing(pe) = e {
                            match pe {
                                collomatique_state_colloscopes::SlotPairingError::InvalidSlotId(
                                    id,
                                ) => AddNewSlotPairingRuleError::InvalidSlotId(id),
                                collomatique_state_colloscopes::SlotPairingError::InvalidPeriodId(
                                    id,
                                ) => AddNewSlotPairingRuleError::InvalidPeriodId(id),
                                collomatique_state_colloscopes::SlotPairingError::SameSlotInBothParts(id) => {
                                    AddNewSlotPairingRuleError::SameSlotInBothParts(id)
                                }
                                collomatique_state_colloscopes::SlotPairingError::SlotsNotInSameSubject(id1, id2) => {
                                    AddNewSlotPairingRuleError::SlotsNotInSameSubject(id1, id2)
                                }
                                _ => panic!(
                                    "Unexpected slot pairing error during AddNewSlotPairingRule: {:?}",
                                    pe
                                ),
                            }
                        } else {
                            panic!(
                                "Unexpected error during AddNewSlotPairingRule: {:?}",
                                e
                            );
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::SlotPairingRuleId(new_id)) = result
                else {
                    panic!("Unexpected result from SlotPairingOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::DeleteSlotPairingRule(rule_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::SlotPairing(
                            collomatique_state_colloscopes::SlotPairingOp::Remove(*rule_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::SlotPairing(pe) = e {
                            match pe {
                                collomatique_state_colloscopes::SlotPairingError::InvalidSlotPairingRuleId(id) => {
                                    DeleteSlotPairingRuleError::InvalidSlotPairingRuleId(id)
                                }
                                _ => panic!(
                                    "Unexpected slot pairing error during DeleteSlotPairingRule: {:?}",
                                    pe
                                ),
                            }
                        } else {
                            panic!(
                                "Unexpected error during DeleteSlotPairingRule: {:?}",
                                e
                            );
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdateSlotPairingRule(rule_id, rule) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::SlotPairing(
                            collomatique_state_colloscopes::SlotPairingOp::Update(
                                *rule_id,
                                rule.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::SlotPairing(pe) = e {
                            match pe {
                                collomatique_state_colloscopes::SlotPairingError::InvalidSlotPairingRuleId(id) => {
                                    UpdateSlotPairingRuleError::InvalidSlotPairingRuleId(id)
                                }
                                collomatique_state_colloscopes::SlotPairingError::InvalidSlotId(
                                    id,
                                ) => UpdateSlotPairingRuleError::InvalidSlotId(id),
                                collomatique_state_colloscopes::SlotPairingError::InvalidPeriodId(
                                    id,
                                ) => UpdateSlotPairingRuleError::InvalidPeriodId(id),
                                collomatique_state_colloscopes::SlotPairingError::SameSlotInBothParts(id) => {
                                    UpdateSlotPairingRuleError::SameSlotInBothParts(id)
                                }
                                collomatique_state_colloscopes::SlotPairingError::SlotsNotInSameSubject(id1, id2) => {
                                    UpdateSlotPairingRuleError::SlotsNotInSameSubject(id1, id2)
                                }
                                _ => panic!(
                                    "Unexpected slot pairing error during UpdateSlotPairingRule: {:?}",
                                    pe
                                ),
                            }
                        } else {
                            panic!(
                                "Unexpected error during UpdateSlotPairingRule: {:?}",
                                e
                            );
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::SlotPairings,
            match self {
                SlotPairingsUpdateOp::AddNewSlotPairingRule(_) => {
                    "Ajouter un appariement de créneaux".into()
                }
                SlotPairingsUpdateOp::DeleteSlotPairingRule(_) => {
                    "Supprimer un appariement de créneaux".into()
                }
                SlotPairingsUpdateOp::UpdateSlotPairingRule(_, _) => {
                    "Modifier un appariement de créneaux".into()
                }
            },
        )
    }
}
