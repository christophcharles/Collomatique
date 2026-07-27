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
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, PeriodRefSite, Reference, SubjectRefSite,
                        };
                        match e {
                            // The pre-op state was valid, so any dangle in the set
                            // was introduced by this Add. Old validator order
                            // (validate_pairing_rule_internal): antecedent subject,
                            // then consequent subject, then excluded period. Both
                            // subject sites map to InvalidSubjectId but carry
                            // different payloads, so the passes stay separate.
                            Error::BrokenInvariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::PairingRuleAntecedent(_),
                                    }) = inv
                                    {
                                        return AddNewPairingRuleError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::PairingRuleConsequent(_),
                                    }) = inv
                                    {
                                        return AddNewPairingRuleError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Period {
                                        target,
                                        site: PeriodRefSite::PairingRuleExcludedPeriods(_),
                                    }) = inv
                                    {
                                        return AddNewPairingRuleError::InvalidPeriodId(*target);
                                    }
                                }
                                panic!(
                                    "Unexpected invariant breaks during AddNewPairingRule: {set:?}"
                                );
                            }
                            _ => panic!("Unexpected error during AddNewPairingRule: {e:?}"),
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
                        use collomatique_state_colloscopes::{
                            Error, InvalidOp, PairingPrecheckError, PrecheckError,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Pairing(
                                PairingPrecheckError::InvalidPairingRuleId(id),
                            ))) => DeletePairingRuleError::InvalidPairingRuleId(id),
                            _ => panic!("Unexpected error during DeletePairingRule: {e:?}"),
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
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, InvalidOp, PairingPrecheckError,
                            PeriodRefSite, PrecheckError, Reference, SubjectRefSite,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Pairing(
                                PairingPrecheckError::InvalidPairingRuleId(id),
                            ))) => UpdatePairingRuleError::InvalidPairingRuleId(id),
                            // Old validator order: antecedent subject, then
                            // consequent subject, then excluded period.
                            Error::BrokenInvariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::PairingRuleAntecedent(_),
                                    }) = inv
                                    {
                                        return UpdatePairingRuleError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::PairingRuleConsequent(_),
                                    }) = inv
                                    {
                                        return UpdatePairingRuleError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Period {
                                        target,
                                        site: PeriodRefSite::PairingRuleExcludedPeriods(_),
                                    }) = inv
                                    {
                                        return UpdatePairingRuleError::InvalidPeriodId(*target);
                                    }
                                }
                                panic!(
                                    "Unexpected invariant breaks during UpdatePairingRule: {set:?}"
                                );
                            }
                            _ => panic!("Unexpected error during UpdatePairingRule: {e:?}"),
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
