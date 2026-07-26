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
    LooseLimitsForStudent(collomatique_state_colloscopes::StudentId),
    LooseStudentInColloscopeGroup(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::GroupListId,
    ),
}

impl StudentsUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            StudentsUpdateWarning::LooseStudentAssignmentForPeriod(student_id, period_id) => {
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
                let Some(period_index) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
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
                    "Perte de l'exclusion de {} {} de la liste de groupes \"{}\"",
                    student.desc.firstname,
                    student.desc.surname,
                    group_list.params().name,
                ))
            }
            Self::LoosePrefilledGroup(student_id, group_list_id) => {
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
                    "Perte du préremplissage de la liste de groupes \"{}\" avec {} {}",
                    group_list.params().name,
                    student.desc.firstname,
                    student.desc.surname,
                ))
            }
            StudentsUpdateWarning::LooseLimitsForStudent(student_id) => {
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
                Some(format!(
                    "Perte des limites paramétrées pour l'élève {} {}",
                    student.desc.firstname, student.desc.surname,
                ))
            }
            Self::LooseStudentInColloscopeGroup(student_id, group_list_id) => {
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
                    "Perte de l'attribution de {} {} dans la liste de groupes \"{}\" dans le colloscope",
                    student.desc.firstname,
                    student.desc.surname,
                    group_list.params().name,
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StudentsUpdateOp {
    AddNewStudent(collomatique_state_colloscopes::students::Student),
    UpdateStudent(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::students::Student,
    ),
    DeleteStudent(collomatique_state_colloscopes::StudentId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum StudentsUpdateError {
    #[error(transparent)]
    AddNewStudent(#[from] AddNewStudentError),
    #[error(transparent)]
    UpdateStudent(#[from] UpdateStudentError),
    #[error(transparent)]
    DeleteStudent(#[from] DeleteStudentError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewStudentError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateStudentError {
    #[error("Student ID {0:?} is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
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
                let Some(old_student) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .get(student_id)
                else {
                    return None;
                };

                for (group_list_id, placements) in data
                    .get_data()
                    .get_inner_data()
                    .colloscope
                    .group_lists_iter()
                {
                    if placements.contains_key(student_id) {
                        let mut new_placements = placements.clone();
                        new_placements.remove(student_id);
                        return Some(CleaningOp {
                            warning: StudentsUpdateWarning::LooseStudentInColloscopeGroup(
                                *student_id,
                                group_list_id,
                            ),
                            op: UpdateOp::Colloscope(
                                ColloscopeUpdateOp::UpdateColloscopeGroupList(
                                    group_list_id,
                                    new_placements,
                                ),
                            ),
                        });
                    }
                }

                for (group_list_id, group_list) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .iter()
                {
                    let group_list_id = &group_list_id;
                    match group_list.filling() {
                        collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups } => {
                            if group_list.filling().contains_student(*student_id) {
                                let new_groups: Vec<_> = groups.iter().map(
                                    |g| collomatique_state_colloscopes::group_lists::PrefilledGroup {
                                        students: g.students.iter().copied().filter(|id| *id != *student_id).collect(),
                                    }
                                ).collect();
                                return Some(CleaningOp {
                                    warning: StudentsUpdateWarning::LoosePrefilledGroup(
                                        *student_id,
                                        *group_list_id,
                                    ),
                                    op: UpdateOp::GroupLists(GroupListsUpdateOp::SetFilling(
                                        *group_list_id,
                                        collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled {
                                            groups: new_groups,
                                        },
                                    )),
                                });
                            }
                        }
                        collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic { excluded_students } => {
                            if excluded_students.contains(student_id) {
                                let mut new_excluded = excluded_students.clone();
                                new_excluded.remove(student_id);
                                return Some(CleaningOp {
                                    warning: StudentsUpdateWarning::LooseExclusionFromGroupList(
                                        *student_id,
                                        *group_list_id,
                                    ),
                                    op: UpdateOp::GroupLists(GroupListsUpdateOp::SetFilling(
                                        *group_list_id,
                                        collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                                            excluded_students: new_excluded,
                                        },
                                    )),
                                });
                            }
                        }
                    }
                }

                for (period_id, subject_id, assigned_students) in
                    data.get_data().get_inner_data().params.assignments.iter()
                {
                    if old_student.excluded_periods.contains(&period_id) {
                        continue;
                    }

                    if assigned_students.contains(student_id) {
                        return Some(CleaningOp {
                            warning: StudentsUpdateWarning::LooseStudentAssignmentForPeriod(
                                *student_id,
                                period_id,
                            ),
                            op: UpdateOp::Assignments(AssignmentsUpdateOp::Assign(
                                period_id,
                                *student_id,
                                subject_id,
                                false,
                            )),
                        });
                    }
                }

                if data
                    .get_data()
                    .get_inner_data()
                    .params
                    .settings
                    .students
                    .contains(student_id)
                {
                    return Some(CleaningOp {
                        warning: StudentsUpdateWarning::LooseLimitsForStudent(*student_id),
                        op: UpdateOp::Settings(SettingsUpdateOp::RemoveStudentLimits(*student_id)),
                    });
                }

                None
            }
            Self::UpdateStudent(student_id, student) => {
                let Some(old_student) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .get(student_id)
                else {
                    return None;
                };

                for (period_id, subject_id, assigned_students) in
                    data.get_data().get_inner_data().params.assignments.iter()
                {
                    if old_student.excluded_periods.contains(&period_id) {
                        continue;
                    }
                    if !student.excluded_periods.contains(&period_id) {
                        continue;
                    }

                    if assigned_students.contains(student_id) {
                        return Some(CleaningOp {
                            warning: StudentsUpdateWarning::LooseStudentAssignmentForPeriod(
                                *student_id,
                                period_id,
                            ),
                            op: UpdateOp::Assignments(AssignmentsUpdateOp::Assign(
                                period_id,
                                *student_id,
                                subject_id,
                                false,
                            )),
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
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, PeriodRefSite, Reference,
                        };
                        match e {
                            // Pre-op validity: any period dangle in the set is this
                            // add's bad excluded-period id.
                            Error::Invariants(set) => {
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Period {
                                        target,
                                        site: PeriodRefSite::StudentExcludedPeriods(_),
                                    }) = inv
                                    {
                                        return AddNewStudentError::InvalidPeriodId(*target);
                                    }
                                }
                                panic!("Unexpected invariant breaks during AddNewStudent: {set:?}");
                            }
                            _ => panic!("Unexpected error during AddNewStudent: {e:?}"),
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
                        use collomatique_state_colloscopes::{
                            Convergence, Error, FixableInvariant, PeriodRefSite, PrecheckError,
                            Reference, StudentPrecheckError,
                        };
                        match e {
                            Error::Precheck(PrecheckError::Student(
                                StudentPrecheckError::InvalidStudentId(id),
                            )) => UpdateStudentError::InvalidStudentId(id),
                            Error::Invariants(set) => {
                                // Old order: validate_student (excluded-period ids)
                                // before the assignment scan.
                                for inv in &set {
                                    if let FixableInvariant::DanglingFk(Reference::Period {
                                        target,
                                        site: PeriodRefSite::StudentExcludedPeriods(_),
                                    }) = inv
                                    {
                                        return UpdateStudentError::InvalidPeriodId(*target);
                                    }
                                }
                                for inv in &set {
                                    if let FixableInvariant::Convergence(
                                        Convergence::AssignedStudentNotPresentForPeriod { .. },
                                    ) = inv
                                    {
                                        panic!(
                                            "Assignments should be cleaned before updating students"
                                        );
                                    }
                                }
                                panic!("Unexpected invariant breaks during UpdateStudent: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateStudent: {e:?}"),
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
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, PrecheckError, Reference,
                            StudentPrecheckError, StudentRefSite,
                        };
                        match e {
                            Error::Precheck(PrecheckError::Student(
                                StudentPrecheckError::InvalidStudentId(id),
                            )) => DeleteStudentError::InvalidStudentId(id),
                            Error::Invariants(set) => {
                                // Every one of these is a cleaning-contract breach:
                                // the cleaning phase strips group-list, prefilled and
                                // assignment references (and colloscope/settings ones)
                                // before the remove reaches apply.
                                for inv in &set {
                                    match inv {
                                        FixableInvariant::DanglingFk(Reference::Student {
                                            site: StudentRefSite::GroupListExcludedStudent(_),
                                            ..
                                        }) => panic!(
                                            "Group lists should be cleaned before removing students"
                                        ),
                                        FixableInvariant::DanglingFk(Reference::Student {
                                            site: StudentRefSite::GroupListPrefilledStudent(_),
                                            ..
                                        }) => panic!(
                                            "Prefilled group lists should be cleaned before removing students"
                                        ),
                                        FixableInvariant::DanglingFk(Reference::Student {
                                            site: StudentRefSite::AssignmentsStudent { .. },
                                            ..
                                        }) => panic!(
                                            "Assignments should be cleaned before removing students"
                                        ),
                                        _ => {}
                                    }
                                }
                                panic!(
                                    "Unexpected invariant breaks during DeleteStudent: {set:?}"
                                );
                            }
                            _ => panic!("Unexpected error during DeleteStudent: {e:?}"),
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
}
