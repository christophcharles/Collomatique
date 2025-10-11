use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WeekPatternsUpdateWarning {
    LooseInterrogationSlot(collomatique_state_colloscopes::SlotId),
    LooseScheduleIncompat(collomatique_state_colloscopes::IncompatId),
    LooseColloscopeLinkWithWeekPattern(
        collomatique_state_colloscopes::ColloscopeId,
        collomatique_state_colloscopes::WeekPatternId,
    ),
}

impl WeekPatternsUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            WeekPatternsUpdateWarning::LooseInterrogationSlot(slot_id) => {
                let Some((subject_id, position)) = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                else {
                    return None;
                };
                let slot = &data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .slots
                    .subject_map
                    .get(&subject_id)
                    .expect("Subject id should be valid at this point")
                    .ordered_slots[position]
                    .1;
                let Some(teacher) = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .teachers
                    .teacher_map
                    .get(&slot.teacher_id)
                else {
                    return None;
                };
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .subjects
                    .find_subject(subject_id)
                else {
                    return None;
                };
                Some(format!(
                    "Pertes du créneaux de colle du colleur {} {} pour la matière \"{}\" le {} à {}",
                    teacher.desc.firstname, teacher.desc.surname, subject.parameters.name, slot.start_time.weekday, slot.start_time.start_time.into_inner(),
                ))
            }
            Self::LooseScheduleIncompat(incompat_id) => {
                let Some(incompat) = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .incompats
                    .incompat_map
                    .get(incompat_id)
                else {
                    return None;
                };
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .subjects
                    .find_subject(incompat.subject_id)
                else {
                    return None;
                };

                let slot_desc: Vec<_> = incompat
                    .slots
                    .iter()
                    .map(|slot| {
                        format!(
                            "le {} à {}",
                            slot.start().weekday,
                            slot.start().start_time.into_inner()
                        )
                    })
                    .collect();

                Some(format!(
                    "Perte d'une incompatibilité horaire pour la matière \"{}\" ({})",
                    subject.parameters.name,
                    slot_desc.join(", "),
                ))
            }
            WeekPatternsUpdateWarning::LooseColloscopeLinkWithWeekPattern(
                colloscope_id,
                week_pattern_id,
            ) => {
                let Some(colloscope) = data
                    .get_data()
                    .get_inner_data()
                    .colloscopes
                    .colloscope_map
                    .get(colloscope_id)
                else {
                    return None;
                };
                let Some(week_pattern) = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .week_patterns
                    .week_pattern_map
                    .get(week_pattern_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte de la possibilité de mettre à jour le colloscope \"{}\" pour le modèle de périodicité \"{}\"",
                    colloscope.name,
                    week_pattern.name,
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WeekPatternsUpdateOp {
    AddNewWeekPattern(collomatique_state_colloscopes::week_patterns::WeekPattern),
    UpdateWeekPattern(
        collomatique_state_colloscopes::WeekPatternId,
        collomatique_state_colloscopes::week_patterns::WeekPattern,
    ),
    DeleteWeekPattern(collomatique_state_colloscopes::WeekPatternId),
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum WeekPatternsUpdateError {
    #[error(transparent)]
    UpdateWeekPattern(#[from] UpdateWeekPatternError),
    #[error(transparent)]
    DeleteWeekPattern(#[from] DeleteWeekPatternError),
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum UpdateWeekPatternError {
    #[error("Week pattern ID {0:?} is invalid")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DeleteWeekPatternError {
    #[error("Week pattern ID {0:?} is invalid")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

impl WeekPatternsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<WeekPatternsUpdateWarning>> {
        match self {
            Self::AddNewWeekPattern(_) => None,
            Self::UpdateWeekPattern(_, _) => None,
            Self::DeleteWeekPattern(week_pattern_id) => {
                for (colloscope_id, colloscope) in
                    &data.get_data().get_inner_data().colloscopes.colloscope_map
                {
                    if colloscope
                        .id_maps
                        .week_patterns
                        .contains_key(week_pattern_id)
                    {
                        let mut new_colloscope = colloscope.clone();
                        new_colloscope.id_maps.week_patterns.remove(week_pattern_id);

                        return Some(CleaningOp {
                            warning: WeekPatternsUpdateWarning::LooseColloscopeLinkWithWeekPattern(
                                *colloscope_id,
                                *week_pattern_id,
                            ),
                            op: UpdateOp::Colloscopes(ColloscopesUpdateOp::UpdateColloscope(
                                *colloscope_id,
                                new_colloscope,
                            )),
                        });
                    }
                }

                for (_subject_id, subject_slots) in &data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .slots
                    .subject_map
                {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.week_pattern == Some(*week_pattern_id) {
                            return Some(CleaningOp {
                                warning: WeekPatternsUpdateWarning::LooseInterrogationSlot(
                                    *slot_id,
                                ),
                                op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(*slot_id)),
                            });
                        }
                    }
                }

                for (incompat_id, incompat) in &data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .incompats
                    .incompat_map
                {
                    if incompat.week_pattern_id == Some(*week_pattern_id) {
                        return Some(CleaningOp {
                            warning: WeekPatternsUpdateWarning::LooseScheduleIncompat(*incompat_id),
                            op: UpdateOp::Incompatibilities(
                                IncompatibilitiesUpdateOp::DeleteIncompat(*incompat_id),
                            ),
                        });
                    }
                }

                None
            }
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::WeekPatternId>, WeekPatternsUpdateError>
    {
        match self {
            Self::AddNewWeekPattern(week_pattern) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Add(
                                week_pattern.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("Unexpected error during AddNewWeekPattern");
                let Some(collomatique_state_colloscopes::NewId::WeekPatternId(new_id)) = result
                else {
                    panic!("Unexpected result from WeekPatternOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateWeekPattern(week_pattern_id, week_pattern) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Update(
                                *week_pattern_id,
                                week_pattern.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::WeekPattern(wpe) = e {
                            match wpe {
                                collomatique_state_colloscopes::WeekPatternError::InvalidWeekPatternId(id) =>
                                    UpdateWeekPatternError::InvalidWeekPatternId(id),
                                _ => panic!(
                                    "Unexpected week pattern error during UpdateWeekPattern: {:?}",
                                    wpe
                                ),
                            }
                        } else {
                            panic!("Unexpected error during UpdateWeekPattern: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteWeekPattern(week_pattern_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Remove(*week_pattern_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::WeekPattern(wpe) = e {
                            match wpe {
                                collomatique_state_colloscopes::WeekPatternError::InvalidWeekPatternId(id) =>
                                    DeleteWeekPatternError::InvalidWeekPatternId(id),
                                collomatique_state_colloscopes::WeekPatternError::WeekPatternStillHasAssociatedIncompat(_, _) => {
                                    panic!("Incompats should be cleaned before removing week patterns");
                                }
                                collomatique_state_colloscopes::WeekPatternError::WeekPatternStillHasAssociatedSlots(_, _) => {
                                    panic!("Slots should be cleaned before removing week patterns");
                                }
                                _ => panic!(
                                    "Unexpected week pattern error during DeleteWeekPattern: {:?}",
                                    wpe
                                ),
                            }
                        } else {
                            panic!("Unexpected error during DeleteWeekPattern: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::WeekPatterns,
            match self {
                WeekPatternsUpdateOp::AddNewWeekPattern(_desc) => {
                    "Ajouter un modèle de périodicité".into()
                }
                WeekPatternsUpdateOp::UpdateWeekPattern(_id, _desc) => {
                    "Modifier un modèle de périodicité".into()
                }
                WeekPatternsUpdateOp::DeleteWeekPattern(_id) => {
                    "Supprimer un modèle de périodicité".into()
                }
            },
        )
    }
}
