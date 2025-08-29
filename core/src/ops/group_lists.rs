use super::*;

#[derive(Debug)]
pub enum GroupListsUpdateWarning {
    LoosePrefilledGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::StudentId,
    ),
    LooseSubjectAssociation(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

impl GroupListsUpdateWarning {
    pub fn build_desc<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            Self::LoosePrefilledGroupList(group_list_id, student_id) => {
                let Some(group_list) = data
                    .get_data()
                    .get_group_lists()
                    .group_list_map
                    .get(group_list_id)
                else {
                    return None;
                };
                let Some(student) = data.get_data().get_students().student_map.get(student_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte du préremplissage de la liste de groupe \"{}\" avec l'élève {} {}",
                    group_list.params.name, student.desc.firstname, student.desc.surname,
                ))
            }
            Self::LooseSubjectAssociation(group_list_id, subject_id, period_id) => {
                let Some(group_list) = data
                    .get_data()
                    .get_group_lists()
                    .group_list_map
                    .get(group_list_id)
                else {
                    return None;
                };
                let Some(subject) = data.get_data().get_subjects().find_subject(*subject_id) else {
                    return None;
                };
                let Some(period_num) = data
                    .get_data()
                    .get_periods()
                    .find_period_position(*period_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte de l'association de la matière \"{}\" à la liste de groupe \"{}\" pour la période {}",
                    subject.parameters.name, group_list.params.name, period_num+1
                ))
            }
        }
    }
}

#[derive(Debug)]
pub enum GroupListsUpdateOp {
    AddNewGroupList(collomatique_state_colloscopes::group_lists::GroupListParameters),
    UpdateGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::group_lists::GroupListParameters,
    ),
    DeleteGroupList(collomatique_state_colloscopes::GroupListId),
    PrefillGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups,
    ),
    AssignGroupListToSubject(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
        Option<collomatique_state_colloscopes::GroupListId>,
    ),
}

