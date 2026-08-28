use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncompatibilitiesUpdateOp {
    AddNewIncompat(collomatique_state_colloscopes::incompats::Incompatibility),
    DeleteIncompat(collomatique_state_colloscopes::IncompatId),
    UpdateIncompat(
        collomatique_state_colloscopes::IncompatId,
        collomatique_state_colloscopes::incompats::Incompatibility,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum IncompatibilitiesUpdateError {
    #[error(transparent)]
    AddNewIncompat(#[from] AddNewIncompatError),
    #[error(transparent)]
    DeleteIncompat(#[from] DeleteIncompatError),
    #[error(transparent)]
    UpdateIncompat(#[from] UpdateIncompatError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewIncompatError {
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteIncompatError {
    #[error("invalid incompat id ({0:?})")]
    InvalidIncompatId(collomatique_state_colloscopes::IncompatId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateIncompatError {
    #[error("invalid incompat id ({0:?})")]
    InvalidIncompatId(collomatique_state_colloscopes::IncompatId),
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

impl IncompatibilitiesUpdateOp {
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::IncompatId>, IncompatibilitiesUpdateError>
    {
        match self {
            Self::AddNewIncompat(incompat) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Add(incompat.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, Reference, SubjectRefSite, WeekPatternRefSite,
                        };
                        match &e {
                            // Whatever the cascade could repair is repaired by
                            // the time this arm runs, so what is left was caused
                            // by the Add's own payload: the incompat it names
                            // went back with the rolled-back op, the map finds
                            // nothing to take out, and the target is convicted.
                            //
                            // The pre-op state was valid, so any dangle in the
                            // set was introduced by this Add. Old validator
                            // order: subject id before week pattern id.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::IncompatSubject(_),
                                    }) = inv
                                    {
                                        return AddNewIncompatError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::WeekPattern {
                                        target,
                                        site: WeekPatternRefSite::IncompatWeekPattern(_),
                                    }) = inv
                                    {
                                        return AddNewIncompatError::InvalidWeekPatternId(*target);
                                    }
                                }
                                // Nothing else can break: the subject an
                                // incompat names is deliberately *not* required
                                // to run interrogations of its own (the whole
                                // point of the edge — see
                                // `incompats::Incompatibility::subject_id`), so
                                // this family has no convergence to answer for.
                                panic!(
                                    "Unexpected invariant breaks during AddNewIncompat: {set:?}"
                                );
                            }
                            _ => panic!("Unexpected error during AddNewIncompat: {e:?}"),
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::IncompatId(new_id)) = result else {
                    panic!("Unexpected result from IncompatOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateIncompat(incompat_id, incompat) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Update(
                                *incompat_id,
                                incompat.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, IncompatPrecheckError, InvalidOp,
                            PrecheckError, Reference, SubjectRefSite, WeekPatternRefSite,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Incompat(ie))) => {
                                match ie {
                                    IncompatPrecheckError::InvalidIncompatId(id) => {
                                        UpdateIncompatError::InvalidIncompatId(*id)
                                    }
                                    IncompatPrecheckError::IncompatIdAlreadyExists(_) => panic!(
                                        "Unexpected IncompatPrecheckError during UpdateIncompat: {e:?}"
                                    ),
                                }
                            }
                            // Same two scans as the Add, and for the same
                            // reason: the payload is the only thing that can
                            // dangle here. Old validator order kept: subject id
                            // before week pattern id.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::IncompatSubject(_),
                                    }) = inv
                                    {
                                        return UpdateIncompatError::InvalidSubjectId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::WeekPattern {
                                        target,
                                        site: WeekPatternRefSite::IncompatWeekPattern(_),
                                    }) = inv
                                    {
                                        return UpdateIncompatError::InvalidWeekPatternId(*target);
                                    }
                                }
                                panic!(
                                    "Unexpected invariant breaks during UpdateIncompat: {set:?}"
                                );
                            }
                            _ => panic!("Unexpected error during UpdateIncompat: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteIncompat(incompat_id) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Incompat(
                            collomatique_state_colloscopes::IncompatOp::Remove(*incompat_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, IncompatPrecheckError, InvalidOp, PrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Incompat(ie))) => {
                                match ie {
                                    IncompatPrecheckError::InvalidIncompatId(id) => {
                                        DeleteIncompatError::InvalidIncompatId(*id)
                                    }
                                    IncompatPrecheckError::IncompatIdAlreadyExists(_) => panic!(
                                        "Unexpected IncompatPrecheckError during DeleteIncompat: {e:?}"
                                    ),
                                }
                            }
                            // There is no invariant arm here and there never
                            // was: nothing in the document points at an
                            // incompat — no `Reference` variant carries an
                            // `IncompatId` — so a removal breaks nothing and
                            // the cascade has nothing to repair.
                            _ => panic!("Unexpected error during DeleteIncompat: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Incompatibilities,
            match self {
                IncompatibilitiesUpdateOp::AddNewIncompat(_) => {
                    "Ajouter une incompatibilité horaire".into()
                }
                IncompatibilitiesUpdateOp::DeleteIncompat(_) => {
                    "Supprimer une incompatibilité horaire".into()
                }
                IncompatibilitiesUpdateOp::UpdateIncompat(_, _) => {
                    "Modifier une incompatibilité horaire".into()
                }
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //! The family sits at the leaf of the reference graph: an incompat points
    //! at a subject and (maybe) at a week pattern, and *nothing* points back at
    //! it — no [collomatique_state_colloscopes::Reference] variant carries an
    //! `IncompatId`. So no op of this family ever makes the cascade repair
    //! anything, and every fixture below asserts an empty warning log; what is
    //! worth pinning is the error surface, and the fact that the log stays
    //! empty even for the removal.
    //!
    //! The fixtures run on the frozen hogwarts base, which already holds what
    //! the family needs: six incompats, two week patterns, and — the point of
    //! the asymmetry fixture — subjects that run no interrogations at all.
    //! Payloads are built by cloning an incompat out of the base and changing
    //! the one field the test is about, so a test says what it changes.

    use super::*;
    use crate::test_utils::hogwarts;
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        IncompatOp, Op,
        ids::{Id, IncompatId, SubjectId, WeekPatternId},
        incompats::Incompatibility,
    };

    fn incompat_by_name(data: &Data, name: &str) -> IncompatId {
        data.get_inner_data()
            .params
            .incompats
            .incompat_map
            .iter()
            .find(|(_id, incompat)| incompat.name == name)
            .map(|(id, _incompat)| id)
            .unwrap_or_else(|| panic!("the fixture should have an incompat named {name}"))
    }

    fn incompat_of(data: &Data, incompat: IncompatId) -> Incompatibility {
        data.get_inner_data()
            .params
            .incompats
            .incompat_map
            .get(&incompat)
            .expect("the fixture's incompat should be live")
            .clone()
    }

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

    fn week_pattern_by_name(data: &Data, name: &str) -> WeekPatternId {
        data.get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .iter()
            .find(|(_id, pattern)| pattern.name == name)
            .map(|(id, _pattern)| id)
            .unwrap_or_else(|| panic!("the fixture should have a week pattern named {name}"))
    }

    /// Ids no document ever issued.
    fn dangling_incompat() -> IncompatId {
        unsafe { IncompatId::new(1u64 << 40) }
    }

    fn dangling_subject() -> SubjectId {
        unsafe { SubjectId::new(1u64 << 40) }
    }

    fn dangling_week_pattern() -> WeekPatternId {
        unsafe { WeekPatternId::new(1u64 << 40) }
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects to have landed.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::Incompatibilities, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// An incompat names only material that already exists, so nothing in the
    /// document can need repairing: the id comes back and the log stays empty.
    #[test]
    fn adding_an_incompat_creates_it_and_warns_about_nothing() {
        let base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");
        let even_weeks = week_pattern_by_name(base.get_data(), "Semaines paires");

        let mut new_incompat = incompat_of(
            base.get_data(),
            incompat_by_name(base.get_data(), "Lundi Midi"),
        );
        new_incompat.name = "Lundi Midi (potions)".into();
        new_incompat.subject_id = potions;
        new_incompat.week_pattern_id = Some(even_weeks);

        let mut session = CascadeSession::new(base.clone());
        let op = IncompatibilitiesUpdateOp::AddNewIncompat(new_incompat.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("the subject and the week pattern are both live");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding an incompat returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(incompat_of(state.get_data(), new_id), new_incompat);
    }

    /// The edge this family is deliberately loose about: unlike a teacher, a
    /// slot, a balancing override or a group-list association, an incompat may
    /// name a subject that runs **no interrogations** — the subject is there to
    /// carry a schedule that blocks colles elsewhere, not to have colles of its
    /// own (`incompats::Incompatibility::subject_id`). The teacher family gets
    /// a typed rejection for exactly this shape; here it must simply land.
    #[test]
    fn an_incompat_may_name_a_subject_without_interrogations() {
        let base = hogwarts();
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
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

        let mut on_quidditch = incompat_of(
            base.get_data(),
            incompat_by_name(base.get_data(), "Lundi Midi"),
        );
        on_quidditch.name = "Lundi Midi (quidditch)".into();
        on_quidditch.subject_id = quidditch;

        let mut session = CascadeSession::new(base.clone());
        let op = IncompatibilitiesUpdateOp::AddNewIncompat(on_quidditch.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("an incompat's subject needs no interrogations of its own");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding an incompat returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(incompat_of(state.get_data(), new_id), on_quidditch);
    }

    /// Rewriting an incompat replaces its whole value; the material it used to
    /// name is not referenced by anything else it leaves behind, so again there
    /// is nothing to repair.
    #[test]
    fn updating_an_incompat_rewrites_it_and_warns_about_nothing() {
        let base = hogwarts();
        let tuesday = incompat_by_name(base.get_data(), "Mardi Midi");
        let odd_weeks = week_pattern_by_name(base.get_data(), "Semaines impaires");

        let mut every_other_tuesday = incompat_of(base.get_data(), tuesday);
        every_other_tuesday.week_pattern_id = Some(odd_weeks);

        let mut session = CascadeSession::new(base.clone());
        let op = IncompatibilitiesUpdateOp::UpdateIncompat(tuesday, every_other_tuesday.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("the incompat and the week pattern are both live");
        let (state, warnings) = session.commit(op.get_desc());

        assert_eq!(new_id, None, "an update creates nothing");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Incompat(IncompatOp::Update(
                    tuesday,
                    every_other_tuesday
                ))],
            )
            .get_data(),
        );
    }

    /// The removal is the interesting empty log: deleting a teacher takes their
    /// slots with them, but nothing at all points at an incompat, so the single
    /// elementary op lands alone and the document is the base minus that one
    /// entry.
    #[test]
    fn deleting_an_incompat_takes_nothing_with_it() {
        let base = hogwarts();
        let wednesday = incompat_by_name(base.get_data(), "Mercredi Midi");

        let mut session = CascadeSession::new(base.clone());
        let op = IncompatibilitiesUpdateOp::DeleteIncompat(wednesday);
        let new_id = op
            .apply_to_session(&mut session)
            .expect("nothing stands in the way of removing an incompat");
        let (state, warnings) = session.commit(op.get_desc());

        assert_eq!(new_id, None, "a removal creates nothing");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(&base, vec![Op::Incompat(IncompatOp::Remove(wednesday))]).get_data(),
        );
    }

    /// The state layer's own precheck, translated by the two ops that name an
    /// existing incompat. A rejected op changes nothing and logs nothing.
    #[test]
    fn a_dead_incompat_id_is_rejected_by_update_and_by_delete() {
        let base = hogwarts();
        let dangling = dangling_incompat();
        let monday = incompat_of(
            base.get_data(),
            incompat_by_name(base.get_data(), "Lundi Midi"),
        );

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            IncompatibilitiesUpdateOp::UpdateIncompat(dangling, monday)
                .apply_to_session(&mut session)
                .unwrap_err(),
            IncompatibilitiesUpdateError::UpdateIncompat(UpdateIncompatError::InvalidIncompatId(
                dangling
            )),
        );
        assert_eq!(
            IncompatibilitiesUpdateOp::DeleteIncompat(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            IncompatibilitiesUpdateError::DeleteIncompat(DeleteIncompatError::InvalidIncompatId(
                dangling
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::Incompatibilities, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// A subject id the payload made up dangles the moment the op lands. No
    /// state holds that subject, so no repair can help — the map answers
    /// nothing, the engine convicts the op, and the scan turns the break back
    /// into the bad input it came from.
    #[test]
    fn a_dead_subject_id_is_rejected_on_add_and_on_update() {
        let base = hogwarts();
        let monday = incompat_by_name(base.get_data(), "Lundi Midi");
        let dangling = dangling_subject();

        let mut on_a_ghost_subject = incompat_of(base.get_data(), monday);
        on_a_ghost_subject.subject_id = dangling;

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            IncompatibilitiesUpdateOp::AddNewIncompat(on_a_ghost_subject.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            IncompatibilitiesUpdateError::AddNewIncompat(AddNewIncompatError::InvalidSubjectId(
                dangling
            )),
        );
        assert_eq!(
            IncompatibilitiesUpdateOp::UpdateIncompat(monday, on_a_ghost_subject)
                .apply_to_session(&mut session)
                .unwrap_err(),
            IncompatibilitiesUpdateError::UpdateIncompat(UpdateIncompatError::InvalidSubjectId(
                dangling
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The second edge, same route: a made-up week pattern id dangles from the
    /// payload and comes back as the input it was.
    #[test]
    fn a_dead_week_pattern_id_is_rejected_on_add_and_on_update() {
        let base = hogwarts();
        let monday = incompat_by_name(base.get_data(), "Lundi Midi");
        let dangling = dangling_week_pattern();

        let mut on_a_ghost_pattern = incompat_of(base.get_data(), monday);
        on_a_ghost_pattern.week_pattern_id = Some(dangling);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            IncompatibilitiesUpdateOp::AddNewIncompat(on_a_ghost_pattern.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            IncompatibilitiesUpdateError::AddNewIncompat(
                AddNewIncompatError::InvalidWeekPatternId(dangling)
            ),
        );
        assert_eq!(
            IncompatibilitiesUpdateOp::UpdateIncompat(monday, on_a_ghost_pattern)
                .apply_to_session(&mut session)
                .unwrap_err(),
            IncompatibilitiesUpdateError::UpdateIncompat(
                UpdateIncompatError::InvalidWeekPatternId(dangling)
            ),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// Which break wins when a payload carries both is public API: the old
    /// validator checked the subject id before the week pattern id, and the two
    /// scans are copied in that order. The set the engine hands over holds both
    /// dangles at once, so only the scan order decides.
    #[test]
    fn a_payload_naming_two_ghosts_reports_the_subject_first() {
        let base = hogwarts();
        let monday = incompat_by_name(base.get_data(), "Lundi Midi");
        let dead_subject = dangling_subject();

        let mut on_two_ghosts = incompat_of(base.get_data(), monday);
        on_two_ghosts.subject_id = dead_subject;
        on_two_ghosts.week_pattern_id = Some(dangling_week_pattern());

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            IncompatibilitiesUpdateOp::AddNewIncompat(on_two_ghosts.clone())
                .apply_to_session(&mut session)
                .unwrap_err(),
            IncompatibilitiesUpdateError::AddNewIncompat(AddNewIncompatError::InvalidSubjectId(
                dead_subject
            )),
        );
        assert_eq!(
            IncompatibilitiesUpdateOp::UpdateIncompat(monday, on_two_ghosts)
                .apply_to_session(&mut session)
                .unwrap_err(),
            IncompatibilitiesUpdateError::UpdateIncompat(UpdateIncompatError::InvalidSubjectId(
                dead_subject
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }
}
