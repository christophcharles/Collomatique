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
                                    op: UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(
                                        *group_list_id,
                                        collomatique_state_colloscopes::group_lists::GroupList::new(
                                            group_list.params().clone(),
                                            collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled {
                                                groups: new_groups,
                                            },
                                        )
                                        .expect("the rebuilt groups keep the existing group count"),
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
                                    op: UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(
                                        *group_list_id,
                                        collomatique_state_colloscopes::group_lists::GroupList::new(
                                            group_list.params().clone(),
                                            collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                                                excluded_students: new_excluded,
                                            },
                                        )
                                        .expect("an automatic filling never constrains the group count"),
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
                            Error::BrokenInvariants(set) => {
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
                            Convergence, Error, FixableInvariant, InvalidOp, PeriodRefSite,
                            PrecheckError, Reference, StudentPrecheckError,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Student(
                                StudentPrecheckError::InvalidStudentId(id),
                            ))) => UpdateStudentError::InvalidStudentId(id),
                            Error::BrokenInvariants(set) => {
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
                            Error, FixableInvariant, InvalidOp, PrecheckError, Reference,
                            StudentPrecheckError, StudentRefSite,
                        };
                        match e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Student(
                                StudentPrecheckError::InvalidStudentId(id),
                            ))) => DeleteStudentError::InvalidStudentId(id),
                            Error::BrokenInvariants(set) => {
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

    // Nothing outside the tests calls this yet: the `UpdateOp` dispatch that
    // does is the last commit of the family migration. Drop the attribute then.
    #[allow(dead_code)]
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::StudentId>, StudentsUpdateError> {
        match self {
            Self::AddNewStudent(student) => {
                let result = session
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
                        match &e {
                            // The pre-op state was valid, so any period dangle
                            // in the set was introduced by this Add: it is the
                            // bad excluded-period id of its own payload. And
                            // the cascade cannot take it back out — the student
                            // went back with the rolled-back op, so the map
                            // finds nobody holding that exclusion and the
                            // target is convicted.
                            Error::BrokenInvariants(set) => {
                                for inv in set {
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
                let result = session
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
                            Error, FixableInvariant, InvalidOp, PeriodRefSite, PrecheckError,
                            Reference, StudentPrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Student(se))) => {
                                match se {
                                    StudentPrecheckError::InvalidStudentId(id) => {
                                        UpdateStudentError::InvalidStudentId(*id)
                                    }
                                    StudentPrecheckError::StudentIdAlreadyExists(_) => panic!(
                                        "Unexpected StudentPrecheckError during UpdateStudent: {e:?}"
                                    ),
                                }
                            }
                            Error::BrokenInvariants(set) => {
                                // Same shape as the Add: the excluded-period
                                // ids are the payload's own, and the student in
                                // the state still holds their old set, so there
                                // is nothing there for the map to take out.
                                for inv in set {
                                    if let FixableInvariant::DanglingFk(Reference::Period {
                                        target,
                                        site: PeriodRefSite::StudentExcludedPeriods(_),
                                    }) = inv
                                    {
                                        return UpdateStudentError::InvalidPeriodId(*target);
                                    }
                                }
                                // The old body's second scan is gone with the
                                // cleaning: a student who now excludes a period
                                // they were assigned on is repaired by the
                                // cascade (`AssignedStudentNotPresentForPeriod`
                                // -> the row is rebuilt without them), never
                                // returned here.
                                panic!("Unexpected invariant breaks during UpdateStudent: {set:?}");
                            }
                            _ => panic!("Unexpected error during UpdateStudent: {e:?}"),
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteStudent(student_id) => {
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::Student(
                            collomatique_state_colloscopes::StudentOp::Remove(*student_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        use collomatique_state_colloscopes::{
                            Error, InvalidOp, PrecheckError, StudentPrecheckError,
                        };
                        match &e {
                            Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Student(se))) => {
                                match se {
                                    StudentPrecheckError::InvalidStudentId(id) => {
                                        DeleteStudentError::InvalidStudentId(*id)
                                    }
                                    StudentPrecheckError::StudentIdAlreadyExists(_) => panic!(
                                        "Unexpected StudentPrecheckError during DeleteStudent: {e:?}"
                                    ),
                                }
                            }
                            // The old `BrokenInvariants` arm — three
                            // cleaned-before panics and a catch-all — is gone:
                            // every place that named the student is repaired by
                            // the cascade, each repair logged.
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

#[cfg(test)]
mod tests {
    //! A student is named from five different places — the prefilled groups of
    //! a group list, the excluded set of an automatic one, the per-student
    //! settings, the assignments rows, and the colloscope's group placements —
    //! and references only the periods they are away for. So this is the
    //! family with the widest deletion cascade of all, and the one where the
    //! old world's three cleaned-before panics were doing the most work.
    //!
    //! Two of those five sites the frozen hogwarts base cannot show: both of
    //! its group lists are prefilled (so nobody is excluded from one) and it
    //! carries no colloscope at all. The delete fixture on hogwarts pins the
    //! other three, and a tiny in-process document — whose whole content reads
    //! at a glance — pins those two.
    //!
    //! What the fixtures pin, family-wide: the repairs the cascade had to make
    //! (as the exact [Fix] list, in application order) *and* the document they
    //! produced (rebuilt by replaying those very ops on the base), plus the
    //! whole error surface — the state layer's student precheck, translated by
    //! the update and the delete, and the dangling-period scan the payload
    //! causes on the add and the update.

    use super::*;
    use crate::test_utils::{fixes, hogwarts};
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        AssignmentOp, ColloscopeOp, Fix, GroupListOp, NewId, NonEmptyRangeInclusive, Op,
        PersonWithContact, SettingsOp, StudentOp,
        group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup},
        ids::{GroupListId, Id, PeriodId, StudentId, SubjectId},
        students::Student,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;

    fn student_by_name(data: &Data, surname: &str, firstname: &str) -> StudentId {
        data.get_inner_data()
            .params
            .students
            .student_map
            .iter()
            .find(|(_id, student)| {
                student.desc.surname == surname && student.desc.firstname == firstname
            })
            .map(|(id, _student)| id)
            .unwrap_or_else(|| {
                panic!("the fixture should have a student named {firstname} {surname}")
            })
    }

    fn group_list_by_name(data: &Data, name: &str) -> GroupListId {
        data.get_inner_data()
            .params
            .group_lists
            .group_list_map
            .iter()
            .find(|(_id, group_list)| group_list.params().name == name)
            .map(|(id, _group_list)| id)
            .unwrap_or_else(|| panic!("the fixture should have a group list named {name}"))
    }

    fn student_of(data: &Data, student: StudentId) -> Student {
        data.get_inner_data()
            .params
            .students
            .student_map
            .get(&student)
            .expect("the fixture's student should be live")
            .clone()
    }

    fn group_list_of(data: &Data, group_list: GroupListId) -> GroupList {
        data.get_inner_data()
            .params
            .group_lists
            .group_list_map
            .get(&group_list)
            .expect("the fixture's group list should be live")
            .clone()
    }

    /// The `n`-th period in display order.
    fn period_at(data: &Data, index: usize) -> PeriodId {
        data.get_inner_data()
            .params
            .periods
            .period_ids()
            .nth(index)
            .unwrap_or_else(|| panic!("the fixture should have at least {} periods", index + 1))
    }

    /// The prefilled group lists holding `student`, in id order — which is the
    /// order the cascade meets them, the reference site carrying the list id.
    fn prefilled_lists_holding(data: &Data, student: StudentId) -> Vec<GroupListId> {
        data.get_inner_data()
            .params
            .group_lists
            .group_list_map
            .iter()
            .filter(|(_id, group_list)| group_list.filling().contains_student(student))
            .map(|(id, _group_list)| id)
            .collect()
    }

    /// A prefilled group list with `student` taken out of the group holding
    /// them — the value the cascade is expected to write back.
    fn prefill_without(data: &Data, group_list: GroupListId, student: StudentId) -> GroupList {
        let old = group_list_of(data, group_list);
        let GroupListFilling::Prefilled { groups } = old.filling() else {
            panic!("the fixture's group list {group_list:?} should be prefilled");
        };
        let groups = groups
            .iter()
            .map(|group| PrefilledGroup {
                students: group
                    .students
                    .iter()
                    .copied()
                    .filter(|id| *id != student)
                    .collect(),
            })
            .collect();

        GroupList::new(old.params().clone(), GroupListFilling::Prefilled { groups })
            .expect("dropping a member changes neither the group count nor the names")
    }

    /// Every assignments row holding `student`, in key order — which is the
    /// order the cascade meets them — each already rebuilt without them.
    fn rows_holding(
        data: &Data,
        student: StudentId,
    ) -> Vec<(PeriodId, SubjectId, BTreeSet<StudentId>)> {
        data.get_inner_data()
            .params
            .assignments
            .iter()
            .filter(|(_period, _subject, students)| students.contains(&student))
            .map(|(period, subject, students)| {
                let mut rebuilt = students.clone();
                rebuilt.remove(&student);
                (period, subject, rebuilt)
            })
            .collect()
    }

    fn new_student(
        surname: &str,
        firstname: &str,
        excluded_periods: BTreeSet<PeriodId>,
    ) -> Student {
        Student {
            desc: PersonWithContact {
                surname: surname.into(),
                firstname: firstname.into(),
                tel: None,
                email: None,
            },
            excluded_periods,
        }
    }

    /// Ids no document ever issued.
    fn dangling_student() -> StudentId {
        unsafe { StudentId::new(1u64 << 40) }
    }

    fn dangling_period() -> PeriodId {
        unsafe { PeriodId::new(1u64 << 40) }
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects the cascade to have landed —
    /// each of them valid in that order, exactly as the cascade lands them.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::Students, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// Runs one op alone on `base` and hands back what the document became and
    /// what the cascade had to repair on the way.
    fn apply_alone(
        base: &AppState<Data, Desc>,
        op: &StudentsUpdateOp,
    ) -> (AppState<Data, Desc>, Vec<CascadeWarning>) {
        let mut session = CascadeSession::new(base.clone());
        op.apply_to_session(&mut session)
            .unwrap_or_else(|e| panic!("{op:?} should land, got {e:?}"));

        session.commit(op.get_desc())
    }

    /// A new student references only the periods they are away for, so nothing
    /// in the document can need repairing: the id comes back and the warning
    /// log stays empty.
    #[test]
    fn adding_a_student_creates_them_and_warns_about_nothing() {
        let base = hogwarts();
        let second_period = period_at(base.get_data(), 1);
        let luna = new_student("Lovegood", "Lucy", BTreeSet::from([second_period]));

        let mut session = CascadeSession::new(base.clone());
        let op = StudentsUpdateOp::AddNewStudent(luna.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("a live period is all this student names");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding a student returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(student_of(state.get_data(), new_id), luna);
    }

    /// Changing a student's name touches nothing that anything else depends
    /// on: the update lands alone.
    #[test]
    fn renaming_a_student_warns_about_nothing() {
        let base = hogwarts();
        let hermione = student_by_name(base.get_data(), "Granger", "Hermione");

        let mut renamed = student_of(base.get_data(), hermione);
        renamed.desc.surname = "Granger-Weasley".into();

        let op = StudentsUpdateOp::UpdateStudent(hermione, renamed.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::Student(StudentOp::Update(hermione, renamed))],
            )
            .get_data(),
        );
    }

    /// Deleting Hermione Granger cannot land alone: she is in a prefilled
    /// group of both group lists, she is the one student with limits of her
    /// own, and she is enrolled in seven subjects on each of the three
    /// periods. Every one of those is a reference to her, and the cascade
    /// takes them out one at a time, in the canonical order of the sites —
    /// prefilled groups (by list id), then settings, then the assignments rows
    /// (by period, then subject).
    ///
    /// Where the old world refused the removal until its own cleaning loop had
    /// emptied the way — and panicked with « … should be cleaned before
    /// removing students » if it had missed one — the family op now issues the
    /// single elementary removal and reports what it cost.
    #[test]
    fn deleting_a_student_takes_every_reference_to_them_with_it() {
        let base = hogwarts();
        let hermione = student_by_name(base.get_data(), "Granger", "Hermione");
        let lists = prefilled_lists_holding(base.get_data(), hermione);
        let rows = rows_holding(base.get_data(), hermione);
        assert_eq!(
            lists,
            vec![
                group_list_by_name(base.get_data(), "Liste principale"),
                group_list_by_name(base.get_data(), "Divination"),
            ],
            "the fixture should hold Hermione in both of its prefilled lists"
        );
        assert!(
            base.get_data()
                .get_inner_data()
                .params
                .settings
                .students
                .contains(&hermione),
            "the fixture's Hermione should be the one student with limits of her own"
        );
        assert_eq!(rows.len(), 21, "seven subjects on each of three periods");

        let mut expected_fixes = vec![
            Fix::RemoveStudentFromGroupListPrefill {
                group_list: lists[0],
                student: hermione,
                rebuilt: prefill_without(base.get_data(), lists[0], hermione),
            },
            Fix::RemoveStudentFromGroupListPrefill {
                group_list: lists[1],
                student: hermione,
                rebuilt: prefill_without(base.get_data(), lists[1], hermione),
            },
            Fix::ClearStudentSettings { student: hermione },
        ];
        let mut expected_ops = vec![
            Op::GroupList(GroupListOp::Update(
                lists[0],
                prefill_without(base.get_data(), lists[0], hermione),
            )),
            Op::GroupList(GroupListOp::Update(
                lists[1],
                prefill_without(base.get_data(), lists[1], hermione),
            )),
            Op::Settings(SettingsOp::SetStudent(hermione, None)),
        ];
        for (period, subject, rebuilt) in rows {
            expected_fixes.push(Fix::RemoveStudentFromAssignmentRow {
                period,
                subject,
                student: hermione,
                rebuilt: rebuilt.clone(),
            });
            expected_ops.push(Op::Assignment(AssignmentOp::SetRow(
                period, subject, rebuilt,
            )));
        }
        expected_ops.push(Op::Student(StudentOp::Remove(hermione)));

        let op = StudentsUpdateOp::DeleteStudent(hermione);
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(fixes(&warnings), expected_fixes);
        assert_eq!(
            state.get_data(),
            expected_document(&base, expected_ops).get_data(),
        );
    }

    /// The two sites hogwarts cannot show, on a document small enough to read
    /// whole: an automatic group list that *excludes* the student, and a
    /// colloscope row that places them in a group. Deleting them lifts the
    /// exclusion and takes their placement out of the row — the other placed
    /// student stays, which is what makes the rebuilt map worth asserting.
    #[test]
    fn deleting_a_student_lifts_their_exclusion_and_takes_their_colloscope_place() {
        let mut base = AppState::new(Data::default());
        let hermione = add_student(&mut base, "Granger", "Hermione");
        let ron = add_student(&mut base, "Weasley", "Ron");
        let excluding = add_automatic_list(&mut base, "Sortilèges", BTreeSet::from([hermione]));
        let placing = add_automatic_list(&mut base, "Astronomie", BTreeSet::new());
        set_placements(
            &mut base,
            placing,
            BTreeMap::from([(hermione, 0), (ron, 1)]),
        );

        let op = StudentsUpdateOp::DeleteStudent(hermione);
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::RemoveStudentGroupListExclusion {
                    group_list: excluding,
                    student: hermione,
                    rebuilt: automatic_list("Sortilèges", BTreeSet::new()),
                },
                Fix::RemoveStudentColloscopePlacement {
                    group_list: placing,
                    student: hermione,
                    rebuilt: BTreeMap::from([(ron, 1)]),
                },
            ],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::GroupList(GroupListOp::Update(
                        excluding,
                        automatic_list("Sortilèges", BTreeSet::new()),
                    )),
                    Op::Colloscope(ColloscopeOp::SetGroupList(
                        placing,
                        BTreeMap::from([(ron, 1)]),
                    )),
                    Op::Student(StudentOp::Remove(hermione)),
                ],
            )
            .get_data(),
        );
    }

    /// Marking a student away for a period contradicts every enrolment they
    /// hold on it — the checker's `AssignedStudentNotPresentForPeriod`. The old
    /// world cleaned those rows first and panicked if it had missed one; the
    /// cascade rebuilds each row without her, in subject order, and says so.
    /// Her enrolments on the other two periods are untouched.
    #[test]
    fn excluding_a_period_takes_the_student_out_of_that_period_s_rows() {
        let base = hogwarts();
        let hermione = student_by_name(base.get_data(), "Granger", "Hermione");
        let first_period = period_at(base.get_data(), 0);
        let rows: Vec<_> = rows_holding(base.get_data(), hermione)
            .into_iter()
            .filter(|(period, _subject, _rebuilt)| *period == first_period)
            .collect();
        assert_eq!(rows.len(), 7, "Hermione takes seven subjects on the period");

        let mut away = student_of(base.get_data(), hermione);
        away.excluded_periods.insert(first_period);

        let op = StudentsUpdateOp::UpdateStudent(hermione, away.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            rows.iter()
                .map(
                    |(period, subject, rebuilt)| Fix::RemoveStudentFromAssignmentRow {
                        period: *period,
                        subject: *subject,
                        student: hermione,
                        rebuilt: rebuilt.clone(),
                    }
                )
                .collect::<Vec<_>>(),
        );
        let mut expected_ops: Vec<_> = rows
            .into_iter()
            .map(|(period, subject, rebuilt)| {
                Op::Assignment(AssignmentOp::SetRow(period, subject, rebuilt))
            })
            .collect();
        expected_ops.push(Op::Student(StudentOp::Update(hermione, away)));
        assert_eq!(
            state.get_data(),
            expected_document(&base, expected_ops).get_data(),
        );
    }

    /// The state layer's own precheck, translated by the two ops that name an
    /// existing student. A rejected op changes nothing and logs nothing: the
    /// engine put the document back before the error came out.
    #[test]
    fn a_dead_student_id_is_rejected_by_update_and_by_delete() {
        let base = hogwarts();
        let dangling = dangling_student();

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            StudentsUpdateOp::UpdateStudent(
                dangling,
                new_student("Rusard", "Argus", BTreeSet::new())
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            StudentsUpdateError::UpdateStudent(UpdateStudentError::InvalidStudentId(dangling)),
        );
        assert_eq!(
            StudentsUpdateOp::DeleteStudent(dangling)
                .apply_to_session(&mut session)
                .unwrap_err(),
            StudentsUpdateError::DeleteStudent(DeleteStudentError::InvalidStudentId(dangling)),
        );

        assert_eq!(session.get_data(), base.get_data());
        let (_state, warnings) = session.commit((OpCategory::Students, "Rien".into()));
        assert!(warnings.is_empty(), "nothing was applied: {warnings:?}");
    }

    /// A period id the payload made up dangles the moment the op lands. No
    /// student in the state excludes that period — the payload went back with
    /// the rolled-back op — so no repair can help: the map answers nothing,
    /// the engine convicts the op, and the scan turns the break back into the
    /// bad input it came from.
    #[test]
    fn a_dead_period_id_is_rejected_on_add_and_on_update() {
        let base = hogwarts();
        let hermione = student_by_name(base.get_data(), "Granger", "Hermione");
        let dangling = dangling_period();

        let mut hermione_away_forever = student_of(base.get_data(), hermione);
        hermione_away_forever.excluded_periods.insert(dangling);

        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            StudentsUpdateOp::AddNewStudent(new_student(
                "Rusard",
                "Argus",
                BTreeSet::from([dangling])
            ))
            .apply_to_session(&mut session)
            .unwrap_err(),
            StudentsUpdateError::AddNewStudent(AddNewStudentError::InvalidPeriodId(dangling)),
        );
        assert_eq!(
            StudentsUpdateOp::UpdateStudent(hermione, hermione_away_forever)
                .apply_to_session(&mut session)
                .unwrap_err(),
            StudentsUpdateError::UpdateStudent(UpdateStudentError::InvalidPeriodId(dangling)),
        );

        assert_eq!(session.get_data(), base.get_data());
    }

    // ---- The tiny document of the exclusion/colloscope fixture.

    fn add_student(base: &mut AppState<Data, Desc>, surname: &str, firstname: &str) -> StudentId {
        match base.apply(
            Op::Student(StudentOp::Add(new_student(
                surname,
                firstname,
                BTreeSet::new(),
            ))),
            (OpCategory::Students, "Préparation".into()),
        ) {
            Ok(Some(NewId::StudentId(id))) => id,
            other => panic!("adding a student should hand back its id, got {other:?}"),
        }
    }

    /// An automatic group list with two unnamed groups, excluding `excluded`.
    fn automatic_list(name: &str, excluded: BTreeSet<StudentId>) -> GroupList {
        GroupList::new(
            GroupListParameters {
                name: name.into(),
                students_per_group: NonEmptyRangeInclusive::new(
                    NonZeroU32::new(1).unwrap()..=NonZeroU32::new(3).unwrap(),
                )
                .expect("statically non-empty"),
                group_names: vec![None, None],
            },
            GroupListFilling::Automatic {
                excluded_students: excluded,
            },
        )
        .expect("an automatic filling never constrains the group count")
    }

    fn add_automatic_list(
        base: &mut AppState<Data, Desc>,
        name: &str,
        excluded: BTreeSet<StudentId>,
    ) -> GroupListId {
        match base.apply(
            Op::GroupList(GroupListOp::Add(automatic_list(name, excluded))),
            (OpCategory::GroupLists, "Préparation".into()),
        ) {
            Ok(Some(NewId::GroupListId(id))) => id,
            other => panic!("adding a group list should hand back its id, got {other:?}"),
        }
    }

    fn set_placements(
        base: &mut AppState<Data, Desc>,
        group_list: GroupListId,
        placements: BTreeMap<StudentId, u32>,
    ) {
        base.apply(
            Op::Colloscope(ColloscopeOp::SetGroupList(group_list, placements)),
            (OpCategory::Colloscope, "Préparation".into()),
        )
        .expect("placing live students in a live automatic list breaks nothing");
    }
}
