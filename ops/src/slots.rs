use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlotsUpdateWarning {
    LooseRuleReferencingSlot(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::RuleId,
    ),
    LooseColloscopeDataForSlot(collomatique_state_colloscopes::SlotId),
}

impl SlotsUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            SlotsUpdateWarning::LooseRuleReferencingSlot(_slot_id, rule_id) => {
                let Some(rule) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .rules
                    .rule_map
                    .get(rule_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte de la règle \"{}\" qui utilise le créneau",
                    rule.name,
                ))
            }
            Self::LooseColloscopeDataForSlot(slot_id) => {
                let Some((subject_id, position)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
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
                let slot = &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .subject_map
                    .get(&subject_id)
                    .expect("Subject id should be valid at this point")
                    .ordered_slots[position]
                    .1;
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
                    teacher.desc.firstname, teacher.desc.surname, subject.parameters.name, slot.start_time.weekday, slot.start_time.start_time.into_inner(),
                ))
            }
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
                let Some(old_slot) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .find_slot(*slot_id)
                else {
                    return None;
                };
                let old_week_pattern_id = old_slot.week_pattern;
                let new_week_pattern_id = slot.week_pattern;

                let week_count = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .count_weeks();
                let old_week_pattern = match old_week_pattern_id {
                    Some(id) => data
                        .get_data()
                        .get_inner_data()
                        .params
                        .week_patterns
                        .week_pattern_map
                        .get(&id)
                        .expect("Week pattern ID should be valid")
                        .weeks
                        .clone(),
                    None => vec![true; week_count],
                };
                let new_week_pattern = match new_week_pattern_id {
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
                        wp.weeks.clone()
                    }
                    None => vec![true; week_count],
                };

                let mut first_week_in_period = 0usize;
                for (period_id, period) in &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list
                {
                    let collo_period = data
                        .get_data()
                        .get_inner_data()
                        .colloscope
                        .period_map
                        .get(period_id)
                        .expect("Period ID should appear in colloscope");
                    let Some(collo_slot) = collo_period.slot_map.get(slot_id) else {
                        continue;
                    };
                    for week_in_period in 0..period.len() {
                        // If the week is disabled at the period level then it is already disabled in colloscope
                        if !period[week_in_period].interrogations {
                            continue;
                        }

                        let current_week = first_week_in_period + week_in_period;
                        let old_status = old_week_pattern[current_week];
                        let new_status = new_week_pattern[current_week];
                        if old_status && !new_status {
                            let interrogation = collo_slot.interrogations[week_in_period]
                                .as_ref()
                                .expect(
                                "There should be an interrogation as the week used to be enabled",
                            );

                            if !interrogation.is_empty() {
                                return Some(CleaningOp {
                                    warning: SlotsUpdateWarning::LooseColloscopeDataForSlot(
                                        *slot_id,
                                    ),
                                    op: UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                        *period_id,
                                        *slot_id,
                                        week_in_period,
                                        collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation::default(),
                                    )),
                                });
                            }
                        }
                    }

                    first_week_in_period += period.len();
                }

                None
            }
            SlotsUpdateOp::DeleteSlot(slot_id) => {
                for (period_id, collo_period) in
                    &data.get_data().get_inner_data().colloscope.period_map
                {
                    let Some(collo_slot) = collo_period.slot_map.get(slot_id) else {
                        continue;
                    };

                    for week in 0..collo_slot.interrogations.len() {
                        let Some(interrogation) = &collo_slot.interrogations[week] else {
                            continue;
                        };

                        if !interrogation.is_empty() {
                            return Some(CleaningOp {
                                warning: SlotsUpdateWarning::LooseColloscopeDataForSlot(
                                    *slot_id,
                                ),
                                op: UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                    *period_id,
                                    *slot_id,
                                    week,
                                    collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation::default(),
                                )),
                            });
                        }
                    }
                }

                for (rule_id, rule) in &data.get_data().get_inner_data().params.rules.rule_map {
                    if rule.desc.references_slot(*slot_id) {
                        return Some(CleaningOp {
                            warning: SlotsUpdateWarning::LooseRuleReferencingSlot(
                                *slot_id, *rule_id,
                            ),
                            op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(*slot_id)),
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
                let Some(subject_slots) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .subject_map
                    .get(subject_id)
                else {
                    return Err(AddNewSlotError::SubjectHasNoInterrogation(*subject_id).into());
                };

                let last_slot_id = subject_slots.ordered_slots.last().map(|(id, _)| id.clone());

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Slot(
                            collomatique_state_colloscopes::SlotOp::AddAfter(
                                *subject_id,
                                last_slot_id,
                                slot.clone(),
                            )
                        ),
                        self.get_desc(),
                    ).map_err(|e| if let collomatique_state_colloscopes::Error::Slot(se) = e {
                        match se {
                            collomatique_state_colloscopes::SlotError::InvalidSubjectId(_) => panic!("Subject id should be valid at this point"),
                            collomatique_state_colloscopes::SlotError::SubjectHasNoInterrogation(_) => panic!("Subject should have interrogations at this point"),
                            collomatique_state_colloscopes::SlotError::InvalidTeacherId(id) => AddNewSlotError::InvalidTeacherId(id),
                            collomatique_state_colloscopes::SlotError::InvalidWeekPatternId(id) => AddNewSlotError::InvalidWeekPatternId(id),
                            collomatique_state_colloscopes::SlotError::TeacherDoesNotTeachInSubject(tid, sid) => AddNewSlotError::TeacherDoesNotTeachInSubject(tid, sid),
                            collomatique_state_colloscopes::SlotError::SlotOverlapsWithNextDay => AddNewSlotError::SlotOverlapsWithNextDay,
                            _ => panic!("Unexpected slot error during AddNewSlot: {:?}", se),
                        }
                    } else {
                        panic!("Unexpected error during AddNewSlot: {:?}", e);
                    })?;
                let Some(collomatique_state_colloscopes::NewId::SlotId(new_id)) = result else {
                    panic!("Unexpected result from SlotOp::AddAfter");
                };
                Ok(Some(new_id))
            }
            Self::UpdateSlot(slot_id, slot) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Slot(
                            collomatique_state_colloscopes::SlotOp::Update(
                                *slot_id,
                                slot.clone(),
                            )
                        ),
                        self.get_desc()
                    ).map_err(|e| if let collomatique_state_colloscopes::Error::Slot(se) = e {
                        match se {
                            collomatique_state_colloscopes::SlotError::InvalidSlotId(id) => UpdateSlotError::InvalidSlotId(id),
                            collomatique_state_colloscopes::SlotError::InvalidSubjectId(id) => UpdateSlotError::InvalidSubjectId(id),
                            collomatique_state_colloscopes::SlotError::SubjectHasNoInterrogation(id) => UpdateSlotError::SubjectHasNoInterrogation(id),
                            collomatique_state_colloscopes::SlotError::InvalidTeacherId(id) => UpdateSlotError::InvalidTeacherId(id),
                            collomatique_state_colloscopes::SlotError::InvalidWeekPatternId(id) => UpdateSlotError::InvalidWeekPatternId(id),
                            collomatique_state_colloscopes::SlotError::TeacherDoesNotTeachInSubject(tid, sid) => UpdateSlotError::TeacherDoesNotTeachInSubject(tid, sid),
                            collomatique_state_colloscopes::SlotError::SlotOverlapsWithNextDay => UpdateSlotError::SlotOverlapsWithNextDay,
                            _ => panic!("Unexpected slot error during UpdateSlot: {:?}", se),
                        }
                    } else {
                        panic!("Unexpected error during UpdateSlot: {:?}", e);
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
                        if let collomatique_state_colloscopes::Error::Slot(se) = e {
                            match se {
                                collomatique_state_colloscopes::SlotError::InvalidSlotId(id) => {
                                    DeleteSlotError::InvalidSlotId(id)
                                }
                                collomatique_state_colloscopes::SlotError::SlotIsReferencedByRule(_slot_id, _rule_id) => {
                                    panic!("Rules should be cleaned before removing a slot");
                                }
                                _ => panic!("Unexpected slot error during DeleteSlot: {:?}", se),
                            }
                        } else {
                            panic!("Unexpected error during DeleteSlot: {:?}", e);
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
                        .subject_map
                        .get(&subject_id)
                        .expect("Subject id should be valid at this point")
                        .ordered_slots
                        .len()
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
