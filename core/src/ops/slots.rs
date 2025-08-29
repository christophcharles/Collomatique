use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlotsUpdateWarning {
    LooseRuleReferencingSlot(
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::RuleId,
    ),
}

impl SlotsUpdateWarning {
    pub fn build_desc<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            SlotsUpdateWarning::LooseRuleReferencingSlot(_slot_id, rule_id) => {
                let Some(rule) = data.get_data().get_rules().rule_map.get(rule_id) else {
                    return None;
                };
                Some(format!(
                    "Perte de la règle \"{}\" qui utilise le créneau",
                    rule.name,
                ))
            }
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Error)]
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

#[derive(Debug, Error)]
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

#[derive(Debug, Error)]
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

#[derive(Debug, Error)]
pub enum DeleteSlotError {
    #[error("Slot ID {0:?} is invalid")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
}

#[derive(Debug, Error)]
pub enum MoveSlotUpError {
    #[error("Slot ID {0:?} is invalid")]
    InvalidSlotId(collomatique_state_colloscopes::SlotId),
    #[error("Slot is already the first slot")]
    NoUpperPosition,
}

#[derive(Debug, Error)]
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
        _data: &T,
    ) -> Option<CleaningOp<SlotsUpdateWarning>> {
        todo!()
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::SlotId>, SlotsUpdateError> {
        todo!()
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

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Vec<SlotsUpdateWarning> {
        match self {
            SlotsUpdateOp::AddNewSlot(_desc, _slot) => vec![],
            SlotsUpdateOp::UpdateSlot(_id, _slot) => vec![],
            SlotsUpdateOp::DeleteSlot(slot_id) => {
                let mut output = vec![];

                for (rule_id, rule) in &data.get_data().get_rules().rule_map {
                    if rule.desc.references_slot(*slot_id) {
                        output.push(SlotsUpdateWarning::LooseRuleReferencingSlot(
                            *slot_id, *rule_id,
                        ));
                    }
                }

                output
            }
            SlotsUpdateOp::MoveSlotUp(_id) => vec![],
            SlotsUpdateOp::MoveSlotDown(_id) => vec![],
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::SlotId>, SlotsUpdateError> {
        match self {
            Self::AddNewSlot(subject_id, slot) => {
                if data
                    .get_data()
                    .get_subjects()
                    .find_subject_position(*subject_id)
                    .is_none()
                {
                    return Err(AddNewSlotError::InvalidSubjectId(*subject_id).into());
                }
                let Some(subject_slots) = data.get_data().get_slots().subject_map.get(subject_id)
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
                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                for (rule_id, rule) in &data.get_data().get_rules().rule_map {
                    if rule.desc.references_slot(*slot_id) {
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Rule(
                                    collomatique_state_colloscopes::RuleOp::Remove(*rule_id),
                                ),
                                "Enlever une règle référençant le créneau".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Slot(
                            collomatique_state_colloscopes::SlotOp::Remove(*slot_id),
                        ),
                        "Suppression effective du créneau de colle".into(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Slot(se) = e {
                            match se {
                                collomatique_state_colloscopes::SlotError::InvalidSlotId(id) => {
                                    DeleteSlotError::InvalidSlotId(id)
                                }
                                _ => panic!("Unexpected slot error during DeleteSlot: {:?}", se),
                            }
                        } else {
                            panic!("Unexpected error during DeleteSlot: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                *data = session.commit(self.get_desc());

                Ok(None)
            }
            Self::MoveSlotUp(slot_id) => {
                let (_subject_id, current_position) = data
                    .get_data()
                    .get_slots()
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
                    .get_slots()
                    .find_slot_subject_and_position(*slot_id)
                    .ok_or(MoveSlotUpError::InvalidSlotId(*slot_id))?;

                if current_position
                    == data
                        .get_data()
                        .get_slots()
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
}
