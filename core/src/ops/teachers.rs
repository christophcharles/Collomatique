use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TeachersUpdateWarning {
    LooseInterrogationSlots(collomatique_state_colloscopes::TeacherId),
    LooseColloscopeLinkWithTeacher(
        collomatique_state_colloscopes::ColloscopeId,
        collomatique_state_colloscopes::TeacherId,
    ),
}

impl TeachersUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            TeachersUpdateWarning::LooseInterrogationSlots(teacher_id) => {
                let Some(teacher) = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .teachers
                    .teacher_map
                    .get(teacher_id)
                else {
                    return None;
                };
                Some(format!(
                    "Pertes des créneaux de colle du colleur {} {}",
                    teacher.desc.firstname, teacher.desc.surname,
                ))
            }
            TeachersUpdateWarning::LooseColloscopeLinkWithTeacher(colloscope_id, teacher_id) => {
                let Some(colloscope) = data
                    .get_data()
                    .get_inner_data()
                    .colloscopes
                    .colloscope_map
                    .get(colloscope_id)
                else {
                    return None;
                };
                let Some(teacher) = data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .teachers
                    .teacher_map
                    .get(teacher_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte de la possibilité de mettre à jour le colloscope \"{}\" sur les paramètres du colleur {} {}",
                    colloscope.name,
                    teacher.desc.firstname,
                    teacher.desc.surname,
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TeachersUpdateOp {
    AddNewTeacher(
        collomatique_state_colloscopes::teachers::Teacher<
            collomatique_state_colloscopes::SubjectId,
        >,
    ),
    UpdateTeacher(
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::teachers::Teacher<
            collomatique_state_colloscopes::SubjectId,
        >,
    ),
    DeleteTeacher(collomatique_state_colloscopes::TeacherId),
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum TeachersUpdateError {
    #[error(transparent)]
    AddNewTeacher(#[from] AddNewTeacherError),
    #[error(transparent)]
    UpdateTeacher(#[from] UpdateTeacherError),
    #[error(transparent)]
    DeleteTeacher(#[from] DeleteTeacherError),
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum AddNewTeacherError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum UpdateTeacherError {
    #[error("Teacher ID {0:?} is invalid")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum DeleteTeacherError {
    #[error("Teacher ID {0:?} is invalid")]
    InvalidTeacherId(collomatique_state_colloscopes::TeacherId),
}

impl TeachersUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<TeachersUpdateWarning>> {
        match self {
            Self::AddNewTeacher(_) => None,
            Self::UpdateTeacher(teacher_id, teacher) => {
                for (subject_id, subject_slots) in &data
                    .get_data()
                    .get_inner_data()
                    .main_params
                    .slots
                    .subject_map
                {
                    if teacher.subjects.contains(subject_id) {
                        continue;
                    }
                    for (slot_id, slot) in &subject_slots.ordered_slots {
                        if slot.teacher_id == *teacher_id {
                            return Some(CleaningOp {
                                warning: TeachersUpdateWarning::LooseInterrogationSlots(
                                    *teacher_id,
                                ),
                                op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(*slot_id)),
                            });
                        }
                    }
                }

                None
            }
            Self::DeleteTeacher(teacher_id) => {
                for (colloscope_id, colloscope) in
                    &data.get_data().get_inner_data().colloscopes.colloscope_map
                {
                    if colloscope.id_maps.teachers.contains_key(teacher_id) {
                        let mut new_colloscope = colloscope.clone();
                        new_colloscope.id_maps.teachers.remove(teacher_id);

                        return Some(CleaningOp {
                            warning: TeachersUpdateWarning::LooseColloscopeLinkWithTeacher(
                                *colloscope_id,
                                *teacher_id,
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
                        if slot.teacher_id == *teacher_id {
                            return Some(CleaningOp {
                                warning: TeachersUpdateWarning::LooseInterrogationSlots(
                                    *teacher_id,
                                ),
                                op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(*slot_id)),
                            });
                        }
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
                let result = data
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
                        if let collomatique_state_colloscopes::Error::Teacher(te) = e {
                            match te {
                                collomatique_state_colloscopes::TeacherError::InvalidTeacherId(
                                    id,
                                ) => UpdateTeacherError::InvalidTeacherId(id),
                                collomatique_state_colloscopes::TeacherError::InvalidSubjectId(
                                    id,
                                ) => UpdateTeacherError::InvalidSubjectId(id),
                                collomatique_state_colloscopes::TeacherError::TeacherStillHasAssociatedSlotsInSubject(_, _) => {
                                    panic!("Slots should be cleaned before updating subjects for teacher");
                                }
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

                Ok(None)
            }
            Self::DeleteTeacher(teacher_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Teacher(
                            collomatique_state_colloscopes::TeacherOp::Remove(*teacher_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Teacher(te) = e {
                            match te {
                                collomatique_state_colloscopes::TeacherError::InvalidTeacherId(
                                    id,
                                ) => DeleteTeacherError::InvalidTeacherId(id),
                                collomatique_state_colloscopes::TeacherError::TeacherStillHasAssociatedSlots(_, _) => {
                                    panic!("Slots should be cleaned before removing teacher");
                                }
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
