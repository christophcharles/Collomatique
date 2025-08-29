use super::*;

#[derive(Debug)]
pub enum TeachersUpdateWarning {}

impl TeachersUpdateWarning {
    pub fn build_desc<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        _data: &T,
    ) -> String {
        String::new()
    }
}

#[derive(Debug)]
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
    pub fn get_desc(&self) -> String {
        match self {
            TeachersUpdateOp::AddNewTeacher(_desc) => "Ajouter un colleur".into(),
            TeachersUpdateOp::UpdateTeacher(_id, _desc) => "Modifier un colleur".into(),
            TeachersUpdateOp::DeleteTeacher(_id) => "Supprimer un colleur".into(),
        }
    }

    pub fn get_warnings(&self) -> Vec<TeachersUpdateWarning> {
        vec![]
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data>>(
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
                let mut session = collomatique_state::AppSession::new(data.clone());

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
