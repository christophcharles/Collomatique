use super::*;

#[derive(Debug)]
pub enum StudentsUpdateOp {
    AddNewStudent(collomatique_state_colloscopes::students::Student),
    UpdateStudent(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::students::Student,
    ),
    DeleteStudent(collomatique_state_colloscopes::StudentId),
}

#[derive(Debug, Error)]
pub enum StudentsUpdateError {
    #[error(transparent)]
    AddNewStudent(#[from] AddNewStudentError),
    #[error(transparent)]
    UpdateStudent(#[from] UpdateStudentError),
    #[error(transparent)]
    DeleteStudent(#[from] DeleteStudentError),
}

#[derive(Debug, Error)]
pub enum AddNewStudentError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

#[derive(Debug, Error)]
pub enum UpdateStudentError {
    #[error("Student ID {0:?} is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

#[derive(Debug, Error)]
pub enum DeleteStudentError {
    #[error("Student ID {0:?} is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
}

impl StudentsUpdateOp {
    pub fn get_desc(&self) -> String {
        match self {
            StudentsUpdateOp::AddNewStudent(_desc) => "Ajouter un élève".into(),
            StudentsUpdateOp::UpdateStudent(_id, _desc) => "Modifier un élève".into(),
            StudentsUpdateOp::DeleteStudent(_id) => "Supprimer un élève".into(),
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::StudentId>, StudentsUpdateError> {
        match self {
            Self::AddNewStudent(student) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Student(
                            collomatique_state_colloscopes::StudentOp::Add(student.clone()),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Student(se) = e {
                            match se {
                                collomatique_state_colloscopes::StudentError::InvalidPeriodId(
                                    period_id,
                                ) => AddNewStudentError::InvalidPeriodId(period_id),
                                _ => panic!(
                                    "Unexpected student error during AddNewStudent: {:?}",
                                    se
                                ),
                            }
                        } else {
                            panic!("Unexpected error during AddNewStudent: {:?}", e);
                        }
                    })?;
                let Some(collomatique_state_colloscopes::NewId::StudentId(new_id)) = result else {
                    panic!("Unexpected result from StudentOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateStudent(student_id, student) => {
                let mut session = collomatique_state::AppSession::new(data.clone());

                let Some(old_student) = data.get_data().get_students().student_map.get(student_id)
                else {
                    return Err(UpdateStudentError::InvalidStudentId(*student_id).into());
                };

                for (period_id, period_assignments) in &data.get_data().get_assignments().period_map
                {
                    if old_student.excluded_periods.contains(period_id) {
                        continue;
                    }
                    if !student.excluded_periods.contains(period_id) {
                        continue;
                    }

                    for (subject_id, assigned_students) in &period_assignments.subject_map {
                        if assigned_students.contains(student_id) {
                            let result = session
                                .apply(
                                    collomatique_state_colloscopes::Op::Assignment(
                                        collomatique_state_colloscopes::AssignmentOp::Assign(
                                            *period_id,
                                            *student_id,
                                            *subject_id,
                                            false,
                                        ),
                                    ),
                                    "Restaurer l'état par défaut sur une affectation de l'élève"
                                        .into(),
                                )
                                .expect("All data should be valid at this point");

                            assert!(result.is_none());
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Student(
                            collomatique_state_colloscopes::StudentOp::Update(
                                *student_id,
                                student.clone(),
                            ),
                        ),
                        "Mise à jour effective de l'élève".into(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Student(se) = e {
                            match se {
                                collomatique_state_colloscopes::StudentError::InvalidStudentId(
                                    id,
                                ) => UpdateStudentError::InvalidStudentId(id),
                                collomatique_state_colloscopes::StudentError::InvalidPeriodId(
                                    id,
                                ) => UpdateStudentError::InvalidPeriodId(id),
                                _ => panic!(
                                    "Unexpected student error during UpdateStudent: {:?}",
                                    se
                                ),
                            }
                        } else {
                            panic!("Unexpected error during UpdateStudent: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                *data = session.commit(self.get_desc());

                Ok(None)
            }
            Self::DeleteStudent(student_id) => {
                let mut session = collomatique_state::AppSession::new(data.clone());

                for (period_id, period_assignments) in &data.get_data().get_assignments().period_map
                {
                    for (subject_id, assigned_students) in &period_assignments.subject_map {
                        if assigned_students.contains(student_id) {
                            let result = session
                                .apply(
                                    collomatique_state_colloscopes::Op::Assignment(
                                        collomatique_state_colloscopes::AssignmentOp::Assign(
                                            *period_id,
                                            *student_id,
                                            *subject_id,
                                            false,
                                        ),
                                    ),
                                    "Restaurer l'état par défaut sur une affectation de l'élève"
                                        .into(),
                                )
                                .expect("All data should be valid at this point");

                            assert!(result.is_none());
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Student(
                            collomatique_state_colloscopes::StudentOp::Remove(*student_id),
                        ),
                        "Suppression effective de l'élève".into(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Student(se) = e {
                            match se {
                                collomatique_state_colloscopes::StudentError::InvalidStudentId(
                                    id,
                                ) => DeleteStudentError::InvalidStudentId(id),
                                _ => panic!(
                                    "Unexpected teacher error during DeleteStudent: {:?}",
                                    se
                                ),
                            }
                        } else {
                            panic!("Unexpected error during DeleteStudent: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                *data = session.commit(self.get_desc());

                Ok(None)
            }
        }
    }
}