#[derive(Debug, Error)]
pub enum GroupListsUpdateError {
    #[error(transparent)]
    AddNewGroupList(#[from] AddNewGroupListError),
    #[error(transparent)]
    UpdateGroupList(#[from] UpdateGroupListError),
    #[error(transparent)]
    DeleteGroupList(#[from] DeleteGroupListError),
    #[error(transparent)]
    PrefillGroupList(#[from] PrefillGroupListError),
    #[error(transparent)]
    AssignGroupListToSubject(#[from] AssignGroupListToSubjectError),
}

#[derive(Debug, Error)]
pub enum AddNewGroupListError {
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("group_count range is empty")]
    GroupCountRangeIsEmpty,
    #[error("students_per_group range is empty")]
    StudentsPerGroupRangeIsEmpty,
}

#[derive(Debug, Error)]
pub enum UpdateGroupListError {
    #[error("Group list id ({0:?}) is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("group_count range is empty")]
    GroupCountRangeIsEmpty,
    #[error("students_per_group range is empty")]
    StudentsPerGroupRangeIsEmpty,
}

#[derive(Debug, Error)]
pub enum DeleteGroupListError {
    #[error("Group list ID {0:?} is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
}

#[derive(Debug, Error)]
pub enum PrefillGroupListError {
    #[error("Group list ID {0:?} is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("Group list {0:?} excludes student {1:?} who cannot be used for prefilled groups")]
    StudentIsExcluded(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::StudentId,
    ),
}

#[derive(Debug, Error)]
pub enum AssignGroupListToSubjectError {
    #[error("Group list ID {0:?} is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Subject {0:?} has no interrogation and does not need a group list")]
    SubjectHasNoInterrogation(collomatique_state_colloscopes::SubjectId),
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

impl GroupListsUpdateOp {
    pub fn get_desc(&self) -> String {
        match self {
            GroupListsUpdateOp::AddNewGroupList(_params) => "Ajouter une liste de groupes".into(),
            GroupListsUpdateOp::UpdateGroupList(_id, _params) => {
                "Modifier les paramètres d'une liste de groupes".into()
            }
            GroupListsUpdateOp::DeleteGroupList(_id) => "Supprimer une liste de groupes".into(),
            GroupListsUpdateOp::PrefillGroupList(_id, _prefilled_groups) => {
                "Modifier le préremplissage d'une liste de groupes".into()
            }
            GroupListsUpdateOp::AssignGroupListToSubject(
                _period_id,
                _subject_id,
                group_list_id,
            ) => {
                if group_list_id.is_some() {
                    "Affecter une liste de groupes à une matière".into()
                } else {
                    "Supprimer l'affectation d'une liste de groupes à une matière".into()
                }
            }
        }
    }

    pub fn get_warnings<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &T,
    ) -> Vec<GroupListsUpdateWarning> {
        match self {
            GroupListsUpdateOp::AddNewGroupList(_params) => vec![],
            GroupListsUpdateOp::UpdateGroupList(group_list_id, params) => {
                let Some(old_group_list) = data
                    .get_data()
                    .get_group_lists()
                    .group_list_map
                    .get(group_list_id)
                else {
                    return vec![];
                };

                let mut output = vec![];

                for student_id in old_group_list.prefilled_groups.iter_students() {
                    if params.excluded_students.contains(&student_id) {
                        output.push(GroupListsUpdateWarning::LoosePrefilledGroupList(
                            *group_list_id,
                            student_id,
                        ));
                    }
                }

                output
            }
            GroupListsUpdateOp::DeleteGroupList(group_list_id) => {
                let mut output = vec![];

                for (period_id, subject_map) in
                    &data.get_data().get_group_lists().subjects_associations
                {
                    for (subject_id, associated_id) in subject_map {
                        if *group_list_id == *associated_id {
                            output.push(GroupListsUpdateWarning::LooseSubjectAssociation(
                                *group_list_id,
                                *subject_id,
                                *period_id,
                            ));
                        }
                    }
                }

                output
            }
            GroupListsUpdateOp::PrefillGroupList(_id, _prefilled_groups) => vec![],
            GroupListsUpdateOp::AssignGroupListToSubject(
                _period_id,
                _subject_id,
                _group_list_id,
            ) => vec![],
        }
    }

    pub fn apply<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::GroupListId>, GroupListsUpdateError> {
        match self {
            Self::AddNewGroupList(params) => {
                for student_id in &params.excluded_students {
                    if !data
                        .get_data()
                        .get_students()
                        .student_map
                        .contains_key(student_id)
                    {
                        return Err(AddNewGroupListError::InvalidStudentId(*student_id).into());
                    }
                }

                if params.group_count.is_empty() {
                    return Err(AddNewGroupListError::GroupCountRangeIsEmpty.into());
                }
                if params.students_per_group.is_empty() {
                    return Err(AddNewGroupListError::StudentsPerGroupRangeIsEmpty.into());
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Add(params.clone()),
                        ),
                        (OpCategory::GroupLists, self.get_desc()),
                    )
                    .expect("All data should be valid at this point");
                let Some(collomatique_state_colloscopes::NewId::GroupListId(new_id)) = result
                else {
                    panic!("Unexpected result from GroupListOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateGroupList(group_list_id, params) => {
                for student_id in &params.excluded_students {
                    if !data
                        .get_data()
                        .get_students()
                        .student_map
                        .contains_key(student_id)
                    {
                        return Err(UpdateGroupListError::InvalidStudentId(*student_id).into());
                    }
                }

                if params.group_count.is_empty() {
                    return Err(UpdateGroupListError::GroupCountRangeIsEmpty.into());
                }
                if params.students_per_group.is_empty() {
                    return Err(UpdateGroupListError::StudentsPerGroupRangeIsEmpty.into());
                }

                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                let Some(group_list) = data
                    .get_data()
                    .get_group_lists()
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(UpdateGroupListError::InvalidGroupListId(*group_list_id).into());
                };

                let mut new_prefilled_groups = group_list.prefilled_groups.clone();
                let mut update_prefilled_groups = false;
                for student_id in group_list.prefilled_groups.iter_students() {
                    if params.excluded_students.contains(&student_id) {
                        new_prefilled_groups.remove_student(student_id);
                        update_prefilled_groups = true;
                    }
                }
                if update_prefilled_groups {
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::GroupList(
                                collomatique_state_colloscopes::GroupListOp::PreFill(
                                    *group_list_id,
                                    new_prefilled_groups,
                                ),
                            ),
                            "Mise à jour du préremplissage de la liste de groupes".into(),
                        )
                        .expect("All data should be valid at this point");
                    assert!(result.is_none());
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Update(
                                *group_list_id,
                                params.clone(),
                            ),
                        ),
                        "Mise à jour effective des paramètres de la liste de groupes".into(),
                    )
                    .expect("All data should be valid at this point");
                assert!(result.is_none());

                *data = session.commit((OpCategory::GroupLists, self.get_desc()));
                Ok(None)
            }
            Self::DeleteGroupList(group_list_id) => {
                let mut session = collomatique_state::AppSession::<_, String>::new(data.clone());

                let Some(group_list) = data
                    .get_data()
                    .get_group_lists()
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(DeleteGroupListError::InvalidGroupListId(*group_list_id).into());
                };

                if !group_list.prefilled_groups.is_empty() {
                    let result = session
                        .apply(
                            collomatique_state_colloscopes::Op::GroupList(
                                collomatique_state_colloscopes::GroupListOp::PreFill(
                                    *group_list_id,
                                    collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups::default(),
                                ),
                            ),
                            "Vidage du préremplissage de la liste de groupes".into(),
                        )
                        .expect("All data should be valid at this point");
                    assert!(result.is_none());
                }

                for (period_id, subject_map) in
                    &data.get_data().get_group_lists().subjects_associations
                {
                    for (subject_id, associated_id) in subject_map {
                        if *associated_id == *group_list_id {
                            let result = session
                                .apply(
                                    collomatique_state_colloscopes::Op::GroupList(
                                        collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                            *period_id,
                                            *subject_id,
                                            None,
                                        ),
                                    ),
                                    "Désassociation de la liste de groupes d'une matière".into(),
                                )
                                .expect("All data should be valid at this point");
                            assert!(result.is_none());
                        }
                    }
                }

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Remove(*group_list_id),
                        ),
                        "Suppression effective de la liste de groupes".into(),
                    )
                    .expect("All data should be valid at this point");
                assert!(result.is_none());

                *data = session.commit((OpCategory::GroupLists, self.get_desc()));
                Ok(None)
            }
            Self::PrefillGroupList(group_list_id, prefilled_groups) => {
                let Some(group_list) = data
                    .get_data()
                    .get_group_lists()
                    .group_list_map
                    .get(group_list_id)
                else {
                    return Err(PrefillGroupListError::InvalidGroupListId(*group_list_id).into());
                };

                for student_id in prefilled_groups.iter_students() {
                    if group_list.params.excluded_students.contains(&student_id) {
                        return Err(PrefillGroupListError::StudentIsExcluded(
                            *group_list_id,
                            student_id,
                        )
                        .into());
                    }
                    if !data
                        .get_data()
                        .get_students()
                        .student_map
                        .contains_key(&student_id)
                    {
                        return Err(PrefillGroupListError::InvalidStudentId(student_id).into());
                    }
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::PreFill(
                                *group_list_id,
                                prefilled_groups.clone(),
                            ),
                        ),
                        (OpCategory::GroupLists, self.get_desc()),
                    )
                    .expect("All data should be valid at this point");
                assert!(result.is_none());

