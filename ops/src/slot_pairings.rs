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
                    .try_apply(
                        collomatique_state_colloscopes::Op::SlotPairing(
                            collomatique_state_colloscopes::SlotPairingOp::Add(rule.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            ApplyError, Convergence, FixableInvariant, PeriodRefSite, Reference,
                            SlotRefSite,
                        };
                        match e {
                            // The pre-op state was valid, so any break in the set was
                            // introduced by this Add. Old validator order
                            // (validate_slot_pairing_rule_internal): antecedent slot,
                            // then consequent slot, then same-subject, then excluded
                            // period. Both slot sites map to InvalidSlotId but carry
                            // different payloads, so the passes stay separate; the
                            // same-subject convergence carries only the rule id, so
                            // the two slot ids come from the op payload in scope.
                            ApplyError::Invariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Slot {
                                        target,
                                        site: SlotRefSite::SlotPairingRuleAntecedent(_),
                                    }) = inv
                                    {
                                        return AddNewSlotPairingRuleError::InvalidSlotId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Slot {
                                        target,
                                        site: SlotRefSite::SlotPairingRuleConsequent(_),
                                    }) = inv
                                    {
                                        return AddNewSlotPairingRuleError::InvalidSlotId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::PairedSlotsNotInSameSubject(_),
                                    ) = inv
                                    {
                                        return AddNewSlotPairingRuleError::SlotsNotInSameSubject(
                                            rule.antecedent().slot_id,
                                            rule.consequent().slot_id,
                                        );
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Period {
                                        target,
                                        site: PeriodRefSite::SlotPairingRuleExcludedPeriods(_),
                                    }) = inv
                                    {
                                        return AddNewSlotPairingRuleError::InvalidPeriodId(*target);
                                    }
                                }
                                panic!(
                                    "Unexpected invariant breaks during AddNewSlotPairingRule: {set:?}"
                                );
                            }
                            _ => panic!("Unexpected error during AddNewSlotPairingRule: {e:?}"),
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
                    .try_apply(
                        collomatique_state_colloscopes::Op::SlotPairing(
                            collomatique_state_colloscopes::SlotPairingOp::Remove(*rule_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            ApplyError, PrecheckError, SlotPairingPrecheckError,
                        };
                        match e {
                            ApplyError::Precheck(PrecheckError::SlotPairing(
                                SlotPairingPrecheckError::InvalidSlotPairingRuleId(id),
                            )) => DeleteSlotPairingRuleError::InvalidSlotPairingRuleId(id),
                            _ => panic!("Unexpected error during DeleteSlotPairingRule: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdateSlotPairingRule(rule_id, rule) => {
                let result = data
                    .try_apply(
                        collomatique_state_colloscopes::Op::SlotPairing(
                            collomatique_state_colloscopes::SlotPairingOp::Update(
                                *rule_id,
                                rule.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            ApplyError, Convergence, FixableInvariant, PeriodRefSite, PrecheckError,
                            Reference, SlotPairingPrecheckError, SlotRefSite,
                        };
                        match e {
                            ApplyError::Precheck(PrecheckError::SlotPairing(
                                SlotPairingPrecheckError::InvalidSlotPairingRuleId(id),
                            )) => UpdateSlotPairingRuleError::InvalidSlotPairingRuleId(id),
                            // Old validator order: antecedent slot, then consequent
                            // slot, then same-subject, then excluded period.
                            ApplyError::Invariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Slot {
                                        target,
                                        site: SlotRefSite::SlotPairingRuleAntecedent(_),
                                    }) = inv
                                    {
                                        return UpdateSlotPairingRuleError::InvalidSlotId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Slot {
                                        target,
                                        site: SlotRefSite::SlotPairingRuleConsequent(_),
                                    }) = inv
                                    {
                                        return UpdateSlotPairingRuleError::InvalidSlotId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::PairedSlotsNotInSameSubject(_),
                                    ) = inv
                                    {
                                        return UpdateSlotPairingRuleError::SlotsNotInSameSubject(
                                            rule.antecedent().slot_id,
                                            rule.consequent().slot_id,
                                        );
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Period {
                                        target,
                                        site: PeriodRefSite::SlotPairingRuleExcludedPeriods(_),
                                    }) = inv
                                    {
                                        return UpdateSlotPairingRuleError::InvalidPeriodId(*target);
                                    }
                                }
                                panic!(
                                    "Unexpected invariant breaks during UpdateSlotPairingRule: {set:?}"
                                );
                            }
                            _ => panic!("Unexpected error during UpdateSlotPairingRule: {e:?}"),
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
