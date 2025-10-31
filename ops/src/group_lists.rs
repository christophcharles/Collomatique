use collomatique_state_colloscopes::group_lists::GroupListPrefilledGroups;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupListsUpdateWarning {
    LooseWholePrefilledGroupList(collomatique_state_colloscopes::GroupListId),
    LooseStudentsInPrefilledGroupList(
        collomatique_state_colloscopes::GroupListId,
        Vec<collomatique_state_colloscopes::StudentId>,
    ),
    LooseSubjectAssociation(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

impl GroupListsUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            Self::LooseWholePrefilledGroupList(group_list_id) => {
                let Some(group_list) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return None;
                };

                Some(format!(
                    "Perte complète du préremplissage de la liste de groupe \"{}\"",
                    group_list.params.name
                ))
            }
            Self::LooseStudentsInPrefilledGroupList(group_list_id, student_ids) => {
                let Some(group_list) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return None;
                };
                let mut student_names = vec![];
                for student_id in student_ids {
                    let Some(student) = data
                        .get_data()
                        .get_inner_data()
                        .params
                        .students
                        .student_map
                        .get(student_id)
                    else {
                        return None;
                    };
                    student_names.push(format!(
                        "{} {}",
                        student.desc.firstname, student.desc.surname,
                    ));
                }

                Some(format!(
                    "Perte du préremplissage de la liste de groupe \"{}\" avec les élèves: {}",
                    group_list.params.name,
                    student_names.join(", ")
                ))
            }
            Self::LooseSubjectAssociation(group_list_id, subject_id, period_id) => {
                let Some(group_list) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return None;
                };
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                else {
                    return None;
                };
                let Some(period_num) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    DuplicatePreviousPeriod(collomatique_state_colloscopes::PeriodId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
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
    #[error(transparent)]
    DuplicatePreviousPeriod(#[from] DuplicatePreviousPeriodAssociationsError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewGroupListError {
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("group_count range is empty")]
    GroupCountRangeIsEmpty,
    #[error("students_per_group range is empty")]
    StudentsPerGroupRangeIsEmpty,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteGroupListError {
    #[error("Group list ID {0:?} is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DuplicatePreviousPeriodAssociationsError {
    /// period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    /// trying to override first period
    #[error("given period ({0:?}) is the first period")]
    FirstPeriodHasNoPreviousPeriod(collomatique_state_colloscopes::PeriodId),
}

impl GroupListsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<GroupListsUpdateWarning>> {
        match self {
            GroupListsUpdateOp::AddNewGroupList(_params) => None,
            GroupListsUpdateOp::UpdateGroupList(group_list_id, params) => {
                let Some(old_group_list) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return None;
                };

                let mut students_to_exclude = vec![];
                let mut new_prefilled_groups = old_group_list.prefilled_groups.clone();
                for student_id in old_group_list.prefilled_groups.iter_students() {
                    if params.excluded_students.contains(&student_id) {
                        new_prefilled_groups.remove_student(student_id);
                        students_to_exclude.push(student_id);
                    }
                }
                if !students_to_exclude.is_empty() {
                    return Some(CleaningOp {
                        warning: GroupListsUpdateWarning::LooseStudentsInPrefilledGroupList(
                            *group_list_id,
                            students_to_exclude,
                        ),
                        op: UpdateOp::GroupLists(GroupListsUpdateOp::PrefillGroupList(
                            *group_list_id,
                            new_prefilled_groups,
                        )),
                    });
                }

                None
            }
            GroupListsUpdateOp::DeleteGroupList(group_list_id) => {
                let Some(old_group_list) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                else {
                    return None;
                };

                if !old_group_list.prefilled_groups.is_empty() {
                    return Some(CleaningOp {
                        warning: GroupListsUpdateWarning::LooseWholePrefilledGroupList(
                            *group_list_id,
                        ),
                        op: UpdateOp::GroupLists(GroupListsUpdateOp::PrefillGroupList(
                            *group_list_id,
                            GroupListPrefilledGroups { groups: vec![] },
                        )),
                    });
                }

                for (period_id, subject_map) in &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                {
                    for (subject_id, associated_id) in subject_map {
                        if *group_list_id == *associated_id {
                            return Some(CleaningOp {
                                warning: GroupListsUpdateWarning::LooseSubjectAssociation(
                                    *group_list_id,
                                    *subject_id,
                                    *period_id,
                                ),
                                op: UpdateOp::GroupLists(
                                    GroupListsUpdateOp::AssignGroupListToSubject(
                                        *period_id,
                                        *subject_id,
                                        None,
                                    ),
                                ),
                            });
                        }
                    }
                }

                None
            }
            GroupListsUpdateOp::PrefillGroupList(_id, _prefilled_groups) => None,
            GroupListsUpdateOp::AssignGroupListToSubject(
                _period_id,
                _subject_id,
                _group_list_id,
            ) => None,
            GroupListsUpdateOp::DuplicatePreviousPeriod(_period_id) => None,
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::GroupListId>, GroupListsUpdateError> {
        match self {
            Self::AddNewGroupList(params) => {
                for student_id in &params.excluded_students {
                    if !data
                        .get_data()
                        .get_inner_data()
                        .params
                        .students
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
                        self.get_desc(),
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
                        .get_inner_data()
                        .params
                        .students
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

                if !data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .contains_key(group_list_id)
                {
                    return Err(UpdateGroupListError::InvalidGroupListId(*group_list_id).into());
                };

                let result = match data
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Update(
                                *group_list_id,
                                params.clone(),
                            ),
                        ),
                        self.get_desc(),
                    ) {
                        Ok(r) => r,
                        Err(collomatique_state_colloscopes::Error::GroupList(ge)) => match ge {
                            collomatique_state_colloscopes::GroupListError::StudentBothIncludedAndExcluded(_) => panic!("Prefilled groups should be properly cleaned"),
                            _ => panic!("Unexpected error when calling GroupListOp::Update")
                        }
                        _ => panic!("Unexpected error when calling GroupListOp::Update")
                    };
                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteGroupList(group_list_id) => {
                if !data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .contains_key(group_list_id)
                {
                    return Err(DeleteGroupListError::InvalidGroupListId(*group_list_id).into());
                };

                let result = match data
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Remove(*group_list_id),
                        ),
                        self.get_desc(),
                    ) {
                        Ok(r) => r,
                        Err(collomatique_state_colloscopes::Error::GroupList(ge)) => match ge {
                            collomatique_state_colloscopes::GroupListError::RemainingPrefilledGroups => panic!("Prefilled groups should be properly cleaned"),
                            collomatique_state_colloscopes::GroupListError::RemainingAssociatedSubjects => panic!("Associated subjects should be properly cleaned"),
                            _ => panic!("Unexpected error when calling GroupListOp::Remove")
                        }
                        _ => panic!("Unexpected error when calling GroupListOp::Remove")
                    };
                assert!(result.is_none());

                Ok(None)
            }
            Self::PrefillGroupList(group_list_id, prefilled_groups) => {
                let Some(group_list) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
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
                        .get_inner_data()
                        .params
                        .students
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
                        self.get_desc(),
                    )
                    .expect("All data should be valid at this point");
                assert!(result.is_none());

                Ok(None)
            }
            Self::AssignGroupListToSubject(period_id, subject_id, group_list_id_opt) => {
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                else {
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
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .contains_key(period_id)
                {
                    return Err(AssignGroupListToSubjectError::InvalidPeriodId(*period_id).into());
                }

                if let Some(group_list_id) = group_list_id_opt {
                    if !data
                        .get_data()
                        .get_inner_data()
                        .params
                        .group_lists
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
                        self.get_desc(),
                    )
                    .expect("All data should be valid at this point");
                assert!(result.is_none());

                Ok(None)
            }
            Self::DuplicatePreviousPeriod(period_id) => {
                let Some(position) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return Err(DuplicatePreviousPeriodAssociationsError::InvalidPeriodId(
                        period_id.clone(),
                    )
                    .into());
                };

                if position == 0 {
                    return Err(
                        DuplicatePreviousPeriodAssociationsError::FirstPeriodHasNoPreviousPeriod(
                            period_id.clone(),
                        )
                        .into(),
                    );
                }

                let previous_period_id = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[position - 1]
                    .0;
                let previous_period_assignments = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .get(&previous_period_id)
                    .expect("Previous period id should be valid at this point")
                    .clone();

                let subjects = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                    .clone();

                for (subject_id, subject) in &subjects {
                    if subject.excluded_periods.contains(period_id) {
                        continue;
                    }
                    if subject.excluded_periods.contains(&previous_period_id) {
                        continue;
                    }
                    if subject.parameters.interrogation_parameters.is_none() {
                        continue;
                    }

                    let previous_group_list_id =
                        previous_period_assignments.get(subject_id).cloned();

                    let result = data
                        .apply(
                            collomatique_state_colloscopes::Op::GroupList(
                                collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                    *period_id,
                                    *subject_id,
                                    previous_group_list_id,
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("All data should be valid at this point");
                    assert!(result.is_none());
                }

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::GroupLists,
            match self {
                GroupListsUpdateOp::AddNewGroupList(_params) => {
                    "Ajouter une liste de groupes".into()
                }
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
                GroupListsUpdateOp::DuplicatePreviousPeriod(_period_id) => {
                    "Dupliquer les listes de groupes d'une période".into()
                }
            },
        )
    }
}
