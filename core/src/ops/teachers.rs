use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TeachersUpdateWarning {
    LooseInterrogationSlots(collomatique_state_colloscopes::TeacherId),
}

impl TeachersUpdateWarning {
    pub fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            TeachersUpdateWarning::LooseInterrogationSlots(teacher_id) => {
                let Some(teacher) = data.get_data().get_teachers().teacher_map.get(teacher_id)
                else {
                    return None;
                };
                Some(format!(
                    "Pertes des créneaux de colle du colleur {} {}",
                    teacher.desc.firstname, teacher.desc.surname,
                ))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum TeachersUpdateOp {
    AddNewTeacher(collomatique_state_colloscopes::teachers::Teacher),
    UpdateTeacher(
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::teachers::Teacher,
    ),
    DeleteTeacher(collomatique_state_colloscopes::TeacherId),
}

#[derive(Debug, Error)]
pub enum TeachersUpdateError {
    #[error(transparent)]
    AddNewTeacher(#[from] AddNewTeacherError),
    #[error(transparent)]
    UpdateTeacher(#[from] UpdateTeacherError),
    #[error(transparent)]
    DeleteTeacher(#[from] DeleteTeacherError),
}

#[derive(Debug, Error)]
pub enum AddNewTeacherError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Debug, Error)]
pub enum UpdateTeacherError {
    #[error("Teacher ID {0:?} is invalid")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Debug, Error)]
pub enum DeleteTeacherError {
    #[error("Teacher ID {0:?} is invalid")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
}

impl TeachersUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        _data: &T,
    ) -> Option<CleaningOp<TeachersUpdateWarning>> {
        todo!()
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::TeacherId>, TeachersUpdateError> {
        todo!()
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

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Vec<TeachersUpdateWarning> {
        match self {
            Self::AddNewTeacher(_) => vec![],
            Self::UpdateTeacher(teacher_id, teacher) => {
                let mut output = vec![];

                let mut loose_slot = false;
                for (subject_id, subject_slots) in &data.get_data().get_slots().subject_map {
                    if teacher.subjects.contains(subject_id) {
                        continue;
                    }
                    for (_slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.teacher_id == *teacher_id {
                            loose_slot = true;
                        }
                    }
                }
                if loose_slot {
                    output.push(TeachersUpdateWarning::LooseInterrogationSlots(*teacher_id));
                }

                output
            }
            Self::DeleteTeacher(teacher_id) => {
                let mut output = vec![];

                let mut loose_slot = false;
                for (_subject_id, subject_slots) in &data.get_data().get_slots().subject_map {
                    for (_slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.teacher_id == *teacher_id {
                            loose_slot = true;
                        }
                    }
                }
                if loose_slot {
                    output.push(TeachersUpdateWarning::LooseInterrogationSlots(*teacher_id));
                }

                output
            }
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::TeacherId>, TeachersUpdateError> {
        match self {
            Self::AddNewTeacher(teacher) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Teacher(
                            collomatique_state_colloscopes::TeacherOp::Add(teacher.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Teacher(te) = e {
                            match te {
                                collomatique_state_colloscopes::TeacherError::InvalidSubjectId(
                                    subject_id,
                                ) => AddNewTeacherError::InvalidSubjectId(subject_id),
                                _ => panic!(
                                    "Unexpected teacher error during AddNewTeacher: {:?}",
                                    te
                                ),
                            }
                        } else {
                            panic!("Unexpected error during AddNewTeacher: {:?}", e);
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::TeacherId(new_id)) = result else {
                    panic!("Unexpected result from TeacherOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateTeacher(teacher_id, teacher) => {
                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                for (subject_id, subject_slots) in &data.get_data().get_slots().subject_map {
                    if teacher.subjects.contains(subject_id) {
                        continue;
                    }
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.teacher_id == *teacher_id {
                            let result = session
                                .apply(
                                    collomatique_state_colloscopes::Op::Slot(
                                        collomatique_state_colloscopes::SlotOp::Remove(*slot_id),
                                    ),
                                    "Suppression d'un créneau pour le colleur".into(),
                                )
                                .expect("All data should be valid at this point");

                            assert!(result.is_none());
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Teacher(
                            collomatique_state_colloscopes::TeacherOp::Update(
                                *teacher_id,
                                teacher.clone(),
                            ),
                        ),
                        "Mise à jour effective du colleur".into(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Teacher(te) = e {
                            match te {
                                collomatique_state_colloscopes::TeacherError::InvalidTeacherId(
                                    id,
                                ) => UpdateTeacherError::InvalidTeacherId(id),
                                collomatique_state_colloscopes::TeacherError::InvalidSubjectId(
                                    id,
                                ) => UpdateTeacherError::InvalidSubjectId(id),
                                _ => panic!(
                                    "Unexpected teacher error during UpdateTeacher: {:?}",
                                    te
                                ),
                            }
                        } else {
                            panic!("Unexpected error during UpdateTeacher: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                *data = session.commit(self.get_desc());

                Ok(None)
            }
            Self::DeleteTeacher(teacher_id) => {
                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                for (_subject_id, subject_slots) in &data.get_data().get_slots().subject_map {
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.teacher_id == *teacher_id {
                            let result = session
                                .apply(
                                    collomatique_state_colloscopes::Op::Slot(
                                        collomatique_state_colloscopes::SlotOp::Remove(*slot_id),
                                    ),
                                    "Suppression d'un créneau pour le colleur".into(),
                                )
                                .expect("All data should be valid at this point");

                            assert!(result.is_none());
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Teacher(
                            collomatique_state_colloscopes::TeacherOp::Remove(*teacher_id),
                        ),
                        "Suppression effective du colleur".into(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Teacher(te) = e {
                            match te {
                                collomatique_state_colloscopes::TeacherError::InvalidTeacherId(
                                    id,
                                ) => DeleteTeacherError::InvalidTeacherId(id),
                                _ => panic!(
                                    "Unexpected teacher error during DeleteTeacher: {:?}",
                                    te
                                ),
                            }
                        } else {
                            panic!("Unexpected error during DeleteTeacher: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                *data = session.commit(self.get_desc());

                Ok(None)
            }
        }
    }
}
