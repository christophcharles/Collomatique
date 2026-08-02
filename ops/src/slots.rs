use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotsUpdateOp {
    AddNewSlot(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::slots::Slot,
    ),
    UpdateSlot(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::slots::Slot,
    ),
    DeleteSlot(collomatique_state_colloscopes::SlotId),
    MoveSlotUp(collomatique_state_colloscopes::SlotId),
    MoveSlotDown(collomatique_state_colloscopes::SlotId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotsUpdateError {
    #[error(transparent)]
    AddNewSlot(#[from] AddNewSlotError),
    #[error(transparent)]
    UpdateSlot(#[from] UpdateSlotError),
    #[error(transparent)]
    DeleteSlot(#[from] DeleteSlotError),
    #[error(transparent)]
    MoveSlotUp(#[from] MoveSlotUpError),
    #[error(transparent)]
    MoveSlotDown(#[from] MoveSlotDownError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewSlotError {
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Subject ({0:?}) does not have interrogations")]
    SubjectHasNoInterrogation(collomatique_state_colloscopes::SubjectId),
    #[error("Teacher id ({0:?}) is invalid")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
    #[error("Week pattern id ({0:?}) is invalid")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
    #[error("Provided teacher ({0:?}) does not teach in subject ({1:?})")]
    TeacherDoesNotTeachInSubject(
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::SubjectId,
    ),
    #[error("The slot start time is too late and the slot overlaps with the next day")]
    SlotOverlapsWithNextDay,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateSlotError {
    #[error("Slot id ({0:?}) is invalid")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
    #[error("Subject ({0:?}) does not have interrogations")]
    SubjectHasNoInterrogation(collomatique_state_colloscopes::SubjectId),
    #[error("Teacher id ({0:?}) is invalid")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
    #[error("Week pattern id ({0:?}) is invalid")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
    #[error("Provided teacher ({0:?}) does not teach in subject ({1:?})")]
    TeacherDoesNotTeachInSubject(
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::SubjectId,
    ),
    #[error("The slot start time is too late and the slot overlaps with the next day")]
    SlotOverlapsWithNextDay,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteSlotError {
    #[error("Slot ID {0:?} is invalid")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveSlotUpError {
    #[error("Slot ID {0:?} is invalid")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
    #[error("Slot is already the first slot")]
    NoUpperPosition,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveSlotDownError {
    #[error("Slot ID {0:?} is invalid")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
    #[error("Slot is already the last slot")]
    NoLowerPosition,
}

impl SlotsUpdateOp {
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::SlotId>, SlotsUpdateError> {
        match self {
            Self::AddNewSlot(subject_id, slot) => {
                if session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject_position(*subject_id)
                    .is_none()
                {
                    return Err(AddNewSlotError::InvalidSubjectId(*subject_id).into());
                }
                // The sparse slots ordering no longer tracks "subject has
                // interrogations" (a row only appears once a slot exists);
                // consult the subject's interrogation parameters directly.
                if !session
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .is_some_and(|subject| subject.parameters.interrogation_parameters.is_some())
                {
                    return Err(AddNewSlotError::SubjectHasNoInterrogation(*subject_id).into());
                }

                let last_slot_id = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .last_slot_id_for_subject(*subject_id);

                // The state op takes the subject from the slot itself.
                let mut slot = slot.clone();
                slot.subject_id = *subject_id;

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Slot(
                            collomatique_state_colloscopes::SlotOp::AddAfter(last_slot_id, slot),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Convergence, Error, FixableInvariant, Reference, TeacherRefSite,
                            WeekPatternRefSite,
                        };
                        match &e {
                            // Nothing here can be repaired, and every arm below
                            // says the same thing in its own way: a fresh slot
                            // goes back with the rolled-back op, so the map
                            // finds no slot at the id the break names, answers
                            // nothing, and the engine convicts the target.
                            //
                            // Subject validity and interrogation presence are
                            // pre-checked in the ops layer above, so those two
                            // guards are unreachable here (their breaks would fall
                            // through to the panic). The pre-op state was valid, so
                            // every break in the set was introduced by this add.
                            // Old validator order (validate_slot_internal):
                            // teacher-resolves, teacher-teaches, week-pattern, then
                            // day overflow.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Teacher {
                                        target,
                                        site: TeacherRefSite::SlotTeacher(_),
                                    }) = inv
                                    {
                                        return AddNewSlotError::InvalidTeacherId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotTeacherDoesNotTeachSubject(
                                            _,
                                            teacher,
                                            subject,
                                        ),
                                    ) = inv
                                    {
                                        return AddNewSlotError::TeacherDoesNotTeachInSubject(
                                            *teacher, *subject,
                                        );
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::WeekPattern {
                                        target,
                                        site: WeekPatternRefSite::SlotWeekPattern(_),
                                    }) = inv
                                    {
                                        return AddNewSlotError::InvalidWeekPatternId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotOverflowsDay { .. },
                                    ) = inv
                                    {
                                        return AddNewSlotError::SlotOverlapsWithNextDay;
                                    }
                                }
                                panic!("Unexpected invariant breaks during AddNewSlot: {set:?}");
                            }
                            _ => panic!("Unexpected error during AddNewSlot: {e:?}"),
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::SlotId(new_id)) = result else {
                    panic!("Unexpected result from SlotOp::AddAfter");
                };
                Ok(Some(new_id))
            }
            Self::UpdateSlot(slot_id, slot) => {
                // A slot cannot change subject, so pin the new slot to its
                // current subject (the incoming slot's subject is not carried
                // by the UI/glue layers). An invalid slot id is reported by the
                // state op itself.
                let mut slot = slot.clone();
                if let Some((subject_id, _pos)) = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                {
                    slot.subject_id = subject_id;
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Slot(
                            collomatique_state_colloscopes::SlotOp::Update(*slot_id, slot),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Convergence, Error, FixableInvariant, InvalidOp, PrecheckError,
                            Reference, SlotPrecheckError, TeacherRefSite, WeekPatternRefSite,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Slot(pe))) => {
                                match pe {
                                    SlotPrecheckError::InvalidSlotId(id) => {
                                        UpdateSlotError::InvalidSlotId(*id)
                                    }
                                    // The subject is pinned to the slot's current
                                    // subject above, so CannotChangeSubject cannot
                                    // arise here; the other three are add/move-only.
                                    SlotPrecheckError::SlotIdAlreadyExists(_)
                                    | SlotPrecheckError::PositionOutOfBounds { .. }
                                    | SlotPrecheckError::PreviousSlotIsNotInRightSubject(_, _)
                                    | SlotPrecheckError::CannotChangeSubject(_, _, _) => {
                                        panic!("Unexpected slot precheck during UpdateSlot: {e:?}")
                                    }
                                }
                            }
                            // The pre-op state was valid, so every break in the set
                            // was introduced by this update, and none of them is
                            // the cascade's to repair: the slot went back to its
                            // old self with the rolled-back op, so every arm of
                            // the map that could name it tests the old field and
                            // finds the slot innocent. Old validator order
                            // (validate_slot_internal): teacher-resolves,
                            // teacher-teaches, week-pattern, subject-has-
                            // interrogations, then day overflow. (InvalidSubjectId
                            // sits between week-pattern and no-interrogations, but
                            // the pinned subject is always valid, so it is
                            // unreachable and omitted.)
                            Error::BrokenInvariants(set) => {
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Teacher {
                                        target,
                                        site: TeacherRefSite::SlotTeacher(_),
                                    }) = inv
                                    {
                                        return UpdateSlotError::InvalidTeacherId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotTeacherDoesNotTeachSubject(
                                            _,
                                            teacher,
                                            subject,
                                        ),
                                    ) = inv
                                    {
                                        return UpdateSlotError::TeacherDoesNotTeachInSubject(
                                            *teacher, *subject,
                                        );
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::WeekPattern {
                                        target,
                                        site: WeekPatternRefSite::SlotWeekPattern(_),
                                    }) = inv
                                    {
                                        return UpdateSlotError::InvalidWeekPatternId(*target);
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotForSubjectWithoutInterrogations(
                                            _,
                                            subject,
                                        ),
                                    ) = inv
                                    {
                                        return UpdateSlotError::SubjectHasNoInterrogation(
                                            *subject,
                                        );
                                    }
                                }
                                for inv in set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotOverflowsDay { .. },
                                    ) = inv
                                    {
                                        return UpdateSlotError::SlotOverlapsWithNextDay;
                                    }
                                }
                                // The old body's colloscope cleaning is gone with
                                // the cleaning phase: a colle written on a week
                                // the *new* pattern excludes is an interrogation
                                // on a week the slot no longer runs on, and the
                                // cascade clears exactly that cell — the same
                                // convergence route the week patterns take.
                                panic!("Unexpected invariant breaks during UpdateSlot: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateSlot: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteSlot(slot_id) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Slot(
                            collomatique_state_colloscopes::SlotOp::Remove(*slot_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, InvalidOp, PrecheckError, SlotPrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Slot(pe))) => {
                                match pe {
                                    SlotPrecheckError::InvalidSlotId(id) => {
                                        DeleteSlotError::InvalidSlotId(*id)
                                    }
                                    SlotPrecheckError::SlotIdAlreadyExists(_)
                                    | SlotPrecheckError::PositionOutOfBounds { .. }
                                    | SlotPrecheckError::PreviousSlotIsNotInRightSubject(_, _)
                                    | SlotPrecheckError::CannotChangeSubject(_, _, _) => {
                                        panic!("Unexpected slot precheck during DeleteSlot: {e:?}")
                                    }
                                }
                            }
                            // The old body had no invariant arm at all — the
                            // colloscope rows and the pairing rules pointing at
                            // the slot were the cleaning phase's business, and
                            // anything it missed died in this catch-all. They are
                            // the cascade's business now: the cells are cleared,
                            // the rules removed, each repair logged, and this
                            // catch-all is genuinely unreachable.
                            _ => panic!("Unexpected error during DeleteSlot: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::MoveSlotUp(slot_id) => {
                let (_subject_id, current_position) = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                    .ok_or(MoveSlotUpError::InvalidSlotId(*slot_id))?;

                if current_position == 0 {
                    Err(MoveSlotUpError::NoUpperPosition)?;
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Slot(
                            collomatique_state_colloscopes::SlotOp::ChangePosition(
                                *slot_id,
                                current_position - 1,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("No error should be possible at this point");

                assert!(result.is_none());

                Ok(None)
            }
            Self::MoveSlotDown(slot_id) => {
                let (subject_id, current_position) = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                    .ok_or(MoveSlotDownError::InvalidSlotId(*slot_id))?;

                if current_position
                    == session
                        .get_data()
                        .get_inner_data()
                        .params
                        .slots
                        .slot_count_for_subject(subject_id)
                        .expect("Subject id should be valid at this point")
                        - 1
                {
                    Err(MoveSlotDownError::NoLowerPosition)?;
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Slot(
                            collomatique_state_colloscopes::SlotOp::ChangePosition(
                                *slot_id,
                                current_position + 1,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("No error should be possible at this point");

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Slots,
            match self {
                SlotsUpdateOp::AddNewSlot(_desc, _slot) => "Ajouter un créneau de colle".into(),
                SlotsUpdateOp::UpdateSlot(_id, _slot) => "Modifier un créneau de colle".into(),
                SlotsUpdateOp::DeleteSlot(_id) => "Supprimer un créneau de colle".into(),
                SlotsUpdateOp::MoveSlotUp(_id) => "Remonter un créneau de colle".into(),
                SlotsUpdateOp::MoveSlotDown(_id) => "Descendre un créneau de colle".into(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    //! The slot is the middle of the reference graph: it points at a subject, a
    //! teacher and — optionally — a week pattern, and two things point back at
    //! it, the slot pairing rules and the colloscope cells written on it.
    //!
    //! That is why the old `DeleteSlot` had **no** invariant arm at all. Its
    //! two reference sites were the cleaning phase's business, one scan each,
    //! and anything the scans missed died in a bare
    //! « Unexpected error during DeleteSlot ». Both scans are gone: the cascade
    //! removes the rules and clears the cells, logs every one of them, and that
    //! catch-all is genuinely unreachable now.
    //!
    //! The update loses a scan too — the one walking the cells of the slot
    //! looking for colles on a week the *new* week pattern excludes. Those
    //! colles are still cleared, but by the same convergence route the week
    //! patterns take: a cell on a week the slot no longer runs on breaks
    //! `InterrogationOnInactiveWeek`, and the map clears exactly that cell.
    //!
    //! What every arm keeps is its payload scan, and the reason is the one the
    //! whole step turns on: a rejected op is rolled back *before* the map is
    //! asked, so the map sees the slot as it was — or, on the add, no slot at
    //! all — matches none of the break's coordinates against it and answers
    //! nothing. The engine convicts the target and the scan turns the break
    //! back into the bad input it came from. `SlotOverflowsDay` is the sharpest
    //! case, since it is a break the map *does* know a repair for
    //! ([Fix::DeleteOverflowingSlot], when a subject's interrogation is
    //! lengthened over a late slot): its arm compares the break's `start`
    //! against the live slot's, and the live slot still starts where it did.
    //!
    //! The frozen hogwarts base carries five Métamorphose slots, one slot
    //! pairing rule over two of them and two week patterns — but no colloscope
    //! at all. The cells a fixture is about are written by one elementary op on
    //! top of the base, at the top of the fixture that needs them.

    use super::*;
    use crate::test_utils::{fixes, hogwarts};
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        ColloscopeOp, Fix, Op, SlotOp, SlotPairingOp,
        ids::{Id, SlotId, SlotPairingRuleId, SubjectId, TeacherId, WeekId, WeekPatternId},
        slots::Slot,
    };
    use std::collections::BTreeSet;

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

    fn week_pattern_by_name(data: &Data, name: &str) -> WeekPatternId {
        data.get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .iter()
            .find(|(_id, week_pattern)| week_pattern.name == name)
            .map(|(id, _week_pattern)| id)
            .unwrap_or_else(|| panic!("the fixture should have a week pattern named {name}"))
    }

    /// The subject's slots, in display order — the order the family's own move
    /// ops shuffle.
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

    fn slot_of(data: &Data, slot: SlotId) -> Slot {
        data.get_inner_data()
            .params
            .slots
            .find_slot(slot)
            .expect("the fixture's slot should be live")
            .clone()
    }

    /// The slot pairing rules naming `slot` on either side.
    fn rules_over(data: &Data, slot: SlotId) -> Vec<SlotPairingRuleId> {
        data.get_inner_data()
            .params
            .slot_pairings
            .slot_pairing_rule_map
            .iter()
            .filter(|(_id, rule)| {
                rule.antecedent().slot_id == slot || rule.consequent().slot_id == slot
            })
            .map(|(id, _rule)| id)
            .collect()
    }

    /// The `n`-th week in global week order.
    fn week_at(data: &Data, index: usize) -> WeekId {
        data.get_inner_data()
            .params
            .week_ids()
            .nth(index)
            .unwrap_or_else(|| panic!("the fixture should have at least {} weeks", index + 1))
    }

    /// A slot at `weekday`/`start`, on the teacher the caller names. The
    /// subject is overwritten by the op anyway, so it is left to the caller's
    /// choice of the one they are adding to.
    fn slot(subject: SubjectId, teacher: TeacherId, weekday: chrono::Weekday, hour: u32) -> Slot {
        Slot {
            subject_id: subject,
            teacher_id: teacher,
            start_time: collomatique_time::SlotStart {
                weekday: collomatique_time::Weekday(weekday),
                start_time: collomatique_time::WholeMinuteTime::new(
                    chrono::NaiveTime::from_hms_opt(hour, 0, 0).expect("a whole hour is a time"),
                )
                .expect("a whole hour is a whole minute"),
            },
            extra_info: String::new(),
            week_pattern: None,
            cost: 0,
        }
    }

    /// A start no interrogation of the fixture can fit before midnight: the
    /// shortest of them lasts an hour.
    fn too_late_in_the_day() -> collomatique_time::SlotStart {
        collomatique_time::SlotStart {
            weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
            start_time: collomatique_time::WholeMinuteTime::new(
                chrono::NaiveTime::from_hms_opt(23, 30, 0).expect("23:30 is a time"),
            )
            .expect("23:30 is a whole minute"),
        }
    }

    /// Ids no document ever issued.
    fn dangling_slot() -> SlotId {
        unsafe { SlotId::new(1u64 << 40) }
    }

    fn dangling_subject() -> SubjectId {
        unsafe { SubjectId::new(1u64 << 40) }
    }

    fn dangling_teacher() -> TeacherId {
        unsafe { TeacherId::new(1u64 << 40) }
    }

    fn dangling_week_pattern() -> WeekPatternId {
        unsafe { WeekPatternId::new(1u64 << 40) }
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects the cascade to have landed —
    /// each of them valid in that order, exactly as the cascade lands them.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::Slots, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// Runs one op alone on `base` and hands back what the document became and
    /// what the cascade had to repair on the way.
    fn apply_alone(
        base: &AppState<Data, Desc>,
        op: &SlotsUpdateOp,
    ) -> (AppState<Data, Desc>, Vec<CascadeWarning>) {
        let mut session = CascadeSession::new(base.clone());
        op.apply_to_session(&mut session)
            .unwrap_or_else(|e| panic!("{op:?} should land, got {e:?}"));

        session.commit(op.get_desc())
    }

    /// A brand new slot is named by no rule and holds no colle, so it can cost
    /// nothing: the id comes back and the warning log stays empty.
    #[test]
    fn adding_a_slot_creates_it_and_warns_about_nothing() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let mcgonagall = teacher_by_surname(base.get_data(), "McGonagall");
        let added = slot(metamorphose, mcgonagall, chrono::Weekday::Mon, 8);

        let mut session = CascadeSession::new(base.clone());
        let op = SlotsUpdateOp::AddNewSlot(metamorphose, added.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("Mme McGonagall teaches Métamorphose");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding a slot returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(slot_of(state.get_data(), new_id), added);
        assert_eq!(
            slots_of_subject(state.get_data(), metamorphose).last(),
            Some(&new_id),
            "a new slot goes after the subject's last one",
        );
    }

    /// The deletion, with everything a slot can drag along. Hogwarts pairs the
    /// second Métamorphose slot with the third, and the setup op writes two
    /// colles on it — one shape the base does not carry at all. Removing it
    /// used to demand that the cleaning phase empty the way first; now the one
    /// elementary removal goes out and the cascade clears what stood there,
    /// reporting each repair.
    ///
    /// The order is the reference sites' own: the pairing rule first, then the
    /// cells in week order.
    #[test]
    fn deleting_a_slot_takes_its_pairing_rule_and_its_colles() {
        let mut base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let paired = slots_of_subject(base.get_data(), metamorphose)[1];
        let rules = rules_over(base.get_data(), paired);
        assert_eq!(
            rules.len(),
            1,
            "the fixture should name this slot in exactly one pairing rule"
        );

        let mut weeks: Vec<_> = base
            .get_data()
            .get_inner_data()
            .params
            .week_ids()
            .filter(|week| {
                base.get_data()
                    .get_inner_data()
                    .params
                    .is_interrogation_possible(paired, *week)
            })
            .take(2)
            .collect();
        // The reference site carries the week, so this is the order the cascade
        // meets the two cells in.
        weeks.sort();
        assert_eq!(weeks.len(), 2, "the slot should be active on two weeks");
        for week in &weeks {
            base.apply(
                Op::Colloscope(ColloscopeOp::SetInterrogation(
                    paired,
                    *week,
                    BTreeSet::from([0]),
                )),
                (OpCategory::Colloscope, "Préparation".into()),
            )
            .expect("a group of the associated list may be placed on an active week");
        }

        let op = SlotsUpdateOp::DeleteSlot(paired);
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::DeleteSlotPairingRule { rule: rules[0] },
                Fix::ClearInterrogationCell {
                    slot: paired,
                    week: weeks[0],
                },
                Fix::ClearInterrogationCell {
                    slot: paired,
                    week: weeks[1],
                },
            ],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::SlotPairing(SlotPairingOp::Remove(rules[0])),
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        paired,
                        weeks[0],
                        BTreeSet::new(),
                    )),
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        paired,
                        weeks[1],
                        BTreeSet::new(),
                    )),
                    Op::Slot(SlotOp::Remove(paired)),
                ],
            )
            .get_data(),
        );
    }

    /// Putting a slot on a week pattern narrows the weeks it runs on, and the
    /// colles already written on the weeks it drops contradict the new pattern
    /// — the checker's `InterrogationOnInactiveWeek`. The old body walked the
    /// slot's cells looking for exactly those; the cascade finds them now.
    ///
    /// The setup writes two colles, one on a week « Semaines paires » keeps and
    /// one on a week it drops, so what the fixture pins is a *choice*: only the
    /// second cell goes.
    #[test]
    fn putting_a_slot_on_a_pattern_clears_the_colles_it_no_longer_runs_on() {
        let mut base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let moved = slots_of_subject(base.get_data(), metamorphose)[1];
        let pattern = week_pattern_by_name(base.get_data(), "Semaines paires");

        // The first period opens with two weeks without interrogations, so
        // these two are both live for the slot; the pattern keeps the first and
        // drops the second.
        let kept = week_at(base.get_data(), 2);
        let dropped = week_at(base.get_data(), 3);
        let excluded = &base
            .get_data()
            .get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .get(&pattern)
            .expect("the fixture's week pattern should be live")
            .excluded_weeks;
        assert!(!excluded.contains(&kept) && excluded.contains(&dropped));

        for week in [kept, dropped] {
            base.apply(
                Op::Colloscope(ColloscopeOp::SetInterrogation(
                    moved,
                    week,
                    BTreeSet::from([0]),
                )),
                (OpCategory::Colloscope, "Préparation".into()),
            )
            .expect("a group of the associated list may be placed on an active week");
        }

        let mut patterned = slot_of(base.get_data(), moved);
        patterned.week_pattern = Some(pattern);

        let op = SlotsUpdateOp::UpdateSlot(moved, patterned.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::ClearInterrogationCell {
                slot: moved,
                week: dropped,
            }],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        moved,
                        dropped,
                        BTreeSet::new(),
                    )),
                    Op::Slot(SlotOp::Update(moved, patterned)),
                ],
            )
            .get_data(),
        );
    }

    /// The conviction route, end to end through `ops/`. A colle lasts an hour
    /// in Métamorphose, so a slot starting at 23:30 would run past midnight —
    /// `Convergence::SlotOverflowsDay`, which the map answers with
    /// [Fix::DeleteOverflowingSlot] when a *subject* grows its interrogations
    /// over a late slot. Here the offending start is the payload's own, and the
    /// arm compares the break's start against the live slot's: the update was
    /// rolled back, the live slot still starts where it did, so the map answers
    /// nothing, the engine convicts the target, and the scan reports the bad
    /// input. The slot is never quietly deleted.
    ///
    /// What the fixture pins is that outcome, and the outcome has two guards,
    /// not one: were the arm's comparison dropped, the fix would remove the
    /// slot, the retried update would find nothing at that id, and the engine
    /// would still restore the entry snapshot and answer the break it
    /// remembered (design doc D4). Same error, same document. The one thing the
    /// ops layer owns here is the scan below the engine, which is what the
    /// [Convergence::SlotOverflowsDay] arm of `UpdateSlot` reddens without.
    #[test]
    fn a_start_too_late_in_the_day_is_rejected_rather_than_repaired() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let bibine = teacher_by_surname(base.get_data(), "Bibine");
        let late = slots_of_subject(base.get_data(), metamorphose)[1];

        let mut too_late = slot_of(base.get_data(), late);
        too_late.start_time = too_late_in_the_day();

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotsUpdateOp::UpdateSlot(late, too_late)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::UpdateSlot(UpdateSlotError::SlotOverlapsWithNextDay),
        );
        assert_eq!(
            SlotsUpdateOp::AddNewSlot(
                metamorphose,
                Slot {
                    start_time: too_late_in_the_day(),
                    ..slot(metamorphose, bibine, chrono::Weekday::Mon, 8)
                },
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            SlotsUpdateError::AddNewSlot(AddNewSlotError::SlotOverlapsWithNextDay),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::Slots, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// The add's own surface, in the order the old validator answered it: the
    /// two ops-level prechecks first — a subject that does not exist, then one
    /// that runs no interrogations — and then the payload scans, teacher
    /// resolves before teacher teaches before week pattern.
    #[test]
    fn the_add_reports_every_way_its_payload_can_be_wrong() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let mcgonagall = teacher_by_surname(base.get_data(), "McGonagall");
        let rogue = teacher_by_surname(base.get_data(), "Rogue");

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotsUpdateOp::AddNewSlot(
                dangling_subject(),
                slot(dangling_subject(), mcgonagall, chrono::Weekday::Mon, 8),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            SlotsUpdateError::AddNewSlot(AddNewSlotError::InvalidSubjectId(dangling_subject())),
        );
        assert_eq!(
            SlotsUpdateOp::AddNewSlot(
                quidditch,
                slot(quidditch, mcgonagall, chrono::Weekday::Mon, 8),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            SlotsUpdateError::AddNewSlot(AddNewSlotError::SubjectHasNoInterrogation(quidditch)),
        );
        assert_eq!(
            SlotsUpdateOp::AddNewSlot(
                metamorphose,
                slot(metamorphose, dangling_teacher(), chrono::Weekday::Mon, 8),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            SlotsUpdateError::AddNewSlot(AddNewSlotError::InvalidTeacherId(dangling_teacher())),
        );
        assert_eq!(
            SlotsUpdateOp::AddNewSlot(
                metamorphose,
                slot(metamorphose, rogue, chrono::Weekday::Mon, 8),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            SlotsUpdateError::AddNewSlot(AddNewSlotError::TeacherDoesNotTeachInSubject(
                rogue,
                metamorphose,
            )),
        );
        assert_eq!(
            SlotsUpdateOp::AddNewSlot(
                metamorphose,
                Slot {
                    week_pattern: Some(dangling_week_pattern()),
                    ..slot(metamorphose, mcgonagall, chrono::Weekday::Mon, 8)
                },
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            SlotsUpdateError::AddNewSlot(AddNewSlotError::InvalidWeekPatternId(
                dangling_week_pattern()
            )),
        );

        // Which break wins when a payload carries several is public API, so the
        // order is pinned rather than left to the set's own: a slot that names
        // nobody, follows nothing and starts too late reports the teacher.
        assert_eq!(
            SlotsUpdateOp::AddNewSlot(
                metamorphose,
                Slot {
                    week_pattern: Some(dangling_week_pattern()),
                    start_time: too_late_in_the_day(),
                    ..slot(metamorphose, dangling_teacher(), chrono::Weekday::Mon, 8)
                },
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            SlotsUpdateError::AddNewSlot(AddNewSlotError::InvalidTeacherId(dangling_teacher())),
        );
        // And a live teacher who does not teach the subject is reported before
        // the late start, one rank down the same order.
        assert_eq!(
            SlotsUpdateOp::AddNewSlot(
                metamorphose,
                Slot {
                    start_time: too_late_in_the_day(),
                    ..slot(metamorphose, rogue, chrono::Weekday::Mon, 8)
                },
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            SlotsUpdateError::AddNewSlot(AddNewSlotError::TeacherDoesNotTeachInSubject(
                rogue,
                metamorphose,
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The update's surface: the state layer's own precheck on a dead slot id,
    /// then the same three payload scans as the add. Every one of them is a
    /// break the map knows a repair for when it arises honestly — a teacher
    /// really deleted takes their slots, a week pattern really deleted is
    /// cleared off them — and every one of them is convicted here instead,
    /// because the live slot still holds its old teacher and its old pattern.
    #[test]
    fn the_update_reports_every_way_its_payload_can_be_wrong() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let updated = slots_of_subject(base.get_data(), metamorphose)[1];
        let rogue = teacher_by_surname(base.get_data(), "Rogue");

        let mut with_dead_teacher = slot_of(base.get_data(), updated);
        with_dead_teacher.teacher_id = dangling_teacher();
        let mut with_wrong_teacher = slot_of(base.get_data(), updated);
        with_wrong_teacher.teacher_id = rogue;
        let mut with_dead_pattern = slot_of(base.get_data(), updated);
        with_dead_pattern.week_pattern = Some(dangling_week_pattern());
        let mut wrong_everywhere = slot_of(base.get_data(), updated);
        wrong_everywhere.teacher_id = dangling_teacher();
        wrong_everywhere.week_pattern = Some(dangling_week_pattern());
        wrong_everywhere.start_time = too_late_in_the_day();
        let mut wrong_teacher_and_late = slot_of(base.get_data(), updated);
        wrong_teacher_and_late.teacher_id = rogue;
        wrong_teacher_and_late.start_time = too_late_in_the_day();

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotsUpdateOp::UpdateSlot(dangling_slot(), slot_of(base.get_data(), updated))
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::UpdateSlot(UpdateSlotError::InvalidSlotId(dangling_slot())),
        );
        assert_eq!(
            SlotsUpdateOp::UpdateSlot(updated, with_dead_teacher)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::UpdateSlot(UpdateSlotError::InvalidTeacherId(dangling_teacher())),
        );
        assert_eq!(
            SlotsUpdateOp::UpdateSlot(updated, with_wrong_teacher)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::UpdateSlot(UpdateSlotError::TeacherDoesNotTeachInSubject(
                rogue,
                metamorphose,
            )),
        );
        assert_eq!(
            SlotsUpdateOp::UpdateSlot(updated, with_dead_pattern)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::UpdateSlot(UpdateSlotError::InvalidWeekPatternId(
                dangling_week_pattern()
            )),
        );
        // Same order as the add's, pinned the same way: teacher resolves, then
        // teacher teaches, then week pattern, then the day overflow.
        assert_eq!(
            SlotsUpdateOp::UpdateSlot(updated, wrong_everywhere)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::UpdateSlot(UpdateSlotError::InvalidTeacherId(dangling_teacher())),
        );
        assert_eq!(
            SlotsUpdateOp::UpdateSlot(updated, wrong_teacher_and_late)
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::UpdateSlot(UpdateSlotError::TeacherDoesNotTeachInSubject(
                rogue,
                metamorphose,
            )),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The deletion's whole surface is the state layer's precheck: a dead id
    /// is the only thing that can stop it, since everything else standing in
    /// the way is repaired.
    #[test]
    fn a_dead_slot_id_is_rejected_by_the_delete() {
        let base = hogwarts();

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotsUpdateOp::DeleteSlot(dangling_slot())
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::DeleteSlot(DeleteSlotError::InvalidSlotId(dangling_slot())),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    /// The two move ops, which reference nothing and are referenced by nothing:
    /// they shuffle the subject's display order and cost the cascade nothing.
    #[test]
    fn moving_a_slot_reorders_its_subject_and_warns_about_nothing() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let order = slots_of_subject(base.get_data(), metamorphose);

        let op = SlotsUpdateOp::MoveSlotUp(order[1]);
        let (state, warnings) = apply_alone(&base, &op);
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            slots_of_subject(state.get_data(), metamorphose),
            vec![order[1], order[0], order[2], order[3], order[4]],
        );

        let op = SlotsUpdateOp::MoveSlotDown(order[0]);
        let (state, warnings) = apply_alone(&base, &op);
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            slots_of_subject(state.get_data(), metamorphose),
            vec![order[1], order[0], order[2], order[3], order[4]],
        );
    }

    /// The move ops' error surface, all of it ops-level: both ends of the list
    /// refuse to go further, and a dead id is caught before any elementary op
    /// is issued.
    ///
    /// `MoveSlotDown` answers a dead id with `MoveSlotUpError::InvalidSlotId`,
    /// across the two enums. That is the wart D14 fixes at the end of the step;
    /// until then it is replicated verbatim, and this assert is what will have
    /// to be re-cut when it goes.
    #[test]
    fn the_moves_report_the_ends_of_the_list_and_a_dead_id() {
        let base = hogwarts();
        let metamorphose = subject_by_name(base.get_data(), "Métamorphose");
        let order = slots_of_subject(base.get_data(), metamorphose);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            SlotsUpdateOp::MoveSlotUp(order[0])
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::MoveSlotUp(MoveSlotUpError::NoUpperPosition),
        );
        assert_eq!(
            SlotsUpdateOp::MoveSlotDown(order[order.len() - 1])
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::MoveSlotDown(MoveSlotDownError::NoLowerPosition),
        );
        assert_eq!(
            SlotsUpdateOp::MoveSlotUp(dangling_slot())
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::MoveSlotUp(MoveSlotUpError::InvalidSlotId(dangling_slot())),
        );
        assert_eq!(
            SlotsUpdateOp::MoveSlotDown(dangling_slot())
                .apply_to_session(&mut session)
                .unwrap_err(),
            SlotsUpdateError::MoveSlotUp(MoveSlotUpError::InvalidSlotId(dangling_slot())),
        );

        assert_eq!(session.get_data(), base.get_data());
    }
}
