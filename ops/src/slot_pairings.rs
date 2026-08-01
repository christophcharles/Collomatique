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
                    .apply(
                        collomatique_state_colloscopes::Op::SlotPairing(
                            collomatique_state_colloscopes::SlotPairingOp::Add(rule.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, Convergence, FixableInvariant, PeriodRefSite, Reference,
                            SlotRefSite,
                        };
                        match e {
                            // The pre-op state was valid, so any break in the set was
                            // introduced by this Add. Old validator order
                            // (validate_slot_pairing_rule_internal): antecedent slot,
                            // then consequent slot, then same-subject, then excluded
                            // period. Both slot sites map to InvalidSlotId but carry
                            // different payloads, so the passes stay separate.
                            Error::BrokenInvariants(set) => {
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
                                        Convergence::PairedSlotsNotInSameSubject(
                                            _,
                                            antecedent,
                                            consequent,
                                        ),
                                    ) = inv
                                    {
                                        return AddNewSlotPairingRuleError::SlotsNotInSameSubject(
                                            *antecedent,
                                            *consequent,
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
                    .apply(
                        collomatique_state_colloscopes::Op::SlotPairing(
                            collomatique_state_colloscopes::SlotPairingOp::Remove(*rule_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, InvalidOp, PrecheckError, SlotPairingPrecheckError,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::SlotPairing(
                                SlotPairingPrecheckError::InvalidSlotPairingRuleId(id),
                            ))) => DeleteSlotPairingRuleError::InvalidSlotPairingRuleId(id),
                            _ => panic!("Unexpected error during DeleteSlotPairingRule: {e:?}"),
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
                        use collomatique_state_colloscopes::{
                            Error, Convergence, FixableInvariant, InvalidOp, PeriodRefSite,
                            PrecheckError, Reference, SlotPairingPrecheckError, SlotRefSite,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::SlotPairing(
                                SlotPairingPrecheckError::InvalidSlotPairingRuleId(id),
                            ))) => UpdateSlotPairingRuleError::InvalidSlotPairingRuleId(id),
                            // Old validator order: antecedent slot, then consequent
                            // slot, then same-subject, then excluded period.
                            Error::BrokenInvariants(set) => {
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
                                        Convergence::PairedSlotsNotInSameSubject(
                                            _,
                                            antecedent,
                                            consequent,
                                        ),
                                    ) = inv
                                    {
                                        return UpdateSlotPairingRuleError::SlotsNotInSameSubject(
                                            *antecedent,
                                            *consequent,
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

    // Nothing outside the tests calls this yet: the `UpdateOp` dispatch that
    // does is the last commit of the family migration. Drop the attribute then.
    #[allow(dead_code)]
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::SlotPairingRuleId>, SlotPairingsUpdateError>
    {
        match self {
            Self::AddNewSlotPairingRule(rule) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::SlotPairing(
                            collomatique_state_colloscopes::SlotPairingOp::Add(rule.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Convergence, Error, FixableInvariant, PeriodRefSite, Reference,
                            SlotRefSite,
                        };
                        match &e {
                            // Whatever the cascade could repair is repaired by
                            // the time this arm runs, so what is left was caused
                            // by the Add's own payload. Every break this op can
                            // cause names the new rule, and every arm of the map
                            // that answers those breaks starts by looking that
                            // rule up — but the rule went back with the
                            // rolled-back op, so none of them finds anything to
                            // take out and the target is convicted.
                            //
                            // The pre-op state was valid, so any break in the
                            // set was introduced by this Add. Old validator
                            // order (validate_slot_pairing_rule_internal):
                            // antecedent slot, then consequent slot, then
                            // same-subject, then excluded period. Both slot
                            // sites map to InvalidSlotId but carry different
                            // payloads, so the passes stay separate.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Slot {
                                        target,
                                        site: SlotRefSite::SlotPairingRuleAntecedent(_),
                                    }) = inv
                                    {
                                        return AddNewSlotPairingRuleError::InvalidSlotId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Slot {
                                        target,
                                        site: SlotRefSite::SlotPairingRuleConsequent(_),
                                    }) = inv
                                    {
                                        return AddNewSlotPairingRuleError::InvalidSlotId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::PairedSlotsNotInSameSubject(
                                            _,
                                            antecedent,
                                            consequent,
                                        ),
                                    ) = inv
                                    {
                                        return AddNewSlotPairingRuleError::SlotsNotInSameSubject(
                                            *antecedent,
                                            *consequent,
                                        );
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Period {
                                        target,
                                        site: PeriodRefSite::SlotPairingRuleExcludedPeriods(_),
                                    }) = inv
                                    {
                                        return AddNewSlotPairingRuleError::InvalidPeriodId(*target);
                                    }
                                }
                                // Nothing else can break: the rule cannot name
                                // one slot twice — that is
                                // `SlotPairingRule::new`'s seal, rejected long
                                // before the state layer is reached — and a
                                // slot's own context (its subject running
                                // interrogations, its week pattern, its start
                                // time) is the slots family's business, already
                                // true of every live slot the rule can name.
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
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::SlotPairing(
                            collomatique_state_colloscopes::SlotPairingOp::Remove(*rule_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, InvalidOp, PrecheckError, SlotPairingPrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::SlotPairing(
                                spe,
                            ))) => match spe {
                                SlotPairingPrecheckError::InvalidSlotPairingRuleId(id) => {
                                    DeleteSlotPairingRuleError::InvalidSlotPairingRuleId(*id)
                                }
                                SlotPairingPrecheckError::SlotPairingRuleIdAlreadyExists(_) => {
                                    panic!(
                                        "Unexpected SlotPairingPrecheckError during DeleteSlotPairingRule: {e:?}"
                                    )
                                }
                            },
                            // There is no invariant arm here and there never
                            // was: nothing in the document points at a slot
                            // pairing rule — no `Reference` variant carries a
                            // `SlotPairingRuleId` — so a removal breaks nothing
                            // and the cascade has nothing to repair.
                            _ => panic!("Unexpected error during DeleteSlotPairingRule: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdateSlotPairingRule(rule_id, rule) => {
                let result = session
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
                        use collomatique_state_colloscopes::{
                            Convergence, Error, FixableInvariant, InvalidOp, PeriodRefSite,
                            PrecheckError, Reference, SlotPairingPrecheckError, SlotRefSite,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::SlotPairing(
                                spe,
                            ))) => match spe {
                                SlotPairingPrecheckError::InvalidSlotPairingRuleId(id) => {
                                    UpdateSlotPairingRuleError::InvalidSlotPairingRuleId(*id)
                                }
                                SlotPairingPrecheckError::SlotPairingRuleIdAlreadyExists(_) => {
                                    panic!(
                                        "Unexpected SlotPairingPrecheckError during UpdateSlotPairingRule: {e:?}"
                                    )
                                }
                            },
                            // Same four scans as the Add, and for the same
                            // reason: the payload is the only thing that can
                            // break anything here. The rule itself survives the
                            // rollback, but with its *old* value, which the map
                            // finds innocent of every break the new one caused —
                            // each arm compares the break's coordinates against
                            // the rule as it stands, and the old value matches
                            // none of them. So again nothing is repaired and the
                            // target is convicted. Old validator order kept:
                            // antecedent slot, then consequent slot, then
                            // same-subject, then excluded period.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Slot {
                                        target,
                                        site: SlotRefSite::SlotPairingRuleAntecedent(_),
                                    }) = inv
                                    {
                                        return UpdateSlotPairingRuleError::InvalidSlotId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Slot {
                                        target,
                                        site: SlotRefSite::SlotPairingRuleConsequent(_),
                                    }) = inv
                                    {
                                        return UpdateSlotPairingRuleError::InvalidSlotId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::PairedSlotsNotInSameSubject(
                                            _,
                                            antecedent,
                                            consequent,
                                        ),
                                    ) = inv
                                    {
                                        return UpdateSlotPairingRuleError::SlotsNotInSameSubject(
                                            *antecedent,
                                            *consequent,
                                        );
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Period {
                                        target,
                                        site: PeriodRefSite::SlotPairingRuleExcludedPeriods(_),
                                    }) = inv
                                    {
                                        return UpdateSlotPairingRuleError::InvalidPeriodId(
                                            *target,
                                        );
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

#[cfg(test)]
mod tests {
    //! The third leaf of the reference graph in a row, after the
    //! incompatibilities and the pairing rules: a slot pairing rule points at
    //! two slots and at the periods it is excluded from, and *nothing* points
    //! back at it — no [collomatique_state_colloscopes::Reference] variant
    //! carries a `SlotPairingRuleId`. So no op of this family ever makes the
    //! cascade repair anything, and every fixture below asserts an empty
    //! warning log, the removal included.
    //!
    //! What is new here is the fourth thing a payload can get wrong: besides
    //! the three dangling edges (antecedent slot, consequent slot, excluded
    //! period) a rule may name two *live* slots that belong to different
    //! subjects, which the checker answers with
    //! [collomatique_state_colloscopes::Convergence::PairedSlotsNotInSameSubject].
    //! And unlike the pairing rules, this family's breaks are ones the map does
    //! know a repair for in general — every one of them answers
    //! [collomatique_state_colloscopes::Fix::DeleteSlotPairingRule] or
    //! [collomatique_state_colloscopes::Fix::RemoveSlotPairingRulePeriodExclusion]
    //! when a real slot or a real period disappears under a settled rule. What
    //! keeps them from firing on the user's own bad payload is the presence
    //! test at the head of each arm: a rejected op changes nothing, so the map
    //! sees either no rule at all (Add) or the old, innocent value (Update),
    //! matches none of the break's coordinates against it, and answers `None`.
    //! The engine then convicts the target and the scans below turn the break
    //! back into the bad input it came from. Every fixture that provokes a
    //! break asserts the document afterwards, so what is pinned is the outcome
    //! — the rule is never quietly deleted or rewritten as a repair — rather
    //! than which guard produced it.
    //!
    //! The frozen hogwarts base already holds two rules — one over two
    //! Métamorphose slots, one over two Arithmancie ones — so nothing here has
    //! to be seeded.

    use super::*;
    use crate::test_utils::hogwarts;
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        Op, SlotPairingOp,
        ids::{Id, PeriodId, SlotId, SlotPairingRuleId, SubjectId},
        slot_pairings::{SlotPairingRule, SlotRulePart},
    };

    fn subject_by_name(data: &Data, name: &str) -> SubjectId {
        data.get_inner_data()
            .params
            .subjects
            .ordered_subject_list
            .iter()
            .find(|(_id, subject)| subject.parameters.name == name)
            .map(|(id, _subject)| id)
            .unwrap_or_else(|| panic!("the fixture should have a subject named {name}"))
    }

    /// The subject's slots, in display order.
    fn slots_of_subject(data: &Data, subject: SubjectId) -> Vec<SlotId> {
        data.get_inner_data()
            .params
            .slots
            .slots_for_subject(subject)
            .into_iter()
            .flatten()
            .map(|(id, _slot)| *id)
            .collect()
    }

    /// The `n`-th period in display order.
    fn period_at(data: &Data, index: usize) -> PeriodId {
        data.get_inner_data()
            .params
            .periods
            .period_ids()
            .nth(index)
            .unwrap_or_else(|| panic!("the fixture should have at least {} periods", index + 1))
    }

    /// The fixture's own rule over `subject`'s slots. Hogwarts pairs two
    /// Métamorphose slots and two Arithmancie ones, so a subject names at most
    /// one rule — asserted here, since a second one would make every fixture
    /// below ambiguous.
    fn rule_over_subject(data: &Data, subject: SubjectId) -> SlotPairingRuleId {
        let slots = slots_of_subject(data, subject);
        let rules: Vec<_> = data
            .get_inner_data()
            .params
            .slot_pairings
            .slot_pairing_rule_map
            .iter()
            .filter(|(_id, rule)| slots.contains(&rule.antecedent().slot_id))
            .map(|(id, _rule)| id)
            .collect();

        assert_eq!(
            rules.len(),
            1,
            "the fixture should pair {subject:?}'s slots by exactly one rule"
        );

        rules[0]
    }

    fn rule_of(data: &Data, rule: SlotPairingRuleId) -> SlotPairingRule {
        data.get_inner_data()
            .params
            .slot_pairings
            .slot_pairing_rule_map
            .get(&rule)
            .expect("the fixture's slot pairing rule should be live")
            .clone()
    }

    /// « le créneau {slot} est utilisé », the shape every fixture part uses
    /// unless the test is about `should_have`.
    fn part(slot: SlotId) -> SlotRulePart {
        SlotRulePart {
            slot_id: slot,
            should_have: true,
        }
    }

    fn rule(antecedent: SlotId, consequent: SlotId, excluded: Vec<PeriodId>) -> SlotPairingRule {
        SlotPairingRule::new(
            part(antecedent),
            part(consequent),
            excluded.into_iter().collect(),
            false,
        )
        .expect("the fixtures never name one slot in both parts")
    }

    /// Ids no document ever issued.
    fn dangling_rule() -> SlotPairingRuleId {
        unsafe { SlotPairingRuleId::new(1u64 << 40) }
    }

    fn dangling_slot() -> SlotId {
        unsafe { SlotId::new(1u64 << 40) }
    }

    /// A second dead slot: the seal on [SlotPairingRule] forbids naming one
    /// slot in both parts, so a rule whose two parts are both dead needs two
    /// distinct dead ids.
    fn other_dangling_slot() -> SlotId {
        unsafe { SlotId::new((1u64 << 40) + 1) }
    }

    fn dangling_period() -> PeriodId {
        unsafe { PeriodId::new(1u64 << 40) }
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects to have landed.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::SlotPairings, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// A rule names only material that already exists, and two slots of one
    /// subject, so nothing in the document can need repairing: the id comes
    /// back and the log stays empty.
    #[test]
    fn adding_a_slot_pairing_rule_creates_it_and_warns_about_nothing() {
        let base = hogwarts();
        let divination = subject_by_name(base.get_data(), "Divination");
        let slots = slots_of_subject(base.get_data(), divination);
        let second_period = period_at(base.get_data(), 1);

        let new_rule = rule(slots[0], slots[1], vec![second_period]);

        let mut session = CascadeSession::new(base.clone());
        let op = SlotPairingsUpdateOp::AddNewSlotPairingRule(new_rule.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("both slots are live and share a subject, and so is the period");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding a slot pairing rule returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(rule_of(state.get_data(), new_id), new_rule);
    }

    /// Rewriting a rule replaces its whole value; what it used to name is
    /// referenced by nothing it leaves behind, so again there is nothing to
    /// repair. The slots it moves to belong to another subject than the ones it
    /// leaves — which is fine, since the constraint is that the *two parts*
    /// agree, not that the rule stays where it was.
    #[test]
    fn updating_a_slot_pairing_rule_rewrites_it_and_warns_about_nothing() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let rule_id = rule_over_subject(base.get_data(), metamorphose);
        let potions = subject_by_name(base.get_data(), "Potions");
        let potions_slots = slots_of_subject(base.get_data(), potions);
        let second_period = period_at(base.get_data(), 1);

        let rewritten = rule(potions_slots[0], potions_slots[1], vec![second_period]);

        let mut session = CascadeSession::new(base.clone());
        let op = SlotPairingsUpdateOp::UpdateSlotPairingRule(rule_id, rewritten.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("the rule, the two slots and the period are all live");
        let (state, warnings) = session.commit(op.get_desc());

        assert_eq!(new_id, None, "an update creates nothing");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::SlotPairing(SlotPairingOp::Update(rule_id, rewritten))],
            )
            .get_data(),
        );
    }

    /// The interesting empty log: deleting a slot takes the rules over it with
    /// it, but nothing at all points at a *rule*, so the single elementary op
    /// lands alone and the document is the base minus that one entry.
    #[test]
    fn deleting_a_slot_pairing_rule_takes_nothing_with_it() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let rule_id = rule_over_subject(base.get_data(), metamorphose);

        let mut session = CascadeSession::new(base.clone());
        let op = SlotPairingsUpdateOp::DeleteSlotPairingRule(rule_id);
        let new_id = op
            .apply_to_session(&mut session)
            .expect("nothing stands in the way of removing a slot pairing rule");
        let (state, warnings) = session.commit(op.get_desc());

        assert_eq!(new_id, None, "a removal creates nothing");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(&base, vec![Op::SlotPairing(SlotPairingOp::Remove(rule_id))])
                .get_data(),
        );
    }

    /// The state layer's own precheck, translated by the two ops that name an
    /// existing rule. A rejected op changes nothing and logs nothing.
    #[test]
    fn a_dead_slot_pairing_rule_id_is_rejected_by_update_and_by_delete() {
        let base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");
        let potions_slots = slots_of_subject(base.get_data(), potions);
        let dangling = dangling_rule();

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotPairingsUpdateOp::UpdateSlotPairingRule(
                dangling,
                rule(potions_slots[0], potions_slots[1], vec![])
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            SlotPairingsUpdateError::UpdateSlotPairingRule(
                UpdateSlotPairingRuleError::InvalidSlotPairingRuleId(dangling)
            ),
        );
        assert_eq!(
            SlotPairingsUpdateOp::DeleteSlotPairingRule(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::DeleteSlotPairingRule(
                DeleteSlotPairingRuleError::InvalidSlotPairingRuleId(dangling)
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::SlotPairings, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// A slot id the payload made up dangles the moment the op lands. On the
    /// Add the rule went back with the rolled-back op, on the Update it is back
    /// to its old — innocent — value; either way the map's presence test fails,
    /// the engine convicts the op, and the scan turns the break back into the
    /// bad input it came from.
    #[test]
    fn a_dead_antecedent_slot_is_rejected_on_add_and_on_update() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let rule_id = rule_over_subject(base.get_data(), metamorphose);
        let potions = subject_by_name(base.get_data(), "Potions");
        let potions_slots = slots_of_subject(base.get_data(), potions);
        let dangling = dangling_slot();

        let from_a_ghost = rule(dangling, potions_slots[0], vec![]);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotPairingsUpdateOp::AddNewSlotPairingRule(from_a_ghost.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::AddNewSlotPairingRule(
                AddNewSlotPairingRuleError::InvalidSlotId(dangling)
            ),
        );
        assert_eq!(
            SlotPairingsUpdateOp::UpdateSlotPairingRule(rule_id, from_a_ghost)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::UpdateSlotPairingRule(
                UpdateSlotPairingRuleError::InvalidSlotId(dangling)
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The second slot edge, same route — and a separate scan, because the two
    /// sites carry different payloads even though they answer with the same
    /// error variant.
    #[test]
    fn a_dead_consequent_slot_is_rejected_on_add_and_on_update() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let rule_id = rule_over_subject(base.get_data(), metamorphose);
        let potions = subject_by_name(base.get_data(), "Potions");
        let potions_slots = slots_of_subject(base.get_data(), potions);
        let dangling = dangling_slot();

        let to_a_ghost = rule(potions_slots[0], dangling, vec![]);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotPairingsUpdateOp::AddNewSlotPairingRule(to_a_ghost.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::AddNewSlotPairingRule(
                AddNewSlotPairingRuleError::InvalidSlotId(dangling)
            ),
        );
        assert_eq!(
            SlotPairingsUpdateOp::UpdateSlotPairingRule(rule_id, to_a_ghost)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::UpdateSlotPairingRule(
                UpdateSlotPairingRuleError::InvalidSlotId(dangling)
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The family's own break, and the one that is not a dangle: two live slots
    /// of *different* subjects. A rule says « si ce créneau sert, alors
    /// celui-là aussi », which only means something inside one subject.
    ///
    /// The Update deliberately keeps the rule's own antecedent and swaps only
    /// its consequent: the live rule then still matches the break's *first*
    /// coordinate, which is the closest a payload of this family can get to
    /// making the map recognize a rule it should not touch. It ends convicted
    /// all the same. Two guards say so and this pin does not distinguish them —
    /// the map's arm compares both coordinates and answers `None`, and even a
    /// map that only compared the first would end here, since the repair it
    /// would then choose consumes the target's own target and the engine
    /// restores its entry snapshot and answers with the break it remembered.
    #[test]
    fn slots_of_different_subjects_are_rejected_on_add_and_on_update() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let rule_id = rule_over_subject(base.get_data(), metamorphose);
        let metamorphose_slots = slots_of_subject(base.get_data(), metamorphose);
        let potions = subject_by_name(base.get_data(), "Potions");
        let potions_slots = slots_of_subject(base.get_data(), potions);

        let across_subjects = rule(metamorphose_slots[0], potions_slots[0], vec![]);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotPairingsUpdateOp::AddNewSlotPairingRule(across_subjects)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::AddNewSlotPairingRule(
                AddNewSlotPairingRuleError::SlotsNotInSameSubject(
                    metamorphose_slots[0],
                    potions_slots[0]
                )
            ),
        );

        // Only the consequent moves: the live rule keeps the break's
        // antecedent, so the map's arm reaches its second test.
        let (antecedent, _consequent, excluded_periods, soft) =
            rule_of(base.get_data(), rule_id).into_parts();
        let antecedent_slot = antecedent.slot_id;
        let consequent_moved_out =
            SlotPairingRule::new(antecedent, part(potions_slots[0]), excluded_periods, soft)
                .expect("the two slots are in different subjects, so they are different slots");

        assert_eq!(
            SlotPairingsUpdateOp::UpdateSlotPairingRule(rule_id, consequent_moved_out)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::UpdateSlotPairingRule(
                UpdateSlotPairingRuleError::SlotsNotInSameSubject(
                    antecedent_slot,
                    potions_slots[0]
                )
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The last edge: a made-up period in the exclusion set comes back as the
    /// input it was. Worth its own fixture because this is the one break whose
    /// repair *rewrites* the rule instead of deleting it —
    /// [collomatique_state_colloscopes::Fix::RemoveSlotPairingRulePeriodExclusion]
    /// lifts a stale exclusion when a real period disappears. It cannot help
    /// here: the live rule (old value, or none at all after the rollback) does
    /// not exclude that period, so the arm answers `None` and the op is
    /// convicted rather than silently stripped of the exclusion the user asked
    /// for.
    #[test]
    fn a_dead_excluded_period_is_rejected_on_add_and_on_update() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let rule_id = rule_over_subject(base.get_data(), metamorphose);
        let potions = subject_by_name(base.get_data(), "Potions");
        let potions_slots = slots_of_subject(base.get_data(), potions);
        let dangling = dangling_period();

        let excluding_a_ghost = rule(potions_slots[0], potions_slots[1], vec![dangling]);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotPairingsUpdateOp::AddNewSlotPairingRule(excluding_a_ghost.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::AddNewSlotPairingRule(
                AddNewSlotPairingRuleError::InvalidPeriodId(dangling)
            ),
        );
        assert_eq!(
            SlotPairingsUpdateOp::UpdateSlotPairingRule(rule_id, excluding_a_ghost)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::UpdateSlotPairingRule(
                UpdateSlotPairingRuleError::InvalidPeriodId(dangling)
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// Which break wins when a payload carries several is public API (D5): the
    /// old validator checked the antecedent slot, then the consequent slot,
    /// then the same-subject rule, then the excluded periods, and the four
    /// scans are copied in that order. The set the engine hands over holds
    /// every break at once, so only the scan order decides.
    ///
    /// Three of the four steps are pinned below; the fourth — antecedent slot
    /// over same-subject — is unreachable rather than untested: the checker
    /// only raises
    /// [collomatique_state_colloscopes::Convergence::PairedSlotsNotInSameSubject]
    /// when *both* slots resolve to a subject, so a dead antecedent gates it
    /// off. The scan stays where the old validator put it all the same.
    #[test]
    fn a_payload_naming_several_bad_things_reports_them_in_the_old_order() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let rule_id = rule_over_subject(base.get_data(), metamorphose);
        let metamorphose_slots = slots_of_subject(base.get_data(), metamorphose);
        let potions = subject_by_name(base.get_data(), "Potions");
        let potions_slots = slots_of_subject(base.get_data(), potions);
        let dead_antecedent = dangling_slot();
        // `SlotPairingRule::new` forbids one slot in both parts, so a payload
        // whose two slots are both dead needs two *distinct* dead ids.
        let dead_consequent = other_dangling_slot();

        // Both slots dead and the period too: the antecedent wins.
        let all_dead = rule(dead_antecedent, dead_consequent, vec![dangling_period()]);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotPairingsUpdateOp::AddNewSlotPairingRule(all_dead)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::AddNewSlotPairingRule(
                AddNewSlotPairingRuleError::InvalidSlotId(dead_antecedent)
            ),
        );

        // Antecedent live, consequent and period dead: the consequent wins over
        // the period.
        let last_two = rule(
            metamorphose_slots[0],
            dead_consequent,
            vec![dangling_period()],
        );
        assert_eq!(
            SlotPairingsUpdateOp::UpdateSlotPairingRule(rule_id, last_two)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::UpdateSlotPairingRule(
                UpdateSlotPairingRuleError::InvalidSlotId(dead_consequent)
            ),
        );

        // Both slots live but in different subjects, and the period dead: the
        // same-subject break wins over the period.
        let across_and_a_ghost_period = rule(
            metamorphose_slots[0],
            potions_slots[0],
            vec![dangling_period()],
        );
        assert_eq!(
            SlotPairingsUpdateOp::AddNewSlotPairingRule(across_and_a_ghost_period)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotPairingsUpdateError::AddNewSlotPairingRule(
                AddNewSlotPairingRuleError::SlotsNotInSameSubject(
                    metamorphose_slots[0],
                    potions_slots[0]
                )
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }
}
