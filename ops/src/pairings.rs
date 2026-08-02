use super::*;

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
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::PairingRuleId>, PairingsUpdateError> {
        match self {
            Self::AddNewPairingRule(rule) => {
                let result = session
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
                        match &e {
                            // Whatever the cascade could repair is repaired by
                            // the time this arm runs, so what is left was caused
                            // by the Add's own payload: the rule went back with
                            // the rolled-back op, every arm of the map starts by
                            // looking that rule up, so none of them finds
                            // anything to take out and the target is convicted.
                            //
                            // The pre-op state was valid, so any dangle in the
                            // set was introduced by this Add. Old validator
                            // order (validate_pairing_rule_internal): antecedent
                            // subject, then consequent subject, then excluded
                            // period. Both subject sites map to
                            // InvalidSubjectId but carry different payloads, so
                            // the passes stay separate.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::PairingRuleAntecedent(_),
                                    }) = inv
                                    {
                                        return AddNewPairingRuleError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::PairingRuleConsequent(_),
                                    }) = inv
                                    {
                                        return AddNewPairingRuleError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Period {
                                        target,
                                        site: PeriodRefSite::PairingRuleExcludedPeriods(_),
                                    }) = inv
                                    {
                                        return AddNewPairingRuleError::InvalidPeriodId(*target);
                                    }
                                }
                                // Nothing else can break: the two subjects a
                                // rule names need no interrogations of their own
                                // (there is no convergence on that edge, unlike
                                // for teachers, slots or balancing options), and
                                // the rule cannot name one subject twice — that
                                // is `PairingRule::new`'s seal, rejected long
                                // before the state layer is reached.
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
                let result = session
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
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Pairing(pe))) => {
                                match pe {
                                    PairingPrecheckError::InvalidPairingRuleId(id) => {
                                        DeletePairingRuleError::InvalidPairingRuleId(*id)
                                    }
                                    PairingPrecheckError::PairingRuleIdAlreadyExists(_) => panic!(
                                        "Unexpected PairingPrecheckError during DeletePairingRule: {e:?}"
                                    ),
                                }
                            }
                            // There is no invariant arm here and there never
                            // was: nothing in the document points at a pairing
                            // rule — no `Reference` variant carries a
                            // `PairingRuleId` — so a removal breaks nothing and
                            // the cascade has nothing to repair.
                            _ => panic!("Unexpected error during DeletePairingRule: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdatePairingRule(rule_id, rule) => {
                let result = session
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
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Pairing(pe))) => {
                                match pe {
                                    PairingPrecheckError::InvalidPairingRuleId(id) => {
                                        UpdatePairingRuleError::InvalidPairingRuleId(*id)
                                    }
                                    PairingPrecheckError::PairingRuleIdAlreadyExists(_) => panic!(
                                        "Unexpected PairingPrecheckError during UpdatePairingRule: {e:?}"
                                    ),
                                }
                            }
                            // Same three scans as the Add, and for the same
                            // reason: the payload is the only thing that can
                            // dangle here. The rule itself survives the
                            // rollback, but with its *old* value, which the map
                            // finds innocent of every break the new one caused —
                            // so again nothing is repaired and the target is
                            // convicted. Old validator order kept: antecedent
                            // subject, then consequent subject, then excluded
                            // period.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::PairingRuleAntecedent(_),
                                    }) = inv
                                    {
                                        return UpdatePairingRuleError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::PairingRuleConsequent(_),
                                    }) = inv
                                    {
                                        return UpdatePairingRuleError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in set {
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

#[cfg(test)]
mod tests {
    //! Like the incompatibilities, the family sits at a leaf of the reference
    //! graph: a pairing rule points at two subjects and at the periods it is
    //! excluded from, and *nothing* points back at it — no
    //! [collomatique_state_colloscopes::Reference] variant carries a
    //! `PairingRuleId`. So no op of this family ever makes the cascade repair
    //! anything, and every fixture below asserts an empty warning log, the
    //! removal included.
    //!
    //! What is worth pinning is the error surface: three edges can dangle
    //! (antecedent subject, consequent subject, excluded period) against the
    //! incompats' two, and the two subject edges answer with the *same* error
    //! variant, so only the scan order tells them apart — which makes the
    //! ordering fixture below the one that carries the family's public API.
    //!
    //! The frozen hogwarts base holds no pairing rule at all, so the fixtures
    //! that need a live one seed it themselves with the elementary op, on top
    //! of the loaded base and in plain sight.

    use super::*;
    use crate::test_utils::hogwarts;
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        Op, PairingOp,
        ids::{Id, PairingRuleId, PeriodId, SubjectId},
        pairings::{PairingRule, RulePart},
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

    /// The `n`-th period in display order.
    fn period_at(data: &Data, index: usize) -> PeriodId {
        data.get_inner_data()
            .params
            .periods
            .period_ids()
            .nth(index)
            .unwrap_or_else(|| panic!("the fixture should have at least {} periods", index + 1))
    }

    fn rule_of(data: &Data, rule: PairingRuleId) -> PairingRule {
        data.get_inner_data()
            .params
            .pairings
            .pairing_rule_map
            .get(&rule)
            .expect("the fixture's pairing rule should be live")
            .clone()
    }

    /// « avoir une colle en {subject} », the shape both parts of every fixture
    /// rule use unless the test is about `should_have`.
    fn part(subject: SubjectId) -> RulePart {
        RulePart {
            subject_id: subject,
            should_have: true,
        }
    }

    fn rule(antecedent: SubjectId, consequent: SubjectId, excluded: Vec<PeriodId>) -> PairingRule {
        PairingRule::new(
            part(antecedent),
            part(consequent),
            excluded.into_iter().collect(),
            false,
        )
        .expect("the fixtures never name one subject in both parts")
    }

    /// Ids no document ever issued.
    fn dangling_rule() -> PairingRuleId {
        unsafe { PairingRuleId::new(1u64 << 40) }
    }

    fn dangling_subject() -> SubjectId {
        unsafe { SubjectId::new(1u64 << 40) }
    }

    /// A second dead subject: the seal on [PairingRule] forbids naming one
    /// subject in both parts, so a rule whose two parts are both dead needs two
    /// distinct dead ids.
    fn other_dangling_subject() -> SubjectId {
        unsafe { SubjectId::new((1u64 << 40) + 1) }
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
                .apply(op, (OpCategory::Pairings, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// The base the update and delete fixtures start from: hogwarts plus one
    /// rule « une colle en Potions implique une colle en Métamorphose », not
    /// applying on the first period.
    fn base_with_a_rule() -> (AppState<Data, Desc>, PairingRuleId) {
        let mut base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let first_period = period_at(base.get_data(), 0);

        let new_id = base
            .apply(
                Op::Pairing(PairingOp::Add(rule(
                    potions,
                    metamorphose,
                    vec![first_period],
                ))),
                (OpCategory::Pairings, "Amorce".into()),
            )
            .expect("the seed names only live material");
        let Some(collomatique_state_colloscopes::NewId::PairingRuleId(rule_id)) = new_id else {
            panic!("Unexpected result from PairingOp::Add");
        };

        (base, rule_id)
    }

    /// A rule names only material that already exists, so nothing in the
    /// document can need repairing: the id comes back and the log stays empty.
    #[test]
    fn adding_a_pairing_rule_creates_it_and_warns_about_nothing() {
        let base = hogwarts();
        let divination = subject_by_name(base.get_data(), "Divination");
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let second_period = period_at(base.get_data(), 1);

        let new_rule = rule(divination, arithmancie, vec![second_period]);

        let mut session = CascadeSession::new(base.clone());
        let op = PairingsUpdateOp::AddNewPairingRule(new_rule.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("the two subjects and the period are all live");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding a pairing rule returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(rule_of(state.get_data(), new_id), new_rule);
    }

    /// The edge the family is deliberately loose about: a pairing rule may name
    /// a subject that runs **no interrogations**. The antecedent then never
    /// holds and the rule is vacuous, which is harmless — and there is no
    /// convergence on that edge (unlike a teacher's subject list, a slot's
    /// subject or a balancing override, each of which the checker forbids on a
    /// subject without interrogations). So this must simply land.
    #[test]
    fn a_rule_may_name_a_subject_without_interrogations() {
        let base = hogwarts();
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let potions = subject_by_name(base.get_data(), "Potions");
        assert!(
            base.get_data()
                .get_inner_data()
                .params
                .subjects
                .find_subject(quidditch)
                .expect("just looked up by name")
                .parameters
                .interrogation_parameters
                .is_none(),
            "the fixture's Quidditch training should run no interrogations"
        );

        let vacuous = rule(quidditch, potions, vec![]);

        let mut session = CascadeSession::new(base.clone());
        let op = PairingsUpdateOp::AddNewPairingRule(vacuous.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("a pairing rule's subjects need no interrogations of their own");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding a pairing rule returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(rule_of(state.get_data(), new_id), vacuous);
    }

    /// Rewriting a rule replaces its whole value; what it used to name is
    /// referenced by nothing it leaves behind, so again there is nothing to
    /// repair.
    #[test]
    fn updating_a_pairing_rule_rewrites_it_and_warns_about_nothing() {
        let (base, rule_id) = base_with_a_rule();
        let divination = subject_by_name(base.get_data(), "Divination");
        let second_period = period_at(base.get_data(), 1);

        // Same antecedent, another consequent, another excluded period.
        let potions = subject_by_name(base.get_data(), "Potions");
        let rewritten = rule(potions, divination, vec![second_period]);

        let mut session = CascadeSession::new(base.clone());
        let op = PairingsUpdateOp::UpdatePairingRule(rule_id, rewritten.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("the rule, the subjects and the period are all live");
        let (state, warnings) = session.commit(op.get_desc());

        assert_eq!(new_id, None, "an update creates nothing");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Pairing(PairingOp::Update(rule_id, rewritten))],
            )
            .get_data(),
        );
    }

    /// The interesting empty log: deleting a teacher takes their slots with
    /// them, but nothing at all points at a pairing rule, so the single
    /// elementary op lands alone and the document is the base minus that one
    /// entry.
    #[test]
    fn deleting_a_pairing_rule_takes_nothing_with_it() {
        let (base, rule_id) = base_with_a_rule();

        let mut session = CascadeSession::new(base.clone());
        let op = PairingsUpdateOp::DeletePairingRule(rule_id);
        let new_id = op
            .apply_to_session(&mut session)
            .expect("nothing stands in the way of removing a pairing rule");
        let (state, warnings) = session.commit(op.get_desc());

        assert_eq!(new_id, None, "a removal creates nothing");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(&base, vec![Op::Pairing(PairingOp::Remove(rule_id))]).get_data(),
        );
    }

    /// The state layer's own precheck, translated by the two ops that name an
    /// existing rule. A rejected op changes nothing and logs nothing.
    #[test]
    fn a_dead_pairing_rule_id_is_rejected_by_update_and_by_delete() {
        let base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let dangling = dangling_rule();

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            PairingsUpdateOp::UpdatePairingRule(dangling, rule(potions, metamorphose, vec![]))
                .apply_to_session(&mut session)
                .unwrap_err(),
            PairingsUpdateError::UpdatePairingRule(UpdatePairingRuleError::InvalidPairingRuleId(
                dangling
            )),
        );
        assert_eq!(
            PairingsUpdateOp::DeletePairingRule(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            PairingsUpdateError::DeletePairingRule(DeletePairingRuleError::InvalidPairingRuleId(
                dangling
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::Pairings, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// A subject id the payload made up dangles the moment the op lands. On the
    /// Add the rule went back with the rolled-back op, on the Update it is back
    /// to its old — innocent — value; either way the map finds nothing to take
    /// out, the engine convicts the op, and the scan turns the break back into
    /// the bad input it came from.
    #[test]
    fn a_dead_antecedent_subject_is_rejected_on_add_and_on_update() {
        let (base, rule_id) = base_with_a_rule();
        let potions = subject_by_name(base.get_data(), "Potions");
        let dangling = dangling_subject();

        let from_a_ghost = rule(dangling, potions, vec![]);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            PairingsUpdateOp::AddNewPairingRule(from_a_ghost.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            PairingsUpdateError::AddNewPairingRule(AddNewPairingRuleError::InvalidSubjectId(
                dangling
            )),
        );
        assert_eq!(
            PairingsUpdateOp::UpdatePairingRule(rule_id, from_a_ghost)
                .apply_to_session(&mut session)
                .unwrap_err(),
            PairingsUpdateError::UpdatePairingRule(UpdatePairingRuleError::InvalidSubjectId(
                dangling
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The second subject edge, same route — and a separate scan, because the
    /// two sites carry different payloads even though they answer with the same
    /// error variant.
    #[test]
    fn a_dead_consequent_subject_is_rejected_on_add_and_on_update() {
        let (base, rule_id) = base_with_a_rule();
        let potions = subject_by_name(base.get_data(), "Potions");
        let dangling = dangling_subject();

        let to_a_ghost = rule(potions, dangling, vec![]);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            PairingsUpdateOp::AddNewPairingRule(to_a_ghost.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            PairingsUpdateError::AddNewPairingRule(AddNewPairingRuleError::InvalidSubjectId(
                dangling
            )),
        );
        assert_eq!(
            PairingsUpdateOp::UpdatePairingRule(rule_id, to_a_ghost)
                .apply_to_session(&mut session)
                .unwrap_err(),
            PairingsUpdateError::UpdatePairingRule(UpdatePairingRuleError::InvalidSubjectId(
                dangling
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The third edge: a made-up period in the exclusion set comes back as the
    /// input it was. Worth its own fixture because this is the one dangle the
    /// map does know how to repair *in general* —
    /// [collomatique_state_colloscopes::Fix::RemovePairingRulePeriodExclusion]
    /// lifts a stale exclusion when a real period disappears. It cannot help
    /// here: the live rule (old value, or none at all after the rollback) does
    /// not exclude that period, so the arm answers `None` and the op is
    /// convicted rather than silently stripped of the exclusion the user asked
    /// for.
    #[test]
    fn a_dead_excluded_period_is_rejected_on_add_and_on_update() {
        let (base, rule_id) = base_with_a_rule();
        let potions = subject_by_name(base.get_data(), "Potions");
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let dangling = dangling_period();

        let excluding_a_ghost = rule(potions, metamorphose, vec![dangling]);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            PairingsUpdateOp::AddNewPairingRule(excluding_a_ghost.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            PairingsUpdateError::AddNewPairingRule(AddNewPairingRuleError::InvalidPeriodId(
                dangling
            )),
        );
        assert_eq!(
            PairingsUpdateOp::UpdatePairingRule(rule_id, excluding_a_ghost)
                .apply_to_session(&mut session)
                .unwrap_err(),
            PairingsUpdateError::UpdatePairingRule(UpdatePairingRuleError::InvalidPeriodId(
                dangling
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// Which break wins when a payload carries several is public API (D5): the
    /// old validator checked the antecedent subject, then the consequent
    /// subject, then the excluded periods, and the three scans are copied in
    /// that order. The set the engine hands over holds every dangle at once, so
    /// only the scan order decides — and both steps of the order are pinned,
    /// the second one by a payload whose antecedent is live.
    #[test]
    fn a_payload_naming_several_ghosts_reports_them_in_the_old_order() {
        let (base, rule_id) = base_with_a_rule();
        let potions = subject_by_name(base.get_data(), "Potions");
        let dead_antecedent = dangling_subject();
        // `PairingRule::new` forbids one subject in both parts, so a payload
        // whose two subjects are both dead needs two *distinct* dead ids.
        let dead_consequent = other_dangling_subject();

        let all_three = rule(dead_antecedent, dead_consequent, vec![dangling_period()]);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            PairingsUpdateOp::AddNewPairingRule(all_three.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            PairingsUpdateError::AddNewPairingRule(AddNewPairingRuleError::InvalidSubjectId(
                dead_antecedent
            )),
        );

        // Antecedent live, consequent and period dead: the consequent wins over
        // the period.
        let last_two = rule(potions, dead_consequent, vec![dangling_period()]);
        assert_eq!(
            PairingsUpdateOp::UpdatePairingRule(rule_id, last_two)
                .apply_to_session(&mut session)
                .unwrap_err(),
            PairingsUpdateError::UpdatePairingRule(UpdatePairingRuleError::InvalidSubjectId(
                dead_consequent
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }
}
