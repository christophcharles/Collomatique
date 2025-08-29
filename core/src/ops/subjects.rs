use std::collections::BTreeSet;

use super::*;

#[derive(Debug)]
pub enum SubjectsUpdateWarning {
    LooseInterrogationDataForTeacher(
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::SubjectId,
    ),
    LooseStudentsAssignmentsForPeriod(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
    ),
}

impl SubjectsUpdateWarning {
    pub fn build_desc<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        data: &T,
    ) -> String {
        match self {
            Self::LooseInterrogationDataForTeacher(teacher_id, subject_id) => {
                let Some(teacher) = data.get_data().get_teachers().teacher_map.get(teacher_id)
                else {
                    return String::new();
                };
                let Some(subject) = data.get_data().get_subjects().find_subject(*subject_id) else {
                    return String::new();
                };
                format!(
                    "Désincription du colleur {} {} pour la matière \"{}\"",
                    teacher.desc.firstname, teacher.desc.surname, subject.parameters.name,
                )
            }
            Self::LooseStudentsAssignmentsForPeriod(period_id, subject_id) => {
                let Some(period_index) = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                else {
                    return String::new();
                };
                let Some(subject) = data.get_data().get_subjects().find_subject(*subject_id) else {
                    return String::new();
                };
                format!(
                    "Perte des inscriptions des élèves pour la matière \"{}\" sur la période {}",
                    subject.parameters.name,
                    period_index + 1
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum SubjectsUpdateOp {
    AddNewSubject(collomatique_state_colloscopes::subjects::SubjectParameters),
    UpdateSubject(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::subjects::SubjectParameters,
    ),
    DeleteSubject(collomatique_state_colloscopes::SubjectId),
    MoveUp(collomatique_state_colloscopes::SubjectId),
    MoveDown(collomatique_state_colloscopes::SubjectId),
    UpdatePeriodStatus(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
        bool,
    ),
}

#[derive(Debug, Error)]
pub enum SubjectsUpdateError {
    #[error(transparent)]
    AddNewSubject(#[from] AddNewSubjectError),
    #[error(transparent)]
    UpdateSubject(#[from] UpdateSubjectError),
    #[error(transparent)]
    DeleteSubject(#[from] DeleteSubjectError),
    #[error(transparent)]
    MoveUp(#[from] MoveUpError),
    #[error(transparent)]
    MoveDown(#[from] MoveDownError),
    #[error(transparent)]
    UpdatePeriodStatus(#[from] UpdatePeriodStatusError),
}

#[derive(Debug, Error)]
pub enum AddNewSubjectError {
    #[error("Students per group range should allow at least one value")]
    StudentsPerGroupRangeIsEmpty,
    #[error("Groups per interrogations range should allow at least one value")]
    GroupsPerInterrogationRangeIsEmpty,
    #[error("Interrogation count range should allow at least one value")]
    InterrogationCountRangeIsEmpty,
}

#[derive(Debug, Error)]
pub enum UpdateSubjectError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Students per group range should allow at least one value")]
    StudentsPerGroupRangeIsEmpty,
    #[error("Groups per interrogations range should allow at least one value")]
    GroupsPerInterrogationRangeIsEmpty,
    #[error("Interrogation count range should allow at least one value")]
    InterrogationCountRangeIsEmpty,
}

#[derive(Debug, Error)]
pub enum DeleteSubjectError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Debug, Error)]
pub enum MoveUpError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Subject is already the first subject")]
    NoUpperPosition,
}

#[derive(Debug, Error)]
pub enum MoveDownError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Subject is already the last subject")]
    NoLowerPosition,
}

#[derive(Debug, Error)]
pub enum UpdatePeriodStatusError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

impl SubjectsUpdateOp {
    pub fn get_desc(&self) -> String {
        match self {
            SubjectsUpdateOp::AddNewSubject(_desc) => "Ajouter une matière".into(),
            SubjectsUpdateOp::UpdateSubject(_id, _desc) => "Modifier une matière".into(),
            SubjectsUpdateOp::DeleteSubject(_id) => "Supprimer une matière".into(),
            SubjectsUpdateOp::MoveUp(_id) => "Remonter une matière".into(),
            SubjectsUpdateOp::MoveDown(_id) => "Descendre une matière".into(),
            Self::UpdatePeriodStatus(_subject_id, _period_id, status) => {
                if *status {
                    "Dispenser une matière sur une période".into()
                } else {
                    "Ne pas dispenser une matière sur une période".into()
                }
            }
        }
    }

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        data: &T,
    ) -> Vec<SubjectsUpdateWarning> {
        match self {
            Self::AddNewSubject(_) => vec![],
            Self::MoveUp(_) => vec![],
            Self::MoveDown(_) => vec![],
            Self::UpdateSubject(subject_id, params) => {
                let Some(current_subject) =
                    data.get_data().get_subjects().find_subject(*subject_id)
                else {
                    return vec![];
                };

                let mut output = vec![];

                let previously_had_interrogations = current_subject
                    .parameters
                    .interrogation_parameters
                    .is_some();

                let no_more_interrogations = params.interrogation_parameters.is_none();

                if previously_had_interrogations && no_more_interrogations {
                    for (teacher_id, teacher) in &data.get_data().get_teachers().teacher_map {
                        if teacher.subjects.contains(subject_id) {
                            output.push(SubjectsUpdateWarning::LooseInterrogationDataForTeacher(
                                *teacher_id,
                                *subject_id,
                            ));
                        }
                    }
                }

                output
            }
            Self::UpdatePeriodStatus(subject_id, period_id, new_status) => {
                let Some(current_subject) =
                    data.get_data().get_subjects().find_subject(*subject_id)
                else {
                    return vec![];
                };

                let mut output = vec![];

                let old_status = !current_subject.excluded_periods.contains(period_id);

                if !*new_status && old_status {
                    let Some(period_assignments) =
                        data.get_data().get_assignments().period_map.get(period_id)
                    else {
                        return vec![];
                    };

                    let assigned_students = period_assignments
                        .subject_map
                        .get(subject_id)
                        .expect("subject_id should be available in subject map at this point");

                    if !assigned_students.is_empty() {
                        output.push(SubjectsUpdateWarning::LooseStudentsAssignmentsForPeriod(
                            *period_id,
                            *subject_id,
                        ));
                    }
                }

                output
            }
            Self::DeleteSubject(subject_id) => {
                let mut output = vec![];

                for (teacher_id, teacher) in &data.get_data().get_teachers().teacher_map {
                    if teacher.subjects.contains(subject_id) {
                        output.push(SubjectsUpdateWarning::LooseInterrogationDataForTeacher(
                            *teacher_id,
                            *subject_id,
                        ));
                    }
                }

                let Some(subject) = &data.get_data().get_subjects().find_subject(*subject_id)
                else {
                    return vec![];
                };

                let excluded_periods = &subject.excluded_periods;

                for (period_id, period_assignments) in &data.get_data().get_assignments().period_map
                {
                    if excluded_periods.contains(period_id) {
                        continue;
                    }
                    let assigned_students = period_assignments.subject_map.get(subject_id)
                        .expect("Assignment data is inconsistent and does not have a required subject entry");

                    if !assigned_students.is_empty() {
                        output.push(SubjectsUpdateWarning::LooseStudentsAssignmentsForPeriod(
                            *period_id,
                            *subject_id,
                        ));
                    }
                }

                output
            }
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::SubjectId>, SubjectsUpdateError> {
        match self {
            Self::AddNewSubject(params) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::AddAfter(
                                data.get_data().get_subjects().ordered_subject_list.last().map(|x| x.0),
                                collomatique_state_colloscopes::Subject {
                                    parameters: params.clone(),
                                    excluded_periods: BTreeSet::new(),
                                }
                            )
                        ),
                        self.get_desc()
                    ).map_err(|e| if let collomatique_state_colloscopes::Error::Subject(se) = e {
                        match se {
                            collomatique_state_colloscopes::SubjectError::GroupsPerInterrogationRangeIsEmpty => AddNewSubjectError::GroupsPerInterrogationRangeIsEmpty,
                            collomatique_state_colloscopes::SubjectError::StudentsPerGroupRangeIsEmpty => AddNewSubjectError::StudentsPerGroupRangeIsEmpty,
                            collomatique_state_colloscopes::SubjectError::InterrogationCountRangeIsEmpty => AddNewSubjectError::InterrogationCountRangeIsEmpty,
                            _ => panic!("Unexpected subject error during AddNewSubject: {:?}", se),
                        }
                    } else {
                        panic!("Unexpected error during AddNewSubject: {:?}", e);
                    })?;
                let Some(collomatique_state_colloscopes::NewId::SubjectId(new_id)) = result else {
                    panic!("Unexpected result from SubjectOp::AddAfter");
                };
                Ok(Some(new_id))
            }
            Self::UpdateSubject(subject_id, params) => {
                let current_subject = data
                    .get_data()
                    .get_subjects()
                    .find_subject(*subject_id)
                    .ok_or(UpdateSubjectError::InvalidSubjectId(*subject_id))?;

                let excluded_periods = current_subject.excluded_periods.clone();

                let previously_had_interrogations = current_subject
                    .parameters
                    .interrogation_parameters
                    .is_some();

                let no_more_interrogations = params.interrogation_parameters.is_none();

                let mut session = collomatique_state::AppSession::new(data.clone());

                if previously_had_interrogations && no_more_interrogations {
                    for (teacher_id, teacher) in &data.get_data().get_teachers().teacher_map {
                        if teacher.subjects.contains(subject_id) {
                            let mut new_teacher = teacher.clone();
                            new_teacher.subjects.remove(subject_id);
                            let result = session
                                .apply(
                                    collomatique_state_colloscopes::Op::Teacher(
                                        collomatique_state_colloscopes::TeacherOp::Update(
                                            *teacher_id,
                                            new_teacher,
                                        ),
                                    ),
                                    "Enlever une référence à la matière à mettre à jour".into(),
                                )
                                .expect("All data should be valid at this point");
                            if result.is_some() {
                                panic!("Unexpected result! {:?}", result);
                            }
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Update(
                                *subject_id,
                                collomatique_state_colloscopes::Subject {
                                    parameters: params.clone(),
                                    excluded_periods,
                                }
                            )
                        ),
                        "Mise à jour effective de la matière".into(),
                    ).map_err(|e| if let collomatique_state_colloscopes::Error::Subject(se) = e {
                        match se {
                            collomatique_state_colloscopes::SubjectError::InvalidSubjectId(_id) => panic!("Subject ID should be valid at this point"),
                            collomatique_state_colloscopes::SubjectError::GroupsPerInterrogationRangeIsEmpty => UpdateSubjectError::GroupsPerInterrogationRangeIsEmpty,
                            collomatique_state_colloscopes::SubjectError::StudentsPerGroupRangeIsEmpty => UpdateSubjectError::StudentsPerGroupRangeIsEmpty,
                            collomatique_state_colloscopes::SubjectError::InterrogationCountRangeIsEmpty => UpdateSubjectError::InterrogationCountRangeIsEmpty,
                            _ => panic!("Unexpected subject error during UpdateSubject: {:?}", se),
                        }
                    } else {
                        panic!("Unexpected error during UpdateSubject: {:?}", e);
                    })?;

                assert!(result.is_none());

                *data = session.commit(self.get_desc());

                Ok(None)
            }
            Self::DeleteSubject(subject_id) => {
                let mut session = collomatique_state::AppSession::new(data.clone());

                for (teacher_id, teacher) in &data.get_data().get_teachers().teacher_map {
                    if teacher.subjects.contains(subject_id) {
                        let mut new_teacher = teacher.clone();
                        new_teacher.subjects.remove(subject_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::Teacher(
                                    collomatique_state_colloscopes::TeacherOp::Update(
                                        *teacher_id,
                                        new_teacher,
                                    ),
                                ),
                                "Enlever une référence à la matière à effacer".into(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let excluded_periods = &data
                    .get_data()
                    .get_subjects()
                    .find_subject(*subject_id)
                    .ok_or(UpdateSubjectError::InvalidSubjectId(*subject_id))?
                    .excluded_periods;

                for (period_id, period_assignments) in &data.get_data().get_assignments().period_map
                {
                    if excluded_periods.contains(period_id) {
                        continue;
                    }
                    let assigned_students = period_assignments.subject_map.get(subject_id)
                        .expect("Assignment data is inconsistent and does not have a required subject entry");

                    for student_id in assigned_students {
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
                                "Valeur par défaut pour l'affectation d'un élève".into(),
                            )
                            .expect("All data should be valid at this point");

                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Remove(*subject_id),
                        ),
                        "Suppression effective de la matière".into(),
                    )
                    .expect("All data should be valid at this point");

                assert!(result.is_none());

                *data = session.commit(self.get_desc());

                Ok(None)
            }
            Self::MoveUp(subject_id) => {
                let current_position = data
                    .get_data()
                    .get_subjects()
                    .find_subject_position(*subject_id)
                    .ok_or(MoveUpError::InvalidSubjectId(*subject_id))?;

                if current_position == 0 {
                    Err(MoveUpError::NoUpperPosition)?;
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::ChangePosition(
                                *subject_id,
                                current_position - 1,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("No error should be possible at this point");

                assert!(result.is_none());

                Ok(None)
            }
            Self::MoveDown(subject_id) => {
                let current_position = data
                    .get_data()
                    .get_subjects()
                    .find_subject_position(*subject_id)
                    .ok_or(MoveDownError::InvalidSubjectId(*subject_id))?;

                if current_position == data.get_data().get_subjects().ordered_subject_list.len() - 1
                {
                    Err(MoveDownError::NoLowerPosition)?;
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::ChangePosition(
                                *subject_id,
                                current_position + 1,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("No error should be possible at this point");

                assert!(result.is_none());

                Ok(None)
            }
            Self::UpdatePeriodStatus(subject_id, period_id, new_status) => {
                if data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                    .is_none()
                {
                    Err(UpdatePeriodStatusError::InvalidPeriodId(*period_id))?;
                }

                let mut session = collomatique_state::AppSession::new(data.clone());

                let mut subject = data
                    .get_data()
                    .get_subjects()
                    .find_subject(*subject_id)
                    .ok_or(UpdatePeriodStatusError::InvalidSubjectId(*subject_id))?
                    .clone();

                let old_status = !subject.excluded_periods.contains(period_id);

                if *new_status {
                    subject.excluded_periods.remove(period_id);
                } else {
                    if old_status {
                        let period_assignments = data
                            .get_data()
                            .get_assignments()
                            .period_map
                            .get(period_id)
                            .expect("Period id should be valid at this point");

                        let assigned_students = period_assignments
                            .subject_map
                            .get(subject_id)
                            .expect("subject_id should be available in subject map at this point");

                        for student_id in assigned_students {
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
                                    "Restauration de l'état par défaut d'un élève".into(),
                                )
                                .expect("No error should be possible at this point");
                            assert!(result.is_none());
                        }
                    }
                    subject.excluded_periods.insert(*period_id);
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Update(*subject_id, subject),
                        ),
                        "Mise à jour effective du statut de la période".into(),
                    )
                    .expect("No error should be possible at this point");
                assert!(result.is_none());

                *data = session.commit(self.get_desc());

                Ok(None)
            }
        }
    }
}
