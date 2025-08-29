use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StudentsUpdateWarning {
    LooseStudentAssignmentForPeriod(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseExclusionFromGroupList(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::GroupListId,
    ),
    LoosePrefilledGroup(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::GroupListId,
    ),
}

impl StudentsUpdateWarning {
    pub fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            StudentsUpdateWarning::LooseStudentAssignmentForPeriod(student_id, period_id) => {
                let Some(student) = data.get_data().get_students().student_map.get(student_id)
                else {
                    return None;
                };
                let Some(period_index) = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte des inscriptions de {} {} sur la période {}",
                    student.desc.firstname,
                    student.desc.surname,
                    period_index + 1
                ))
            }
            Self::LooseExclusionFromGroupList(student_id, group_list_id) => {
                let Some(student) = data.get_data().get_students().student_map.get(student_id)
                else {
                    return None;
                };
                let Some(group_list) = data
                    .get_data()
                    .get_group_lists()
                    .group_list_map
                    .get(group_list_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte de l'exclusion de {} {} de la liste de groupes \"{}\"",
                    student.desc.firstname, student.desc.surname, group_list.params.name,
                ))
            }
            Self::LoosePrefilledGroup(student_id, group_list_id) => {
                let Some(student) = data.get_data().get_students().student_map.get(student_id)
                else {
                    return None;
                };
                let Some(group_list) = data
                    .get_data()
                    .get_group_lists()
                    .group_list_map
                    .get(group_list_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte du préremplissage de la liste de groupes \"{}\" avec {} {}",
                    group_list.params.name, student.desc.firstname, student.desc.surname,
                ))
            }
        }
    }
}