                Ok(None)
            }
            Self::AssignGroupListToSubject(period_id, subject_id, group_list_id_opt) => {
                let Some(subject) = data.get_data().get_subjects().find_subject(*subject_id) else {
                    return Err(AssignGroupListToSubjectError::InvalidSubjectId(*subject_id).into());
                };

                if subject.parameters.interrogation_parameters.is_none() {
                    return Err(AssignGroupListToSubjectError::SubjectHasNoInterrogation(
                        *subject_id,
                    )
                    .into());
                }

                if subject.excluded_periods.contains(period_id) {
                    return Err(AssignGroupListToSubjectError::SubjectDoesNotRunOnPeriod(
                        *subject_id,
                        *period_id,
                    )
                    .into());
                }

                if !data
                    .get_data()
                    .get_group_lists()
                    .subjects_associations
                    .contains_key(period_id)
                {
                    return Err(AssignGroupListToSubjectError::InvalidPeriodId(*period_id).into());
                }

                if let Some(group_list_id) = group_list_id_opt {
                    if !data
                        .get_data()
                        .get_group_lists()
                        .group_list_map
                        .contains_key(group_list_id)
                    {
                        return Err(AssignGroupListToSubjectError::InvalidGroupListId(
                            *group_list_id,
                        )
                        .into());
                    }
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                *period_id,
                                *subject_id,
                                *group_list_id_opt,
                            ),
                        ),
                        (OpCategory::GroupLists, self.get_desc()),
                    )
                    .expect("All data should be valid at this point");
                assert!(result.is_none());

                Ok(None)
            }
        }
    }
}
