use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeachersUpdateOp {
    AddNewTeacher(collomatique_state_colloscopes::teachers::Teacher),
    UpdateTeacher(
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::teachers::Teacher,
    ),
    DeleteTeacher(collomatique_state_colloscopes::TeacherId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum TeachersUpdateError {
    #[error(transparent)]
    AddNewTeacher(#[from] AddNewTeacherError),
    #[error(transparent)]
    UpdateTeacher(#[from] UpdateTeacherError),
    #[error(transparent)]
    DeleteTeacher(#[from] DeleteTeacherError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewTeacherError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    /// The subject exists but runs no interrogations, and nobody can be
    /// declared to teach one — there are no colles to hold there (the
    /// checker's `TeacherSubjectWithoutInterrogations`).
    #[error("Subject ({0:?}) does not have interrogations")]
    SubjectHasNoInterrogation(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateTeacherError {
    #[error("Teacher ID {0:?} is invalid")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    /// The subject exists but runs no interrogations, and nobody can be
    /// declared to teach one — there are no colles to hold there (the
    /// checker's `TeacherSubjectWithoutInterrogations`).
    #[error("Subject ({0:?}) does not have interrogations")]
    SubjectHasNoInterrogation(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteTeacherError {
    #[error("Teacher ID {0:?} is invalid")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
}

impl TeachersUpdateOp {
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::TeacherId>, TeachersUpdateError> {
        match self {
            Self::AddNewTeacher(teacher) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Teacher(
                            collomatique_state_colloscopes::TeacherOp::Add(teacher.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Convergence, Error, FixableInvariant, Reference, SubjectRefSite,
                        };
                        match &e {
                            // Whatever the cascade could repair is repaired by
                            // the time this arm runs, so what is left was caused
                            // by the Add's own payload: the teacher it names went
                            // back with the rolled-back op, the map finds nothing
                            // to remove, and the target is convicted.
                            //
                            // The pre-op state was valid, so any teacher->subject
                            // dangle in the set was introduced by this Add; the
                            // dangling target is the bad input subject id.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::TeacherSubjects(_),
                                    }) = inv
                                    {
                                        return AddNewTeacherError::InvalidSubjectId(*target);
                                    }
                                }
                                // And a live subject is not enough either: a
                                // subject whose interrogations are disabled has
                                // no colles to hold, so nobody may be declared to
                                // teach it. Scanned after the dangles, so a
                                // payload naming both keeps reporting the
                                // dangle first, as the old validator did.
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::TeacherSubjectWithoutInterrogations(
                                            _teacher,
                                            subject,
                                        ),
                                    ) = inv
                                    {
                                        return AddNewTeacherError::SubjectHasNoInterrogation(
                                            *subject,
                                        );
                                    }
                                }
                                panic!("Unexpected invariant breaks during AddNewTeacher: {set:?}");
                            }
                            _ => panic!("Unexpected error during AddNewTeacher: {e:?}"),
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::TeacherId(new_id)) = result else {
                    panic!("Unexpected result from TeacherOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateTeacher(teacher_id, teacher) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Teacher(
                            collomatique_state_colloscopes::TeacherOp::Update(
                                *teacher_id,
                                teacher.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Convergence, Error, FixableInvariant, InvalidOp, PrecheckError,
                            Reference, SubjectRefSite, TeacherPrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Teacher(te))) => {
                                match te {
                                    TeacherPrecheckError::InvalidTeacherId(id) => {
                                        UpdateTeacherError::InvalidTeacherId(*id)
                                    }
                                    TeacherPrecheckError::TeacherIdAlreadyExists(_) => panic!(
                                        "Unexpected TeacherPrecheckError during UpdateTeacher: {e:?}"
                                    ),
                                }
                            }
                            Error::BrokenInvariants(set) => {
                                // Old validator order kept: validate_teacher
                                // (subject ids) is what fires first, so a payload
                                // naming a made-up subject reports that, whatever
                                // else the new list breaks.
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Subject {
                                        target,
                                        site: SubjectRefSite::TeacherSubjects(_),
                                    }) = inv
                                    {
                                        return UpdateTeacherError::InvalidSubjectId(*target);
                                    }
                                }
                                // Same second scan as `AddNewTeacher`: a live
                                // subject with interrogations disabled cannot be
                                // taught either, and the map cannot repair the
                                // pair — the teacher in the state still holds
                                // their old subject list, so there is nothing
                                // there to take out.
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::TeacherSubjectWithoutInterrogations(
                                            _teacher,
                                            subject,
                                        ),
                                    ) = inv
                                    {
                                        return UpdateTeacherError::SubjectHasNoInterrogation(
                                            *subject,
                                        );
                                    }
                                }
                                // The old body's own second scan is gone with the
                                // cleaning: a slot whose teacher no longer teaches
                                // its subject is repaired by the cascade
                                // (`SlotTeacherDoesNotTeachSubject` -> the slot is
                                // removed), never returned here.
                                panic!("Unexpected invariant breaks during UpdateTeacher: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateTeacher: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteTeacher(teacher_id) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Teacher(
                            collomatique_state_colloscopes::TeacherOp::Remove(*teacher_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, InvalidOp, PrecheckError, TeacherPrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Teacher(te))) => {
                                match te {
                                    TeacherPrecheckError::InvalidTeacherId(id) => {
                                        DeleteTeacherError::InvalidTeacherId(*id)
                                    }
                                    TeacherPrecheckError::TeacherIdAlreadyExists(_) => panic!(
                                        "Unexpected TeacherPrecheckError during DeleteTeacher: {e:?}"
                                    ),
                                }
                            }
                            // The old `BrokenInvariants` arm is gone: a teacher
                            // whose slots reference them is no longer an error at
                            // all, the cascade removes the slots (and whatever
                            // dangles from *them*) and logs each removal.
                            _ => panic!("Unexpected error during DeleteTeacher: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Teachers,
            match self {
                TeachersUpdateOp::AddNewTeacher(_desc) => "Ajouter un colleur".into(),
                TeachersUpdateOp::UpdateTeacher(_id, _desc) => "Modifier un colleur".into(),
                TeachersUpdateOp::DeleteTeacher(_id) => "Supprimer un colleur".into(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //! A teacher is referenced by the slots they hold, and references the
    //! subjects they teach — so this is the first family whose ops make the
    //! cascade work, and the fixtures run on the frozen hogwarts base rather
    //! than on a document built here: it already has teachers with several
    //! slots, and slot pairing rules over those slots, which is a two-level
    //! cascade nobody had to assemble by hand.
    //!
    //! Ids are looked up by name: the fixture is frozen, but a test that says
    //! « Bibine » says what it means.
    //!
    //! What the fixtures pin, family-wide: the repairs the cascade had to make
    //! (as the exact [Fix] list, in application order) *and* the document they
    //! produced (rebuilt by replaying those very ops on the base), plus the
    //! whole error surface — the two prechecks the state layer answers, the
    //! dangling-subject scan the payload causes, and the no-interrogation
    //! rejection that replaces a crash.

    use super::*;
    use crate::test_utils::{fixes, hogwarts};
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        Fix, Op, PersonWithContact, SlotOp, SlotPairingOp, TeacherOp,
        ids::{Id, SlotId, SlotPairingRuleId, SubjectId, TeacherId},
        teachers::Teacher,
    };
    use std::collections::BTreeSet;

    fn teacher_by_surname(data: &Data, surname: &str) -> TeacherId {
        data.get_inner_data()
            .params
            .teachers
            .teacher_map
            .iter()
            .find(|(_id, teacher)| teacher.desc.surname == surname)
            .map(|(id, _teacher)| id)
            .unwrap_or_else(|| panic!("the fixture should have a teacher named {surname}"))
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

    fn teacher_of(data: &Data, teacher: TeacherId) -> Teacher {
        data.get_inner_data()
            .params
            .teachers
            .teacher_map
            .get(&teacher)
            .expect("the fixture's teacher should be live")
            .clone()
    }

    /// The teacher's slots, id-ordered — which is the order the cascade meets
    /// them, since the invariant the engine picks first is the one naming the
    /// first slot.
    fn slots_of(data: &Data, teacher: TeacherId) -> Vec<SlotId> {
        let mut slots: Vec<_> = data
            .get_inner_data()
            .params
            .slots
            .all_slots()
            .filter(|(_id, slot)| slot.teacher_id == teacher)
            .map(|(id, _slot)| *id)
            .collect();
        slots.sort();

        slots
    }

    /// The teacher's slots in one subject only — what dropping that subject
    /// from their list costs them.
    fn slots_of_in_subject(data: &Data, teacher: TeacherId, subject: SubjectId) -> Vec<SlotId> {
        let mut slots: Vec<_> = data
            .get_inner_data()
            .params
            .slots
            .slots_for_subject(subject)
            .into_iter()
            .flatten()
            .filter(|(_id, slot)| slot.teacher_id == teacher)
            .map(|(id, _slot)| *id)
            .collect();
        slots.sort();

        slots
    }

    /// The slot pairing rules naming any of `slots` — the second level of the
    /// cascade a teacher removal sets off.
    fn slot_pairing_rules_over(data: &Data, slots: &[SlotId]) -> Vec<SlotPairingRuleId> {
        data.get_inner_data()
            .params
            .slot_pairings
            .slot_pairing_rule_map
            .iter()
            .filter(|(_id, rule)| {
                slots.contains(&rule.antecedent().slot_id)
                    || slots.contains(&rule.consequent().slot_id)
            })
            .map(|(id, _rule)| id)
            .collect()
    }

    fn new_teacher(subjects: BTreeSet<SubjectId>) -> Teacher {
        Teacher {
            desc: PersonWithContact {
                surname: "Rusard".into(),
                firstname: "Argus".into(),
                tel: None,
                email: None,
            },
            subjects,
        }
    }

    /// An id no document ever issued.
    fn dangling_teacher() -> TeacherId {
        unsafe { TeacherId::new(1u64 << 40) }
    }

    fn dangling_subject() -> SubjectId {
        unsafe { SubjectId::new(1u64 << 40) }
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects the cascade to have landed —
    /// each of them valid in that order, exactly as the cascade lands them.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::Teachers, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// A new teacher references only subjects, so nothing in the document can
    /// need repairing: the id comes back and the warning log stays empty.
    #[test]
    fn adding_a_teacher_creates_them_and_warns_about_nothing() {
        let base = hogwarts();
        let potions = subject_by_name(base.get_data(), "Potions");

        let mut session = CascadeSession::new(base.clone());
        let op = TeachersUpdateOp::AddNewTeacher(new_teacher(BTreeSet::from([potions])));
        let new_id = op
            .apply_to_session(&mut session)
            .expect("a live subject is all this teacher names");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding a teacher returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            teacher_of(state.get_data(), new_id),
            new_teacher(BTreeSet::from([potions])),
        );
    }

    /// Removing Mme Bibine cannot land alone: her two slots reference her, and
    /// one slot pairing rule references *both* her slots. The engine unwinds
    /// depth-first, so the rule — the deepest repair — lands first, then the
    /// slot that dragged it in, then the second slot, and finally the teacher.
    ///
    /// Where the old world refused the removal until its own cleaning loop had
    /// emptied the way, the family op now issues the single elementary removal
    /// and the cascade clears it, reporting what it cost.
    #[test]
    fn deleting_a_teacher_takes_their_slots_and_reports_them() {
        let base = hogwarts();
        let bibine = teacher_by_surname(base.get_data(), "Bibine");
        let slots = slots_of(base.get_data(), bibine);
        let rules = slot_pairing_rules_over(base.get_data(), &slots);
        assert_eq!(slots.len(), 2, "the fixture's Bibine should have two slots");
        assert_eq!(
            rules.len(),
            1,
            "the fixture should pair Bibine's two slots by exactly one rule"
        );

        let mut session = CascadeSession::new(base.clone());
        let op = TeachersUpdateOp::DeleteTeacher(bibine);
        let new_id = op
            .apply_to_session(&mut session)
            .expect("the cascade clears the way for the removal");
        let (state, warnings) = session.commit(op.get_desc());

        assert_eq!(new_id, None, "a removal creates nothing");
        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::DeleteSlotPairingRule { rule: rules[0] },
                Fix::DeleteSlot { slot: slots[0] },
                Fix::DeleteSlot { slot: slots[1] },
            ],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::SlotPairing(SlotPairingOp::Remove(rules[0])),
                    Op::Slot(SlotOp::Remove(slots[0])),
                    Op::Slot(SlotOp::Remove(slots[1])),
                    Op::Teacher(TeacherOp::Remove(bibine)),
                ],
            )
            .get_data(),
        );
    }

    /// Rogue teaches Potions and Potions - TP, with slots in both. Taking
    /// Potions - TP off his list makes the slots he holds there impossible —
    /// a slot's teacher must teach its subject — so the cascade removes
    /// exactly those, and leaves his Potions slots alone.
    #[test]
    fn dropping_a_subject_takes_the_slots_the_teacher_held_in_it() {
        let base = hogwarts();
        let rogue = teacher_by_surname(base.get_data(), "Rogue");
        let potions = subject_by_name(base.get_data(), "Potions");
        let potions_tp = subject_by_name(base.get_data(), "Potions - TP");
        let lost = slots_of_in_subject(base.get_data(), rogue, potions_tp);
        let kept = slots_of_in_subject(base.get_data(), rogue, potions);
        assert_eq!(lost.len(), 2, "Rogue should hold two Potions - TP slots");
        assert!(!kept.is_empty(), "Rogue should keep his Potions slots");

        let mut rogue_teaches_only_potions = teacher_of(base.get_data(), rogue);
        rogue_teaches_only_potions.subjects = BTreeSet::from([potions]);

        let mut session = CascadeSession::new(base.clone());
        let op = TeachersUpdateOp::UpdateTeacher(rogue, rogue_teaches_only_potions.clone());
        op.apply_to_session(&mut session)
            .expect("the cascade removes the slots the dropped subject leaves behind");
        let (state, warnings) = session.commit(op.get_desc());

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::DeleteSlot { slot: lost[0] },
                Fix::DeleteSlot { slot: lost[1] },
            ],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::Slot(SlotOp::Remove(lost[0])),
                    Op::Slot(SlotOp::Remove(lost[1])),
                    Op::Teacher(TeacherOp::Update(rogue, rogue_teaches_only_potions)),
                ],
            )
            .get_data(),
        );
    }

    /// The state layer's own precheck, translated by the two ops that name an
    /// existing teacher. A rejected op changes nothing and logs nothing: the
    /// engine put the document back before the error came out.
    #[test]
    fn a_dead_teacher_id_is_rejected_by_update_and_by_delete() {
        let base = hogwarts();
        let dangling = dangling_teacher();

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            TeachersUpdateOp::UpdateTeacher(dangling, new_teacher(BTreeSet::new()))
                .apply_to_session(&mut session)
                .unwrap_err(),
            TeachersUpdateError::UpdateTeacher(UpdateTeacherError::InvalidTeacherId(dangling)),
        );
        assert_eq!(
            TeachersUpdateOp::DeleteTeacher(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            TeachersUpdateError::DeleteTeacher(DeleteTeacherError::InvalidTeacherId(dangling)),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::Teachers, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// A subject id the payload made up dangles the moment the op lands. No
    /// state holds that subject, so no repair can help — the map answers
    /// nothing, the engine convicts the op, and the scan turns the break back
    /// into the bad input it came from.
    #[test]
    fn a_dead_subject_id_is_rejected_on_add_and_on_update() {
        let base = hogwarts();
        let rogue = teacher_by_surname(base.get_data(), "Rogue");
        let dangling = dangling_subject();

        let mut rogue_teaches_a_ghost = teacher_of(base.get_data(), rogue);
        rogue_teaches_a_ghost.subjects.insert(dangling);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            TeachersUpdateOp::AddNewTeacher(new_teacher(BTreeSet::from([dangling])))
                .apply_to_session(&mut session)
                .unwrap_err(),
            TeachersUpdateError::AddNewTeacher(AddNewTeacherError::InvalidSubjectId(dangling)),
        );
        assert_eq!(
            TeachersUpdateOp::UpdateTeacher(rogue, rogue_teaches_a_ghost)
                .apply_to_session(&mut session)
                .unwrap_err(),
            TeachersUpdateError::UpdateTeacher(UpdateTeacherError::InvalidSubjectId(dangling)),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// Quidditch training is a subject with interrogations *disabled*, and
    /// nobody may be declared to teach one: there are no colles to hold there
    /// (the checker's `TeacherSubjectWithoutInterrogations`). The subject is
    /// live, so the op reaches the state layer and gets convicted — the map
    /// answers nothing, since the pair it would take out went back with the
    /// rolled-back op. The user must get a typed rejection out of that, not a
    /// dead process: this is the teacher half of the crash fixed for balancing.
    #[test]
    fn a_subject_without_interrogations_cannot_be_taught() {
        let base = hogwarts();
        let rogue = teacher_by_surname(base.get_data(), "Rogue");
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

        let mut rogue_also_coaches = teacher_of(base.get_data(), rogue);
        rogue_also_coaches.subjects.insert(quidditch);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            TeachersUpdateOp::AddNewTeacher(new_teacher(BTreeSet::from([quidditch])))
                .apply_to_session(&mut session)
                .unwrap_err(),
            TeachersUpdateError::AddNewTeacher(AddNewTeacherError::SubjectHasNoInterrogation(
                quidditch
            )),
        );
        assert_eq!(
            TeachersUpdateOp::UpdateTeacher(rogue, rogue_also_coaches)
                .apply_to_session(&mut session)
                .unwrap_err(),
            TeachersUpdateError::UpdateTeacher(UpdateTeacherError::SubjectHasNoInterrogation(
                quidditch
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }
}