#[derive(Debug, Clone)]
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
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<StudentsUpdateWarning>> {
        match self {
            Self::AddNewStudent(_student) => None,
            Self::DeleteStudent(student_id) => {
                let Some(old_student) = data.get_data().get_students().student_map.get(student_id)
                else {
                    return None;
                };

                for (group_list_id, group_list) in &data.get_data().get_group_lists().group_list_map
                {
                    if group_list.prefilled_groups.contains_student(*student_id) {
                        let new_prefilled_groups = collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups {
                            groups: group_list.prefilled_groups.groups.iter().map(
                                |g| collomatique_state_colloscopes::group_lists::PrefilledGroup {
                                    sealed: g.sealed,
                                    students: g.students.iter().copied().filter(|id| *id != *student_id).collect(),
                                }
                            ).collect(),
                        };
                        return Some(CleaningOp {
                            warning: StudentsUpdateWarning::LoosePrefilledGroup(
                                *student_id,
                                *group_list_id,
                            ),
                            op: UpdateOp::GroupLists(GroupListsUpdateOp::PrefillGroupList(
                                *group_list_id,
                                new_prefilled_groups,
                            )),
                        });
                    }
                    if group_list.params.excluded_students.contains(student_id) {
                        let mut new_params = group_list.params.clone();
                        new_params.excluded_students.remove(student_id);
                        return Some(CleaningOp {
                            warning: StudentsUpdateWarning::LooseExclusionFromGroupList(
                                *student_id,
                                *group_list_id,
                            ),
                            op: UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(
                                *group_list_id,
                                new_params,
                            )),
                        });
                    }
                }

                for (period_id, period_assignments) in &data.get_data().get_assignments().period_map
                {
                    if old_student.excluded_periods.contains(period_id) {
                        continue;
                    }

                    for (subject_id, assigned_students) in &period_assignments.subject_map {
                        if assigned_students.contains(student_id) {
                            return Some(CleaningOp {
                                warning: StudentsUpdateWarning::LooseStudentAssignmentForPeriod(
                                    *student_id,
                                    *period_id,
                                ),
                                op: UpdateOp::Assignments(AssignmentsUpdateOp::Assign(
                                    *period_id,
                                    *student_id,
                                    *subject_id,
                                    false,
                                )),
                            });
                        }
                    }
                }

                None
            }
            Self::UpdateStudent(student_id, student) => {
                let Some(old_student) = data.get_data().get_students().student_map.get(student_id)
                else {
                    return None;
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
                            return Some(CleaningOp {
                                warning: StudentsUpdateWarning::LooseStudentAssignmentForPeriod(
                                    *student_id,
                                    *period_id,
                                ),
                                op: UpdateOp::Assignments(AssignmentsUpdateOp::Assign(
                                    *period_id,
                                    *student_id,
                                    *subject_id,
                                    false,
                                )),
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
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Student(
                            collomatique_state_colloscopes::StudentOp::Update(
                                *student_id,
                                student.clone(),
                            ),
                        ),
                        self.get_desc(),
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
                                collomatique_state_colloscopes::StudentError::StudentStillHasNonTrivialAssignments(_, _, _) => {
                                    panic!("Assignments should be cleaned before updating students");
                                }
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

                Ok(None)
            }
            Self::DeleteStudent(student_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Student(
                            collomatique_state_colloscopes::StudentOp::Remove(*student_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Student(se) = e {
                            match se {
                                collomatique_state_colloscopes::StudentError::InvalidStudentId(
                                    id,
                                ) => DeleteStudentError::InvalidStudentId(id),
                                collomatique_state_colloscopes::StudentError::StudentIsStillExcludedByGroupList(_, _) => {
                                    panic!("Group lists should be cleaned before removing students");
                                }
                                collomatique_state_colloscopes::StudentError::StudentIsStillReferencedByPrefilledGroupList(_, _) => {
                                    panic!("Prefilled group lists should be cleaned before removing students");
                                }
                                collomatique_state_colloscopes::StudentError::StudentStillHasNonTrivialAssignments(_, _, _) => {
                                    panic!("Assignments should be cleaned before removing students");
                                }
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

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Students,
            match self {
                StudentsUpdateOp::AddNewStudent(_desc) => "Ajouter un élève".into(),
                StudentsUpdateOp::UpdateStudent(_id, _desc) => "Modifier un élève".into(),
                StudentsUpdateOp::DeleteStudent(_id) => "Supprimer un élève".into(),
            },
        )
    }

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Vec<StudentsUpdateWarning> {
        match self {
            Self::AddNewStudent(_student) => vec![],
            Self::DeleteStudent(student_id) => {
                let mut output = vec![];

                for (group_list_id, group_list) in &data.get_data().get_group_lists().group_list_map
                {
                    if group_list.params.excluded_students.contains(student_id) {
                        output.push(StudentsUpdateWarning::LooseExclusionFromGroupList(
                            *student_id,
                            *group_list_id,
                        ));
                    }
                    if group_list.prefilled_groups.contains_student(*student_id) {
                        output.push(StudentsUpdateWarning::LoosePrefilledGroup(
                            *student_id,
                            *group_list_id,
                        ));
                    }
                }

                output
            }
            Self::UpdateStudent(student_id, student) => {
                let Some(old_student) = data.get_data().get_students().student_map.get(student_id)
                else {
                    return vec![];
                };

                let mut output = vec![];

                for (period_id, period_assignments) in &data.get_data().get_assignments().period_map
                {
                    if old_student.excluded_periods.contains(period_id) {
                        continue;
                    }
                    if !student.excluded_periods.contains(period_id) {
                        continue;
                    }

                    let mut has_assignments = false;
                    for (_subject_id, assigned_students) in &period_assignments.subject_map {
                        if assigned_students.contains(student_id) {
                            has_assignments = true;
                        }
                    }

                    if has_assignments {
                        output.push(StudentsUpdateWarning::LooseStudentAssignmentForPeriod(
                            *student_id,
                            *period_id,
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
                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

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
                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                for (group_list_id, group_list) in &data.get_data().get_group_lists().group_list_map
                {
                    if group_list.params.excluded_students.contains(student_id) {
                        let mut new_params = group_list.params.clone();
                        new_params.excluded_students.remove(student_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::GroupList(
                                    collomatique_state_colloscopes::GroupListOp::Update(
                                        *group_list_id,
                                        new_params,
                                    ),
                                ),
                                "Désactiver l'exclusion de l'élève d'une liste de groupes".into(),
                            )
                            .expect("All data should be valid at this point");

                        assert!(result.is_none());
                    }
                    if group_list.prefilled_groups.contains_student(*student_id) {
                        let mut new_prefilled_groups = group_list.prefilled_groups.clone();
                        new_prefilled_groups.remove_student(*student_id);
                        let result = session
                            .apply(
                                collomatique_state_colloscopes::Op::GroupList(
                                    collomatique_state_colloscopes::GroupListOp::PreFill(
                                        *group_list_id,
                                        new_prefilled_groups,
                                    ),
                                ),
                                "Retirer l'élève du préremplissage d'une liste de groupes".into(),
                            )
                            .expect("All data should be valid at this point");

                        assert!(result.is_none());
                    }
                }

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
