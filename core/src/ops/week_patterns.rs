use super::*;

#[derive(Debug, Clone)]
pub enum WeekPatternsUpdateWarning {
    LooseInterrogationSlot(collomatique_state_colloscopes::SlotId),
    LooseScheduleIncompat(collomatique_state_colloscopes::IncompatId),
}

impl WeekPatternsUpdateWarning {
    pub fn build_desc<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            WeekPatternsUpdateWarning::LooseInterrogationSlot(slot_id) => {
                let Some((subject_id, position)) = data
                    .get_data()
                    .get_slots()
                    .find_slot_subject_and_position(*slot_id)
                else {
                    return None;
                };
                let slot = &data
                    .get_data()
                    .get_slots()
                    .subject_map
                    .get(&subject_id)
                    .expect("Subject id should be valid at this point")
                    .ordered_slots[position]
                    .1;
                let Some(teacher) = data
                    .get_data()
                    .get_teachers()
                    .teacher_map
                    .get(&slot.teacher_id)
                else {
                    return None;
                };
                let Some(subject) = data.get_data().get_subjects().find_subject(subject_id) else {
                    return None;
                };
                Some(format!(
                    "Pertes du créneaux de colle du colleur {} {} pour la matière \"{}\" le {} à {}",
                    teacher.desc.firstname, teacher.desc.surname, subject.parameters.name, slot.start_time.weekday, slot.start_time.start_time,
                ))
            }
            Self::LooseScheduleIncompat(incompat_id) => {
                let Some(incompat) = data
                    .get_data()
                    .get_incompats()
                    .incompat_map
                    .get(incompat_id)
                else {
                    return None;
                };
                let Some(subject) = data
                    .get_data()
                    .get_subjects()
                    .find_subject(incompat.subject_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte d'une incompatibilité horaire le {} à {} pour la matière \"{}\"",
                    incompat.slot.start().weekday,
                    incompat.slot.start().start_time,
                    subject.parameters.name,
                ))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum WeekPatternsUpdateOp {
    AddNewWeekPattern(collomatique_state_colloscopes::week_patterns::WeekPattern),
    UpdateWeekPattern(
        collomatique_state_colloscopes::WeekPatternId,
        collomatique_state_colloscopes::week_patterns::WeekPattern,
    ),
    DeleteWeekPattern(collomatique_state_colloscopes::WeekPatternId),
}

#[derive(Debug, Error)]
pub enum WeekPatternsUpdateError {
    #[error(transparent)]
    UpdateWeekPattern(#[from] UpdateWeekPatternError),
    #[error(transparent)]
    DeleteWeekPattern(#[from] DeleteWeekPatternError),
}

#[derive(Debug, Error)]
pub enum UpdateWeekPatternError {
    #[error("Week pattern ID {0:?} is invalid")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

#[derive(Debug, Error)]
pub enum DeleteWeekPatternError {
    #[error("Week pattern ID {0:?} is invalid")]
    InvalidWeekPatternId(collomatique_state_colloscopes::WeekPatternId),
}

impl WeekPatternsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<WeekPatternsUpdateWarning>> {
        todo!()
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::WeekPatternId>, WeekPatternsUpdateError>
    {
        todo!()
    }

    pub fn get_desc(&self) -> String {
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
        }
    }

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Vec<WeekPatternsUpdateWarning> {
        match self {
            Self::AddNewWeekPattern(_) => vec![],
            Self::UpdateWeekPattern(_, _) => vec![],
            Self::DeleteWeekPattern(week_pattern_id) => {
                let mut output = vec![];

                for (_subject_id, subject_slots) in &data.get_data().get_slots().subject_map {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.week_pattern == Some(*week_pattern_id) {
                            output
                                .push(WeekPatternsUpdateWarning::LooseInterrogationSlot(*slot_id));
                        }
                    }
                }

                for (incompat_id, incompat) in &data.get_data().get_incompats().incompat_map {
                    if incompat.week_pattern_id == Some(*week_pattern_id) {
                        output.push(WeekPatternsUpdateWarning::LooseScheduleIncompat(
                            *incompat_id,
                        ));
                    }
                }

                output
            }
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
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
                        (OpCategory::WeekPatterns, self.get_desc()),
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
                        (OpCategory::WeekPatterns, self.get_desc()),
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
                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                for (_subject_id, subject_slots) in &data.get_data().get_slots().subject_map {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.week_pattern == Some(*week_pattern_id) {
                            let result = session
                                .apply(
                                    collomatique_state_colloscopes::Op::Slot(
                                        collomatique_state_colloscopes::SlotOp::Remove(*slot_id),
                                    ),
                                    "Suppression d'un créneau de colle utilisant le modèle".into(),
                                )
                                .expect("All data should be valid at this point");

                            assert!(result.is_none());
                        }
                    }
                }

                for (incompat_id, incompat) in &data.get_data().get_incompats().incompat_map {
                    if incompat.week_pattern_id == Some(*week_pattern_id) {
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Incompat(
                                    collomatique_state_colloscopes::IncompatOp::Remove(
                                        *incompat_id,
                                    ),
                                ),
                                "Suppression d'une incompatibilité horaire utilisant le modèle"
                                    .into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Remove(*week_pattern_id),
                        ),
                        "Suppression effective du modèle de périodicité".into(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::WeekPattern(wpe) = e {
                            match wpe {
                                collomatique_state_colloscopes::WeekPatternError::InvalidWeekPatternId(id) =>
                                    DeleteWeekPatternError::InvalidWeekPatternId(id),
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

                *data = session.commit((OpCategory::WeekPatterns, self.get_desc()));

                Ok(None)
            }
        }
    }
}
