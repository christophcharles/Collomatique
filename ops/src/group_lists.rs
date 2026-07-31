use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupListsUpdateWarning {
    LooseWholePrefilledGroupList(collomatique_state_colloscopes::GroupListId),
    LooseExcludedStudents(
        collomatique_state_colloscopes::GroupListId,
        Vec<collomatique_state_colloscopes::StudentId>,
    ),
    LooseSubjectAssociation(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseGroupListInColloscope(collomatique_state_colloscopes::GroupListId),
    LooseInterrogationsInColloscope(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseStudentGroupInColloscope(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::StudentId,
    ),
    LooseGroupsInInterrogationsInColloscope(
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
                    group_list.params().name
                ))
            }
            Self::LooseExcludedStudents(group_list_id, student_ids) => {
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
                    "Perte des élèves exclus de la liste de groupe \"{}\": {}",
                    group_list.params().name,
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
                    subject.parameters.name,
                    group_list.params().name,
                    period_num + 1
                ))
            }
            Self::LooseGroupListInColloscope(group_list_id) => {
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
                    "Perte du remplissage de la liste de groupe \"{}\" dans le colloscope",
                    group_list.params().name
                ))
            }
            Self::LooseInterrogationsInColloscope(subject_id, period_id) => {
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
                    "Perte des colles de la matière \"{}\" pour la période {}",
                    subject.parameters.name,
                    period_num + 1
                ))
            }
            Self::LooseStudentGroupInColloscope(group_list_id, student_id) => {
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
                    "Perte de l'affectation du l'élève {} {} dans la liste de groupe \"{}\" du le colloscope",
                    student.desc.firstname,
                    student.desc.surname,
                    group_list.params().name
                ))
            }
            Self::LooseGroupsInInterrogationsInColloscope(subject_id, period_id) => {
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
                    "Perte des groupes devenus invalides sur les colles de la matière \"{}\" pour la période {}",
                    subject.parameters.name,
                    period_num + 1
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GroupListsUpdateOp {
    AddNewGroupList(collomatique_state_colloscopes::group_lists::GroupList),
    /// Replaces a whole group list — parameters *and* filling — with the
    /// sealed value the caller supplies.
    UpdateGroupList(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::group_lists::GroupList,
    ),
    DeleteGroupList(collomatique_state_colloscopes::GroupListId),
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
    AssignGroupListToSubject(#[from] AssignGroupListToSubjectError),
    #[error(transparent)]
    DuplicatePreviousPeriod(#[from] DuplicatePreviousPeriodAssociationsError),
}

/// The payload carries a filling, which can name students, so adding a list
/// can fail on a dangling student id.
#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewGroupListError {
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateGroupListError {
    #[error("Group list id ({0:?}) is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
    #[error("Student id ({0:?}) is invalid")]
    InvalidStudentId(collomatique_state_colloscopes::StudentId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteGroupListError {
    #[error("Group list ID {0:?} is invalid")]
    InvalidGroupListId(collomatique_state_colloscopes::GroupListId),
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

/// Every student id a filling names, whichever variant it is.
///
/// `GroupListFilling::iter_students` deliberately covers the prefilled groups
/// only, but a student-existence sweep must also see the excluded set of an
/// automatic filling.
fn students_of(
    filling: &collomatique_state_colloscopes::group_lists::GroupListFilling,
) -> impl Iterator<Item = collomatique_state_colloscopes::StudentId> + '_ {
    filling
        .iter_students()
        .chain(filling.excluded_students().iter().copied())
}

impl GroupListsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<GroupListsUpdateWarning>> {
        match self {
            GroupListsUpdateOp::AddNewGroupList(_group_list) => None,
            GroupListsUpdateOp::UpdateGroupList(group_list_id, new_group_list) => {
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

                let new_count = new_group_list.params().group_names.len();

                // The payload carries both halves, so every check reads the
                // payload against the stored data.
                //
                // What is *not* checked here is the difference between the old
                // filling and the payload's. The payload is the caller's whole
                // description of the list, already validated: a group they
                // deleted, or a student they took out of a group, is their own
                // edit, not collateral damage this layer discovered for them.
                // Cleaning is for the data that hangs off the list — colloscope
                // placements and interrogation cells — which the caller never
                // saw.

                // Groups that the new count no longer has cannot stay in the
                // colloscope placement row. (A prefilled list has no placement
                // row, hence the guard on the payload's filling.)
                if !new_group_list.is_prefilled()
                    && let Some(placements) = data
                        .get_data()
                        .get_inner_data()
                        .colloscope
                        .group_list(*group_list_id)
                {
                    for (student_id, group) in placements {
                        if (*group as usize) >= new_count {
                            let mut new_placements = placements.clone();
                            new_placements.remove(student_id);
                            return Some(CleaningOp {
                                warning: GroupListsUpdateWarning::LooseStudentGroupInColloscope(
                                    *group_list_id,
                                    *student_id,
                                ),
                                op: UpdateOp::Colloscope(
                                    ColloscopeUpdateOp::UpdateColloscopeGroupList(
                                        *group_list_id,
                                        new_placements,
                                    ),
                                ),
                            });
                        }
                    }
                }

                let inner = data.get_data().get_inner_data();
                for ((assoc_period, subject_id), associated_group_list) in
                    inner.params.group_lists.subjects_associations.iter()
                {
                    if *associated_group_list != *group_list_id {
                        continue;
                    }
                    let Some(subject_slots) = inner.params.slots.slots_for_subject(subject_id)
                    else {
                        continue;
                    };
                    let slot_ids: Vec<_> = subject_slots.map(|(slot_id, _slot)| *slot_id).collect();
                    for slot_id in slot_ids {
                        for (week_id, groups) in inner.colloscope.interrogations_for_slot(slot_id) {
                            let (row_period, _pos) = inner
                                .params
                                .weeks
                                .week_position(week_id)
                                .expect("week id from a live colloscope row is valid");
                            if row_period != assoc_period {
                                continue;
                            }
                            let new_assigned_groups: std::collections::BTreeSet<u32> = groups
                                .iter()
                                .copied()
                                .filter(|group| (*group as usize) < new_count)
                                .collect();
                            if new_assigned_groups.len() != groups.len() {
                                return Some(CleaningOp {
                                    warning: GroupListsUpdateWarning::LooseGroupsInInterrogationsInColloscope(subject_id, assoc_period),
                                    op: UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                        slot_id,
                                        week_id,
                                        new_assigned_groups,
                                    )),
                                });
                            }
                        }
                    }
                }

                // Newly-excluded students cannot keep a group in the colloscope.
                if let collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                    excluded_students,
                } = new_group_list.filling()
                    && !old_group_list.is_prefilled()
                    && let Some(placements) = data
                        .get_data()
                        .get_inner_data()
                        .colloscope
                        .group_list(*group_list_id)
                {
                    for student_id in placements.keys() {
                        if excluded_students.contains(student_id) {
                            let mut new_placements = placements.clone();
                            new_placements.remove(student_id);
                            return Some(CleaningOp {
                                warning: GroupListsUpdateWarning::LooseStudentGroupInColloscope(
                                    *group_list_id,
                                    *student_id,
                                ),
                                op: UpdateOp::Colloscope(
                                    ColloscopeUpdateOp::UpdateColloscopeGroupList(
                                        *group_list_id,
                                        new_placements,
                                    ),
                                ),
                            });
                        }
                    }
                }

                // Becoming prefilled retires the placement row altogether.
                if !old_group_list.is_prefilled()
                    && new_group_list.is_prefilled()
                    && let Some(placements) = data
                        .get_data()
                        .get_inner_data()
                        .colloscope
                        .group_list(*group_list_id)
                    && let Some((student_id, _)) = placements.iter().next()
                {
                    let mut new_placements = placements.clone();
                    new_placements.remove(student_id);
                    return Some(CleaningOp {
                        warning: GroupListsUpdateWarning::LooseStudentGroupInColloscope(
                            *group_list_id,
                            *student_id,
                        ),
                        op: UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeGroupList(
                            *group_list_id,
                            new_placements,
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

                let inner = data.get_data().get_inner_data();
                for ((assoc_period, subject_id), associated_group_list) in
                    inner.params.group_lists.subjects_associations.iter()
                {
                    if *associated_group_list != *group_list_id {
                        continue;
                    }
                    let Some(subject_slots) = inner.params.slots.slots_for_subject(subject_id)
                    else {
                        continue;
                    };
                    let slot_ids: Vec<_> = subject_slots.map(|(slot_id, _slot)| *slot_id).collect();
                    for slot_id in slot_ids {
                        for (week_id, _groups) in inner.colloscope.interrogations_for_slot(slot_id)
                        {
                            let (row_period, _pos) = inner
                                .params
                                .weeks
                                .week_position(week_id)
                                .expect("week id from a live colloscope row is valid");
                            if row_period != assoc_period {
                                continue;
                            }
                            return Some(CleaningOp {
                                warning: GroupListsUpdateWarning::LooseInterrogationsInColloscope(
                                    subject_id,
                                    assoc_period,
                                ),
                                op: UpdateOp::Colloscope(
                                    ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                        slot_id,
                                        week_id,
                                        std::collections::BTreeSet::new(),
                                    ),
                                ),
                            });
                        }
                    }
                }

                // Clear excluded_students if Automatic mode with non-empty excluded_students
                if let collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                    excluded_students,
                } = old_group_list.filling()
                    && !excluded_students.is_empty()
                {
                    return Some(CleaningOp {
                            warning: GroupListsUpdateWarning::LooseExcludedStudents(
                                *group_list_id,
                                excluded_students.iter().copied().collect(),
                            ),
                            op: UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(
                                *group_list_id,
                                collomatique_state_colloscopes::group_lists::GroupList::new(
                                    old_group_list.params().clone(),
                                    collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
                                        excluded_students: std::collections::BTreeSet::new(),
                                    },
                                )
                                .expect("an automatic filling never constrains the group count"),
                            )),
                        });
                }

                if !old_group_list.is_prefilled() {
                    if data
                        .get_data()
                        .get_inner_data()
                        .colloscope
                        .group_list(*group_list_id)
                        .is_some()
                    {
                        return Some(CleaningOp {
                            warning: GroupListsUpdateWarning::LooseGroupListInColloscope(
                                *group_list_id,
                            ),
                            op: UpdateOp::Colloscope(
                                ColloscopeUpdateOp::UpdateColloscopeGroupList(
                                    *group_list_id,
                                    std::collections::BTreeMap::new(),
                                ),
                            ),
                        });
                    }
                } else {
                    return Some(CleaningOp {
                        warning: GroupListsUpdateWarning::LooseWholePrefilledGroupList(
                            *group_list_id,
                        ),
                        op: UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(
                            *group_list_id,
                            collomatique_state_colloscopes::group_lists::GroupList::new(
                                old_group_list.params().clone(),
                                collomatique_state_colloscopes::group_lists::GroupListFilling::default(),
                            )
                            .expect("the default filling is automatic, so it never constrains the group count"),
                        )),
                    });
                }

                for ((period_id, subject_id), associated_id) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                {
                    if *group_list_id == *associated_id {
                        return Some(CleaningOp {
                            warning: GroupListsUpdateWarning::LooseSubjectAssociation(
                                *group_list_id,
                                subject_id,
                                period_id,
                            ),
                            op: UpdateOp::GroupLists(GroupListsUpdateOp::AssignGroupListToSubject(
                                period_id, subject_id, None,
                            )),
                        });
                    }
                }

                None
            }
            GroupListsUpdateOp::AssignGroupListToSubject(period_id, subject_id, group_list_id) => {
                let inner = data.get_data().get_inner_data();
                let new_group_list = match group_list_id {
                    Some(id) => match inner.params.group_lists.group_list_map.get(id) {
                        Some(group_list) => Some(group_list),
                        None => return None,
                    },
                    None => None,
                };
                let first_forbidden_group_number = match new_group_list {
                    Some(group_list) => group_list.params().group_names.len() as u32,
                    None => 0,
                };

                let Some(subject_slots) = inner.params.slots.slots_for_subject(*subject_id) else {
                    return None;
                };
                let slot_ids: Vec<_> = subject_slots.map(|(slot_id, _slot)| *slot_id).collect();

                for slot_id in slot_ids {
                    for (week_id, groups) in inner.colloscope.interrogations_for_slot(slot_id) {
                        let (row_period, _pos) = inner
                            .params
                            .weeks
                            .week_position(week_id)
                            .expect("week id from a live colloscope row is valid");
                        if row_period != *period_id {
                            continue;
                        }
                        let new_assigned_groups: std::collections::BTreeSet<u32> = groups
                            .iter()
                            .copied()
                            .filter(|group| *group < first_forbidden_group_number)
                            .collect();
                        if new_assigned_groups.len() != groups.len() {
                            return Some(CleaningOp {
                                warning:
                                    GroupListsUpdateWarning::LooseGroupsInInterrogationsInColloscope(
                                        *subject_id,
                                        *period_id,
                                    ),
                                op: UpdateOp::Colloscope(
                                    ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                        slot_id,
                                        week_id,
                                        new_assigned_groups,
                                    ),
                                ),
                            });
                        }
                    }
                }

                None
            }
            GroupListsUpdateOp::DuplicatePreviousPeriod(period_id) => {
                let inner = data.get_data().get_inner_data();
                let Some(position) = inner.params.periods.find_period_position(*period_id) else {
                    return None;
                };

                if position == 0 {
                    return None;
                }

                let previous_period_id = inner
                    .params
                    .periods
                    .period_id_at(position - 1)
                    .expect("position > 0 checked above");
                let previous_period_assignments: std::collections::BTreeMap<_, _> = inner
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                    .filter_map(|((period, subject), group_list)| {
                        (period == previous_period_id).then_some((subject, *group_list))
                    })
                    .collect();

                for (subject_id, subject) in inner.params.subjects.ordered_subject_list.iter() {
                    if subject.excluded_periods.contains(period_id) {
                        continue;
                    }
                    if subject.excluded_periods.contains(&previous_period_id) {
                        continue;
                    }
                    if subject.parameters.interrogation_parameters.is_none() {
                        continue;
                    }

                    let group_list_id = previous_period_assignments.get(&subject_id);
                    let new_group_list = match group_list_id {
                        Some(id) => match inner.params.group_lists.group_list_map.get(id) {
                            Some(group_list) => Some(group_list),
                            None => return None,
                        },
                        None => None,
                    };
                    let first_forbidden_group_number = match new_group_list {
                        Some(group_list) => group_list.params().group_names.len() as u32,
                        None => 0,
                    };

                    let Some(subject_slots) = inner.params.slots.slots_for_subject(subject_id)
                    else {
                        return None;
                    };
                    let slot_ids: Vec<_> = subject_slots.map(|(slot_id, _slot)| *slot_id).collect();

                    for slot_id in slot_ids {
                        for (week_id, groups) in inner.colloscope.interrogations_for_slot(slot_id) {
                            let (row_period, _pos) = inner
                                .params
                                .weeks
                                .week_position(week_id)
                                .expect("week id from a live colloscope row is valid");
                            if row_period != *period_id {
                                continue;
                            }
                            let new_assigned_groups: std::collections::BTreeSet<u32> = groups
                                .iter()
                                .copied()
                                .filter(|group| *group < first_forbidden_group_number)
                                .collect();
                            if new_assigned_groups.len() != groups.len() {
                                return Some(CleaningOp {
                                    warning: GroupListsUpdateWarning::LooseGroupsInInterrogationsInColloscope(subject_id, *period_id),
                                    op: UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                        slot_id,
                                        week_id,
                                        new_assigned_groups,
                                    )),
                                });
                            }
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
    ) -> Result<Option<collomatique_state_colloscopes::GroupListId>, GroupListsUpdateError> {
        match self {
            Self::AddNewGroupList(group_list) => {
                // The payload is a sealed `GroupList`, so its internal
                // consistency is already settled; only student *existence* — a
                // state-dependent fact — is left to check here.
                for student_id in students_of(group_list.filling()) {
                    if !data
                        .get_data()
                        .get_inner_data()
                        .params
                        .students
                        .student_map
                        .contains(&student_id)
                    {
                        return Err(AddNewGroupListError::InvalidStudentId(student_id).into());
                    }
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Add(group_list.clone()),
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
            Self::UpdateGroupList(group_list_id, group_list) => {
                if !data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .contains(group_list_id)
                {
                    return Err(UpdateGroupListError::InvalidGroupListId(*group_list_id).into());
                }

                for student_id in students_of(group_list.filling()) {
                    if !data
                        .get_data()
                        .get_inner_data()
                        .params
                        .students
                        .student_map
                        .contains(&student_id)
                    {
                        return Err(UpdateGroupListError::InvalidStudentId(student_id).into());
                    }
                }

                // No reshaping, no rebuild, no arity assert: the payload is a
                // sealed value that already carries both halves.
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Update(
                                *group_list_id,
                                group_list.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("All data should be valid at this point");
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
                    .contains(group_list_id)
                {
                    return Err(DeleteGroupListError::InvalidGroupListId(*group_list_id).into());
                };

                let result = match data.apply(
                    collomatique_state_colloscopes::Op::GroupList(
                        collomatique_state_colloscopes::GroupListOp::Remove(*group_list_id),
                    ),
                    self.get_desc(),
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        use collomatique_state_colloscopes::{
                            Error, FixableInvariant, GroupListRefSite, Reference,
                        };
                        // The subject associations referencing this group list are
                        // stripped by get_next_cleaning_op before Remove reaches
                        // apply, so a leftover association dangle here is a
                        // cleaning-contract breach, not user input.
                        match e {
                            Error::BrokenInvariants(set)
                                if set.iter().any(|inv| {
                                    matches!(
                                        inv,
                                        FixableInvariant::DanglingFk(Reference::GroupList {
                                            site: GroupListRefSite::AssociationEntry { .. },
                                            ..
                                        })
                                    )
                                }) =>
                            {
                                panic!("Associated subjects should be properly cleaned: {set:?}")
                            }
                            _ => panic!("Unexpected error when calling GroupListOp::Remove: {e:?}"),
                        }
                    }
                };
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

                if data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    return Err(AssignGroupListToSubjectError::InvalidPeriodId(*period_id).into());
                }

                if let Some(group_list_id) = group_list_id_opt
                    && !data
                        .get_data()
                        .get_inner_data()
                        .params
                        .group_lists
                        .group_list_map
                        .contains(group_list_id)
                {
                    return Err(
                        AssignGroupListToSubjectError::InvalidGroupListId(*group_list_id).into(),
                    );
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
                        *period_id,
                    )
                    .into());
                };

                if position == 0 {
                    return Err(
                        DuplicatePreviousPeriodAssociationsError::FirstPeriodHasNoPreviousPeriod(
                            *period_id,
                        )
                        .into(),
                    );
                }

                let previous_period_id = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .period_id_at(position - 1)
                    .expect("position > 0 checked above");
                let previous_period_assignments: std::collections::BTreeMap<_, _> = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                    .filter_map(|((period, subject), group_list)| {
                        (period == previous_period_id).then_some((subject, *group_list))
                    })
                    .collect();

                let subjects = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                    .clone();

                for (subject_id, subject) in subjects.iter() {
                    let subject_id = &subject_id;
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
                GroupListsUpdateOp::AddNewGroupList(_group_list) => {
                    "Ajouter une liste de groupes".into()
                }
                GroupListsUpdateOp::UpdateGroupList(_id, _group_list) => {
                    "Modifier une liste de groupes".into()
                }
                GroupListsUpdateOp::DeleteGroupList(_id) => "Supprimer une liste de groupes".into(),
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
