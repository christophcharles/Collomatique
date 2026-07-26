use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlotsUpdateWarning {
    LooseColloscopeDataForSlot(collomatique_state_colloscopes::SlotId),
    LooseSlotPairingRulesForSlot(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::SlotPairingRuleId,
    ),
}

impl SlotsUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            Self::LooseColloscopeDataForSlot(slot_id) => {
                let Some((subject_id, slot)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot_with_subject(*slot_id)
                else {
                    return None;
                };
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(subject_id)
                else {
                    return None;
                };
                let Some(teacher) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .teachers
                    .teacher_map
                    .get(&slot.teacher_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte du remplissage du créneaux de colle du colleur {} {} pour la matière \"{}\" le {} à {} dans le colloscope",
                    teacher.desc.firstname,
                    teacher.desc.surname,
                    subject.parameters.name,
                    slot.start_time.weekday,
                    slot.start_time.start_time.into_inner(),
                ))
            }
            Self::LooseSlotPairingRulesForSlot(_slot_id, _rule_id) => Some(
                "Suppression de l'appariement de créneaux référençant le créneau supprimé"
                    .to_string(),
            ),
        }
    }
}

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
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<SlotsUpdateWarning>> {
        match self {
            SlotsUpdateOp::AddNewSlot(_desc, _slot) => None,
            SlotsUpdateOp::UpdateSlot(slot_id, slot) => {
                if data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot(*slot_id)
                    .is_none()
                {
                    return None;
                }
                let new_week_pattern_id = slot.week_pattern;

                let new_excluded = match new_week_pattern_id {
                    Some(id) => {
                        let Some(wp) = data
                            .get_data()
                            .get_inner_data()
                            .params
                            .week_patterns
                            .week_pattern_map
                            .get(&id)
                        else {
                            return None;
                        };
                        wp.excluded_weeks.clone()
                    }
                    None => std::collections::BTreeSet::new(),
                };

                // A non-empty colloscope row on a week the new pattern newly
                // excludes must be cleared first. A row only exists on a week the
                // slot is currently active on (period interrogations on ∧ not
                // excluded by the old pattern), so "active before" is implicit;
                // only "excluded after" needs checking.
                let inner = data.get_data().get_inner_data();
                for (week_id, _groups) in inner.colloscope.interrogations_for_slot(*slot_id) {
                    if !new_excluded.contains(&week_id) {
                        continue;
                    }
                    return Some(CleaningOp {
                        warning: SlotsUpdateWarning::LooseColloscopeDataForSlot(*slot_id),
                        op: UpdateOp::Colloscope(
                            ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                *slot_id,
                                week_id,
                                std::collections::BTreeSet::new(),
                            ),
                        ),
                    });
                }

                None
            }
            SlotsUpdateOp::DeleteSlot(slot_id) => {
                let inner = data.get_data().get_inner_data();
                if let Some((week_id, _groups)) =
                    inner.colloscope.interrogations_for_slot(*slot_id).next()
                {
                    return Some(CleaningOp {
                        warning: SlotsUpdateWarning::LooseColloscopeDataForSlot(*slot_id),
                        op: UpdateOp::Colloscope(
                            ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                *slot_id,
                                week_id,
                                std::collections::BTreeSet::new(),
                            ),
                        ),
                    });
                }

                for (rule_id, rule) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .iter()
                {
                    let rule_id = &rule_id;
                    if rule.antecedent().slot_id == *slot_id
                        || rule.consequent().slot_id == *slot_id
                    {
                        return Some(CleaningOp {
                            warning: SlotsUpdateWarning::LooseSlotPairingRulesForSlot(
                                *slot_id, *rule_id,
                            ),
                            op: UpdateOp::SlotPairings(
                                SlotPairingsUpdateOp::DeleteSlotPairingRule(*rule_id),
                            ),
                        });
                    }
                }

                None
            }
            SlotsUpdateOp::MoveSlotUp(_id) => None,
            SlotsUpdateOp::MoveSlotDown(_id) => None,
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::SlotId>, SlotsUpdateError> {
        match self {
            Self::AddNewSlot(subject_id, slot) => {
                if data
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
                if !data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .is_some_and(|subject| subject.parameters.interrogation_parameters.is_some())
                {
                    return Err(AddNewSlotError::SubjectHasNoInterrogation(*subject_id).into());
                }

                let last_slot_id = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .last_slot_id_for_subject(*subject_id);

                // The state op takes the subject from the slot itself.
                let mut slot = slot.clone();
                slot.subject_id = *subject_id;
                // Capture the teacher id before `slot` is moved into the op:
                // the SlotTeacherDoesNotTeachSubject convergence carries only the
                // slot id, so the reported (teacher, subject) pair is synthesized
                // from the op payload in scope.
                let teacher_id = slot.teacher_id;

                let result = data
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
                        match e {
                            // Subject validity and interrogation presence are
                            // pre-checked in the ops layer above, so those two
                            // guards are unreachable here (their breaks would fall
                            // through to the panic). The pre-op state was valid, so
                            // every break in the set was introduced by this add.
                            // Old validator order (validate_slot_internal):
                            // teacher-resolves, teacher-teaches, week-pattern, then
                            // day overflow.
                            Error::Invariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Teacher {
                                        target,
                                        site: TeacherRefSite::SlotTeacher(_),
                                    }) = inv
                                    {
                                        return AddNewSlotError::InvalidTeacherId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotTeacherDoesNotTeachSubject(_),
                                    ) = inv
                                    {
                                        return AddNewSlotError::TeacherDoesNotTeachInSubject(
                                            teacher_id,
                                            *subject_id,
                                        );
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::WeekPattern {
                                        target,
                                        site: WeekPatternRefSite::SlotWeekPattern(_),
                                    }) = inv
                                    {
                                        return AddNewSlotError::InvalidWeekPatternId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotOverflowsDay(_),
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
                if let Some((subject_id, _pos)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                {
                    slot.subject_id = subject_id;
                }
                // Sources for the reduced convergence variants (both carry only
                // the slot id). On the Invariants path the slot existed (else the
                // InvalidSlotId precheck fires first), so the subject is pinned.
                let teacher_id = slot.teacher_id;
                let subject_id = slot.subject_id;

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Slot(
                            collomatique_state_colloscopes::SlotOp::Update(*slot_id, slot),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Convergence, Error, FixableInvariant, PrecheckError, Reference,
                            SlotPrecheckError, TeacherRefSite, WeekPatternRefSite,
                        };
                        match e {
                            Error::Precheck(PrecheckError::Slot(pe)) => match pe {
                                SlotPrecheckError::InvalidSlotId(id) => {
                                    UpdateSlotError::InvalidSlotId(id)
                                }
                                // The subject is pinned to the slot's current
                                // subject above, so CannotChangeSubject cannot
                                // arise here; the remaining precheck variants are
                                // add/move-only.
                                _ => panic!("Unexpected slot precheck during UpdateSlot: {pe:?}"),
                            },
                            // The pre-op state was valid, so every break in the set
                            // was introduced by this update. Old validator order
                            // (validate_slot_internal): teacher-resolves,
                            // teacher-teaches, week-pattern, subject-has-
                            // interrogations, then day overflow. (InvalidSubjectId
                            // sits between week-pattern and no-interrogations, but
                            // the pinned subject is always valid, so it is
                            // unreachable and omitted.)
                            Error::Invariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Teacher {
                                        target,
                                        site: TeacherRefSite::SlotTeacher(_),
                                    }) = inv
                                    {
                                        return UpdateSlotError::InvalidTeacherId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotTeacherDoesNotTeachSubject(_),
                                    ) = inv
                                    {
                                        return UpdateSlotError::TeacherDoesNotTeachInSubject(
                                            teacher_id, subject_id,
                                        );
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::WeekPattern {
                                        target,
                                        site: WeekPatternRefSite::SlotWeekPattern(_),
                                    }) = inv
                                    {
                                        return UpdateSlotError::InvalidWeekPatternId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotForSubjectWithoutInterrogations(_),
                                    ) = inv
                                    {
                                        return UpdateSlotError::SubjectHasNoInterrogation(
                                            subject_id,
                                        );
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::SlotOverflowsDay(_),
                                    ) = inv
                                    {
                                        return UpdateSlotError::SlotOverlapsWithNextDay;
                                    }
                                }
                                panic!("Unexpected invariant breaks during UpdateSlot: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateSlot: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteSlot(slot_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Slot(
                            collomatique_state_colloscopes::SlotOp::Remove(*slot_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, PrecheckError, SlotPrecheckError,
                        };
                        match e {
                            Error::Precheck(PrecheckError::Slot(
                                SlotPrecheckError::InvalidSlotId(id),
                            )) => DeleteSlotError::InvalidSlotId(id),
                            // Colloscope rows and slot-pairing references are
                            // cleared by the cleaning phase before this runs, so
                            // their stripped-guard breaks are unreachable here.
                            _ => panic!("Unexpected error during DeleteSlot: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::MoveSlotUp(slot_id) => {
                let (_subject_id, current_position) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                    .ok_or(MoveSlotUpError::InvalidSlotId(*slot_id))?;

                if current_position == 0 {
                    Err(MoveSlotUpError::NoUpperPosition)?;
                }

                let result = data
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
                let (subject_id, current_position) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                    .ok_or(MoveSlotUpError::InvalidSlotId(*slot_id))?;

                if current_position
                    == data
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

                let result = data
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
