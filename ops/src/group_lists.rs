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

    // Nothing outside the tests calls this yet: the `UpdateOp` dispatch that
    // does is the last commit of the family migration. Drop the attribute then.
    #[allow(dead_code)]
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<Option<collomatique_state_colloscopes::GroupListId>, GroupListsUpdateError> {
        match self {
            Self::AddNewGroupList(group_list) => {
                // The payload is a sealed `GroupList`, so its internal
                // consistency is already settled; only student *existence* — a
                // state-dependent fact — is left to check here.
                for student_id in students_of(group_list.filling()) {
                    if !session
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

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Add(group_list.clone()),
                        ),
                        self.get_desc(),
                    )
                    // A brand new list is named by nobody: no subject is
                    // associated with an id that did not exist a moment ago, and
                    // the colloscope holds no placement row for it either, so
                    // none of the four predicates watching a list has anything
                    // to look at. The only ids the payload carries are its
                    // students', checked just above.
                    .expect("a list nothing names yet contradicts nothing");
                let Some(collomatique_state_colloscopes::NewId::GroupListId(new_id)) = result
                else {
                    panic!("Unexpected result from GroupListOp::Add");
                };
                Ok(Some(new_id))
            }
            Self::UpdateGroupList(group_list_id, group_list) => {
                if !session
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
                    if !session
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
                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Update(
                                *group_list_id,
                                group_list.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    // Everything a new payload can contradict is material that
                    // was already there, so the cascade repairs all of it: the
                    // colloscope placement of a student it now excludes, the
                    // placements and the interrogation groups the new group
                    // count no longer has, and the whole placement row if the
                    // list becomes prefilled. What the payload says about the
                    // list *itself* is the caller's own edit and lands verbatim
                    // — the four cleaning scans the old body ran here never
                    // looked at it either.
                    .expect("the cascade repairs whatever a new payload contradicts");
                assert!(result.is_none());

                Ok(None)
            }
            Self::DeleteGroupList(group_list_id) => {
                if !session
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .group_list_map
                    .contains(group_list_id)
                {
                    return Err(DeleteGroupListError::InvalidGroupListId(*group_list_id).into());
                };

                let result = session
                    .apply(
                        collomatique_state_colloscopes::Op::GroupList(
                            collomatique_state_colloscopes::GroupListOp::Remove(*group_list_id),
                        ),
                        self.get_desc(),
                    )
                    // The old body panicked here when a subject association
                    // still named the list — five cleaning scans were supposed
                    // to have emptied the way, three of them undoing the doomed
                    // list's own filling first, which the removal takes with it
                    // anyway. Both sites a removal leaves dangling are the
                    // cascade's business now: every association goes, the
                    // colloscope placement row goes, and each dropped
                    // association takes the group numbers of the colles at its
                    // coordinate with it.
                    .expect("the cascade repairs everything a removed list leaves behind");
                assert!(result.is_none());

                Ok(None)
            }
            Self::AssignGroupListToSubject(period_id, subject_id, group_list_id_opt) => {
                let Some(subject) = session
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

                if session
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
                    && !session
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

                let result = session
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
                    // The three ids the entry carries are checked just above,
                    // and the two predicates watching an association — a
                    // subject with no interrogations, a subject that does not
                    // run on the period — are the two prechecks between them.
                    // What is left is the colles already written at this
                    // coordinate: a list with fewer groups than they name (or no
                    // list at all, which puts the bound at zero) leaves them out
                    // of range, and the cascade trims them one group at a time.
                    .expect("the cascade trims whatever colles the new bound leaves out of range");
                assert!(result.is_none());

                Ok(None)
            }
            Self::DuplicatePreviousPeriod(period_id) => {
                let Some(position) = session
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

                let previous_period_id = session
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .period_id_at(position - 1)
                    .expect("position > 0 checked above");
                let previous_period_assignments: std::collections::BTreeMap<_, _> = session
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

                // Read once, before the loop: nothing a cascade can answer
                // touches the subject list or the previous period's
                // associations — no fix creates a subject, excludes one from a
                // period or removes a group list — so what the loop plans
                // against the pre-state stays true op after op (the frame
                // rule).
                let subjects = session
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

                    let result = session
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
                        // The same reasoning as the single assignment above,
                        // with the three prechecks replaced by the loop's own
                        // filters: the subject runs on both periods and holds
                        // interrogations, and the group list comes out of a live
                        // association. Only the colles of the target period can
                        // be left out of range, and those the cascade trims.
                        .expect(
                            "the cascade trims whatever colles the new bound leaves out of range",
                        );
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

#[cfg(test)]
mod tests {
    //! A group list is the one entity of the document that sits between two
    //! worlds. Above it, the *parameters*: how many groups there are and who is
    //! in them, which the caller describes whole. Below it, everything the
    //! colloscope wrote against those groups: the students placed in them and
    //! the group numbers the colles name. The two are what every fixture below
    //! separates.
    //!
    //! The caller's half is silent. Since the merge of July 31 2026 the payload
    //! is one sealed [`GroupList`] carrying parameters *and* filling, so a group
    //! they deleted and a student they took out of a group are their own edits:
    //! the op lands them verbatim and says nothing. The old split ops had to
    //! guess, and warned.
    //!
    //! The colloscope's half is the cascade's, and it is where the eleven
    //! cleaning scans of the old module went. Four of them are one convergence
    //! each — a placement out of the new count's range, a colle group out of
    //! range, a placement of a newly-excluded student, a placement row on a list
    //! that just became prefilled — and the remaining two dangle arms answer the
    //! removal. The panic the old removal kept for an association it had failed
    //! to clean (« Associated subjects should be properly cleaned ») has no
    //! reachable input left: the associations are dropped by the cascade now.
    //!
    //! Two shapes of the base document shape the fixtures. Its two group lists
    //! are **prefilled**, and a prefilled list may hold no colloscope placement
    //! row at all, so every fixture that needs a placement builds an automatic
    //! list of its own on top, in plain sight at its head. And it carries no
    //! colloscope, so the colles a fixture is about are written there too.
    //!
    //! One order is worth reading twice, in
    //! [deleting_a_list_takes_the_colles_its_associations_bounded_with_it]: the
    //! colles of a coordinate die *before* the association that bounded them,
    //! because dropping the association is what makes them out of range and the
    //! engine lands a repair's own repairs first. One group at a time, which is
    //! the case §3.13 of the plan looked at and deliberately left alone — here
    //! the user asked for the list to go, so the colles going with it is no
    //! surprise.

    use super::*;
    use crate::test_utils::{fixes, hogwarts};
    use collomatique_state::AppState;
    use collomatique_state::traits::Manager;
    use collomatique_state_colloscopes::{
        AssignmentOp, ColloscopeOp, Fix, GroupListOp, NewId, NonEmptyRangeInclusive, Op, SubjectOp,
        group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup},
        ids::{GroupListId, Id, PeriodId, SlotId, StudentId, SubjectId, WeekId},
        subjects::Subject,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;

    fn subject_by_name(data: &Data, name: &str) -> SubjectId {
        data.get_inner_data()
            .params
            .subjects
            .ordered_subject_list
            .iter()
            .find(|(_id, subject)| subject.parameters.name == name)
            .map(|(id, _subject)| id)
            .unwrap_or_else(|| panic!("the fixture should have a subject named {name}"))
    }

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

    fn list_of(data: &Data, group_list: GroupListId) -> GroupList {
        data.get_inner_data()
            .params
            .group_lists
            .group_list_map
            .get(&group_list)
            .expect("the fixture's group list should be live")
            .clone()
    }

    /// The `(period, subject)` coordinates `group_list` is used at, in key
    /// order — the order the reference site carries.
    fn associations_of(data: &Data, group_list: GroupListId) -> Vec<(PeriodId, SubjectId)> {
        data.get_inner_data()
            .params
            .group_lists
            .subjects_associations
            .iter()
            .filter(|(_coordinate, assigned)| **assigned == group_list)
            .map(|(coordinate, _assigned)| coordinate)
            .collect()
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

    /// The subject's slots, in display order.
    fn slots_of_subject(data: &Data, subject: SubjectId) -> Vec<SlotId> {
        data.get_inner_data()
            .params
            .slots
            .slots_for_subject(subject)
            .into_iter()
            .flatten()
            .map(|(id, _slot)| *id)
            .collect()
    }

    /// The weeks of `period` a colle may be written on for `slot`, in id order.
    fn writable_weeks(data: &Data, slot: SlotId, period: PeriodId) -> Vec<WeekId> {
        let params = &data.get_inner_data().params;
        let mut weeks: Vec<_> = params
            .week_ids()
            .filter(|week| {
                params.weeks.week_position(*week).map(|(p, _pos)| p) == Some(period)
                    && params.is_interrogation_possible(slot, *week)
            })
            .collect();
        weeks.sort();

        weeks
    }

    /// Group-list parameters with `count` unnamed groups.
    fn list_params(name: &str, count: usize) -> GroupListParameters {
        GroupListParameters {
            name: name.into(),
            students_per_group: NonEmptyRangeInclusive::new(
                NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            )
            .expect("statically non-empty"),
            group_names: vec![None; count],
        }
    }

    fn automatic_list(name: &str, count: usize, excluded: BTreeSet<StudentId>) -> GroupList {
        GroupList::new(
            list_params(name, count),
            GroupListFilling::Automatic {
                excluded_students: excluded,
            },
        )
        .expect("an automatic filling never constrains the group count")
    }

    fn prefilled_list(name: &str, groups: Vec<BTreeSet<StudentId>>) -> GroupList {
        GroupList::new(
            list_params(name, groups.len()),
            GroupListFilling::Prefilled {
                groups: groups
                    .into_iter()
                    .map(|students| PrefilledGroup { students })
                    .collect(),
            },
        )
        .expect("the group count is read off the group list itself")
    }

    /// Ids no document ever issued.
    fn dangling_group_list() -> GroupListId {
        unsafe { GroupListId::new(1u64 << 40) }
    }

    fn dangling_student() -> StudentId {
        unsafe { StudentId::new(1u64 << 40) }
    }

    fn dangling_subject() -> SubjectId {
        unsafe { SubjectId::new(1u64 << 40) }
    }

    fn dangling_period() -> PeriodId {
        unsafe { PeriodId::new(1u64 << 40) }
    }

    /// Replays `ops` on a clone of `base`: the document a fixture expects,
    /// written as the elementary ops it expects the composite to have landed —
    /// each of them valid in that order, exactly as the cascade lands them.
    fn expected_document(base: &AppState<Data, Desc>, ops: Vec<Op>) -> AppState<Data, Desc> {
        let mut expected = base.clone();
        for op in ops {
            expected
                .apply(op, (OpCategory::GroupLists, "Expected".into()))
                .expect("each expected op lands in the order the cascade landed it");
        }

        expected
    }

    /// Applies one preparation op to the base a fixture builds on.
    fn prepare(base: &mut AppState<Data, Desc>, op: Op) {
        base.apply(op.clone(), (OpCategory::GroupLists, "Préparation".into()))
            .unwrap_or_else(|e| panic!("the preparation op {op:?} should land, got {e:?}"));
    }

    /// Runs one op alone on `base` and hands back what the document became and
    /// what the cascade had to repair on the way.
    fn apply_alone(
        base: &AppState<Data, Desc>,
        op: &GroupListsUpdateOp,
    ) -> (AppState<Data, Desc>, Vec<CascadeWarning>) {
        let mut session = CascadeSession::new(base.clone());
        op.apply_to_session(&mut session)
            .unwrap_or_else(|e| panic!("{op:?} should land, got {e:?}"));

        session.commit(op.get_desc())
    }

    /// The number of groups the automatic list of [placed_list] offers.
    const AUTOMATIC_GROUPS: usize = 3;

    /// The corner the base document does not carry, since both of its lists are
    /// prefilled: an **automatic** list — the only shape that may hold a
    /// colloscope placement row — used by Divination on the first period in
    /// place of the base's own list, with two students placed in it and one
    /// colle written on two of its groups.
    struct PlacedList {
        base: AppState<Data, Desc>,
        list: GroupListId,
        subject: SubjectId,
        period: PeriodId,
        slot: SlotId,
        week: WeekId,
        harry: StudentId,
        ron: StudentId,
    }

    fn placed_list() -> PlacedList {
        let mut base = hogwarts();
        let subject = subject_by_name(base.get_data(), "Divination");
        let period = period_at(base.get_data(), 0);
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let ron = student_by_name(base.get_data(), "Weasley", "Ron");

        let list = match base.apply(
            Op::GroupList(GroupListOp::Add(automatic_list(
                "Liste automatique",
                AUTOMATIC_GROUPS,
                BTreeSet::new(),
            ))),
            (OpCategory::GroupLists, "Préparation".into()),
        ) {
            Ok(Some(NewId::GroupListId(id))) => id,
            other => panic!("adding a group list should hand back its id, got {other:?}"),
        };
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(period, subject, Some(list))),
        );

        let slot = slots_of_subject(base.get_data(), subject)[0];
        let week = writable_weeks(base.get_data(), slot, period)[0];
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetGroupList(
                list,
                BTreeMap::from([(harry, 0), (ron, 1)]),
            )),
        );
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                slot,
                week,
                BTreeSet::from([0, 2]),
            )),
        );

        PlacedList {
            base,
            list,
            subject,
            period,
            slot,
            week,
            harry,
            ron,
        }
    }

    /// A list nothing names yet cannot cost anything: the id comes back, the
    /// log stays empty, and the filling the caller described lands untouched —
    /// which is the whole point of the widened payload, the old op having
    /// forced every new list to be automatic.
    #[test]
    fn adding_a_list_lands_its_filling_verbatim_and_warns_about_nothing() {
        let base = hogwarts();
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let ron = student_by_name(base.get_data(), "Weasley", "Ron");
        let payload = prefilled_list(
            "Liste de rattrapage",
            vec![BTreeSet::from([harry]), BTreeSet::from([ron])],
        );

        let mut session = CascadeSession::new(base.clone());
        let op = GroupListsUpdateOp::AddNewGroupList(payload.clone());
        let new_id = op
            .apply_to_session(&mut session)
            .expect("a fresh list names nothing but its students");
        let (state, warnings) = session.commit(op.get_desc());

        let new_id = new_id.expect("adding a group list returns the id it issued");
        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(list_of(state.get_data(), new_id), payload);
    }

    /// The two writing ops share one student-existence sweep, and it has to see
    /// **both** halves of a filling: `GroupListFilling::iter_students` walks the
    /// prefilled groups only, so an automatic list's excluded set needs its own
    /// pass. Also the order between the two checks of the update: the list's own
    /// id is answered before anything the payload says.
    #[test]
    fn both_writing_ops_report_a_dead_id_whichever_part_of_the_payload_names_it() {
        let base = hogwarts();
        let harry = student_by_name(base.get_data(), "Potter", "Harry");
        let live = group_list_by_name(base.get_data(), "Divination");
        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            GroupListsUpdateOp::AddNewGroupList(prefilled_list(
                "Liste",
                vec![BTreeSet::from([dangling_student()])],
            ))
            .apply_to_session(&mut session)
            .unwrap_err(),
            GroupListsUpdateError::AddNewGroupList(AddNewGroupListError::InvalidStudentId(
                dangling_student()
            )),
        );
        assert_eq!(
            GroupListsUpdateOp::AddNewGroupList(automatic_list(
                "Liste",
                2,
                BTreeSet::from([dangling_student()]),
            ))
            .apply_to_session(&mut session)
            .unwrap_err(),
            GroupListsUpdateError::AddNewGroupList(AddNewGroupListError::InvalidStudentId(
                dangling_student()
            )),
        );
        assert_eq!(
            GroupListsUpdateOp::UpdateGroupList(
                live,
                prefilled_list("Liste", vec![BTreeSet::from([dangling_student()])]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            GroupListsUpdateError::UpdateGroupList(UpdateGroupListError::InvalidStudentId(
                dangling_student()
            )),
        );
        assert_eq!(
            GroupListsUpdateOp::UpdateGroupList(
                live,
                automatic_list("Liste", 2, BTreeSet::from([dangling_student()])),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            GroupListsUpdateError::UpdateGroupList(UpdateGroupListError::InvalidStudentId(
                dangling_student()
            )),
        );
        // A payload naming a dead student, aimed at a dead list: the list wins.
        assert_eq!(
            GroupListsUpdateOp::UpdateGroupList(
                dangling_group_list(),
                prefilled_list("Liste", vec![BTreeSet::from([dangling_student()])]),
            )
            .apply_to_session(&mut session)
            .unwrap_err(),
            GroupListsUpdateError::UpdateGroupList(UpdateGroupListError::InvalidGroupListId(
                dangling_group_list()
            )),
        );
        // And a removal, whose only way of being wrong is that one.
        assert_eq!(
            GroupListsUpdateOp::DeleteGroupList(dangling_group_list())
                .apply_to_session(&mut session)
                .unwrap_err(),
            GroupListsUpdateError::DeleteGroupList(DeleteGroupListError::InvalidGroupListId(
                dangling_group_list()
            )),
        );
        // A live list with a live filling still passes both sweeps.
        GroupListsUpdateOp::UpdateGroupList(
            live,
            automatic_list("Liste", 2, BTreeSet::from([harry])),
        )
        .apply_to_session(&mut session)
        .expect("a live list and a live student are all the sweeps ask for");
    }

    /// The merge's own rule, on the new path: the payload is the caller's whole
    /// description of the list, so a group they deleted and the students they
    /// took out of it are their own edit. The list lands exactly as given and
    /// the log stays empty — no scan compares the old filling with the new one.
    #[test]
    fn replacing_a_list_lands_verbatim_and_says_nothing_about_what_the_caller_dropped() {
        let base = hogwarts();
        let list = group_list_by_name(base.get_data(), "Divination");
        let (params, filling) = list_of(base.get_data(), list).into_parts();
        let GroupListFilling::Prefilled { groups } = filling else {
            panic!("the fixture's Divination list should be prefilled");
        };
        assert_eq!(
            groups.len(),
            5,
            "the fixture's list should hold five groups"
        );

        // One group less, and the three students it held simply absent from the
        // payload.
        let payload = GroupList::new(
            GroupListParameters {
                group_names: params.group_names[..4].to_vec(),
                ..params
            },
            GroupListFilling::Prefilled {
                groups: groups[..4].to_vec(),
            },
        )
        .expect("four groups and four group names");

        let op = GroupListsUpdateOp::UpdateGroupList(list, payload.clone());
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![Op::GroupList(GroupListOp::Update(list, payload))],
            )
            .get_data(),
        );
    }

    /// What the caller *cannot* see, and so must be told about: down to a
    /// single group, both a colle naming group 2 and Ron's placement in group 1
    /// are out of range. The colle goes first — the interrogation-row predicate
    /// is declared ahead of the placement one — and what still fits (group 0,
    /// and Harry in it) is left alone.
    #[test]
    fn shrinking_a_list_trims_the_colles_and_the_placements_the_dropped_groups_held() {
        let placed = placed_list();
        let payload = automatic_list("Liste automatique", 1, BTreeSet::new());

        let op = GroupListsUpdateOp::UpdateGroupList(placed.list, payload.clone());
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::RemoveGroupsFromInterrogationCell {
                    slot: placed.slot,
                    week: placed.week,
                    groups: BTreeSet::from([2]),
                    rebuilt: BTreeSet::from([0]),
                },
                Fix::RemoveStudentColloscopePlacement {
                    group_list: placed.list,
                    student: placed.ron,
                    rebuilt: BTreeMap::from([(placed.harry, 0)]),
                },
            ],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &placed.base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        placed.slot,
                        placed.week,
                        BTreeSet::from([0]),
                    )),
                    Op::Colloscope(ColloscopeOp::SetGroupList(
                        placed.list,
                        BTreeMap::from([(placed.harry, 0)]),
                    )),
                    Op::GroupList(GroupListOp::Update(placed.list, payload)),
                ],
            )
            .get_data(),
        );
    }

    /// Excluding a student the colloscope already placed: the placement goes,
    /// the other one stays, and the colles — which name groups, not students —
    /// are untouched.
    #[test]
    fn excluding_a_placed_student_takes_their_placement_out_of_the_colloscope() {
        let placed = placed_list();
        let payload = automatic_list(
            "Liste automatique",
            AUTOMATIC_GROUPS,
            BTreeSet::from([placed.ron]),
        );

        let op = GroupListsUpdateOp::UpdateGroupList(placed.list, payload.clone());
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::RemoveStudentColloscopePlacement {
                group_list: placed.list,
                student: placed.ron,
                rebuilt: BTreeMap::from([(placed.harry, 0)]),
            }],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &placed.base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetGroupList(
                        placed.list,
                        BTreeMap::from([(placed.harry, 0)]),
                    )),
                    Op::GroupList(GroupListOp::Update(placed.list, payload)),
                ],
            )
            .get_data(),
        );
    }

    /// A prefilled list holds no colloscope placement row at all, so turning a
    /// placed list prefilled retires the whole row — **one** repair, where the
    /// old cleaning walked it one student at a time and warned once per
    /// student. The row is the offending thing here, and there is no single
    /// placement to blame for it.
    #[test]
    fn turning_a_placed_list_prefilled_clears_its_whole_placement_row_at_once() {
        let placed = placed_list();
        let payload = prefilled_list(
            "Liste automatique",
            vec![
                BTreeSet::from([placed.harry]),
                BTreeSet::from([placed.ron]),
                BTreeSet::new(),
            ],
        );

        let op = GroupListsUpdateOp::UpdateGroupList(placed.list, payload.clone());
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::ClearColloscopeGroupListRow {
                group_list: placed.list,
            }],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &placed.base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetGroupList(placed.list, BTreeMap::new())),
                    Op::GroupList(GroupListOp::Update(placed.list, payload)),
                ],
            )
            .get_data(),
        );
    }

    /// The removal's first dangle site, on the base's own document: a list used
    /// by one subject on all three periods. Every association goes, in the order
    /// the reference site carries — and the list's own filling goes with the row
    /// it lives in, which is why the old body's three pre-cleaning scans of it
    /// have no successor here.
    #[test]
    fn deleting_a_list_unassigns_every_subject_that_used_it() {
        let base = hogwarts();
        let list = group_list_by_name(base.get_data(), "Divination");
        let divination = subject_by_name(base.get_data(), "Divination");
        let coordinates = associations_of(base.get_data(), list);
        assert_eq!(
            coordinates,
            (0..3)
                .map(|index| (period_at(base.get_data(), index), divination))
                .collect::<Vec<_>>(),
            "the fixture's Divination list should serve its subject on all three periods",
        );

        let op = GroupListsUpdateOp::DeleteGroupList(list);
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            coordinates
                .iter()
                .map(|(period, subject)| Fix::UnassignGroupList {
                    period: *period,
                    subject: *subject,
                })
                .collect::<Vec<_>>(),
        );
        let mut expected_ops: Vec<_> = coordinates
            .iter()
            .map(|(period, subject)| {
                Op::GroupList(GroupListOp::AssignToSubject(*period, *subject, None))
            })
            .collect();
        expected_ops.push(Op::GroupList(GroupListOp::Remove(list)));
        assert_eq!(
            state.get_data(),
            expected_document(&base, expected_ops).get_data(),
        );
    }

    /// The removal's second dangle site, and the order that is worth reading
    /// twice. Dropping the association is what takes the group bound of that
    /// coordinate to zero, so the colles written there become out of range —
    /// and the engine lands a repair's own repairs before the repair itself.
    /// The colles therefore die *first*, each cell emptied of all its groups in
    /// one go, then the association, then the placement row, and only then the
    /// list.
    #[test]
    fn deleting_a_list_takes_the_colles_its_associations_bounded_with_it() {
        let placed = placed_list();

        let op = GroupListsUpdateOp::DeleteGroupList(placed.list);
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![
                Fix::RemoveGroupsFromInterrogationCell {
                    slot: placed.slot,
                    week: placed.week,
                    groups: BTreeSet::from([0, 2]),
                    rebuilt: BTreeSet::new(),
                },
                Fix::UnassignGroupList {
                    period: placed.period,
                    subject: placed.subject,
                },
                Fix::ClearColloscopeGroupListRow {
                    group_list: placed.list,
                },
            ],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &placed.base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        placed.slot,
                        placed.week,
                        BTreeSet::new(),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        placed.period,
                        placed.subject,
                        None,
                    )),
                    Op::Colloscope(ColloscopeOp::SetGroupList(placed.list, BTreeMap::new())),
                    Op::GroupList(GroupListOp::Remove(placed.list)),
                ],
            )
            .get_data(),
        );
    }

    /// Swapping in a shorter list: the colles that named a group the new list
    /// does not have are trimmed, the ones that fit are left alone.
    #[test]
    fn assigning_a_shorter_list_trims_the_colles_that_overflow_it() {
        let placed = placed_list();
        let mut base = placed.base;
        let short = match base.apply(
            Op::GroupList(GroupListOp::Add(automatic_list(
                "Petite liste",
                2,
                BTreeSet::new(),
            ))),
            (OpCategory::GroupLists, "Préparation".into()),
        ) {
            Ok(Some(NewId::GroupListId(id))) => id,
            other => panic!("adding a group list should hand back its id, got {other:?}"),
        };

        let op = GroupListsUpdateOp::AssignGroupListToSubject(
            placed.period,
            placed.subject,
            Some(short),
        );
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::RemoveGroupsFromInterrogationCell {
                slot: placed.slot,
                week: placed.week,
                groups: BTreeSet::from([2]),
                rebuilt: BTreeSet::from([0]),
            }],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        placed.slot,
                        placed.week,
                        BTreeSet::from([0]),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        placed.period,
                        placed.subject,
                        Some(short),
                    )),
                ],
            )
            .get_data(),
        );
    }

    /// Taking the list away outright takes the bound to zero, so every group of
    /// every colle at that coordinate is out of range and the cell empties in a
    /// single fix naming all of them: here it reads the user's own edit back to
    /// them.
    #[test]
    fn unassigning_a_list_empties_the_colles_it_bounded_in_one_go() {
        let placed = placed_list();

        let op = GroupListsUpdateOp::AssignGroupListToSubject(placed.period, placed.subject, None);
        let (state, warnings) = apply_alone(&placed.base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::RemoveGroupsFromInterrogationCell {
                slot: placed.slot,
                week: placed.week,
                groups: BTreeSet::from([0, 2]),
                rebuilt: BTreeSet::new(),
            }],
        );
        assert_eq!(
            state.get_data(),
            expected_document(
                &placed.base,
                vec![
                    Op::Colloscope(ColloscopeOp::SetInterrogation(
                        placed.slot,
                        placed.week,
                        BTreeSet::new(),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        placed.period,
                        placed.subject,
                        None,
                    )),
                ],
            )
            .get_data(),
        );
    }

    /// The assignment's five ops-level prechecks, in the order they run — which
    /// is the surface: a call wrong in two ways at once gets the first answer.
    /// The subject comes before the period, and the two predicates the state
    /// layer would break on (a subject with no interrogations, a subject that
    /// does not run on the period) are pre-empted here rather than translated,
    /// because the cascade's answer to either would be to undo the caller's own
    /// association.
    #[test]
    fn the_assignment_op_reports_every_way_its_coordinates_can_be_wrong() {
        let mut base = hogwarts();
        let divination = subject_by_name(base.get_data(), "Divination");
        let quidditch = subject_by_name(base.get_data(), "Entrainement de Quidditch");
        let list = group_list_by_name(base.get_data(), "Liste principale");
        let first = period_at(base.get_data(), 0);
        let last = period_at(base.get_data(), 2);

        // A subject that skips a period: the enrolments and the association it
        // holds there have to go before the exclusion is legal.
        prepare(
            &mut base,
            Op::Assignment(AssignmentOp::SetRow(last, divination, BTreeSet::new())),
        );
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(last, divination, None)),
        );
        let excluding = Subject {
            excluded_periods: BTreeSet::from([last]),
            ..base
                .get_data()
                .get_inner_data()
                .params
                .subjects
                .find_subject(divination)
                .expect("the fixture's Divination subject should be live")
                .clone()
        };
        prepare(
            &mut base,
            Op::Subject(SubjectOp::Update(divination, excluding)),
        );

        let mut session = CascadeSession::new(base.clone());
        for (op, expected) in [
            (
                GroupListsUpdateOp::AssignGroupListToSubject(first, dangling_subject(), Some(list)),
                AssignGroupListToSubjectError::InvalidSubjectId(dangling_subject()),
            ),
            // Wrong in two ways: the subject is checked first, so it answers.
            (
                GroupListsUpdateOp::AssignGroupListToSubject(
                    dangling_period(),
                    dangling_subject(),
                    Some(list),
                ),
                AssignGroupListToSubjectError::InvalidSubjectId(dangling_subject()),
            ),
            (
                GroupListsUpdateOp::AssignGroupListToSubject(first, quidditch, Some(list)),
                AssignGroupListToSubjectError::SubjectHasNoInterrogation(quidditch),
            ),
            // Likewise between the interrogation check and the period one.
            (
                GroupListsUpdateOp::AssignGroupListToSubject(
                    dangling_period(),
                    quidditch,
                    Some(list),
                ),
                AssignGroupListToSubjectError::SubjectHasNoInterrogation(quidditch),
            ),
            (
                GroupListsUpdateOp::AssignGroupListToSubject(last, divination, Some(list)),
                AssignGroupListToSubjectError::SubjectDoesNotRunOnPeriod(divination, last),
            ),
            (
                GroupListsUpdateOp::AssignGroupListToSubject(
                    dangling_period(),
                    divination,
                    Some(list),
                ),
                AssignGroupListToSubjectError::InvalidPeriodId(dangling_period()),
            ),
            (
                GroupListsUpdateOp::AssignGroupListToSubject(
                    first,
                    divination,
                    Some(dangling_group_list()),
                ),
                AssignGroupListToSubjectError::InvalidGroupListId(dangling_group_list()),
            ),
        ] {
            assert_eq!(
                op.apply_to_session(&mut session).unwrap_err(),
                GroupListsUpdateError::AssignGroupListToSubject(expected),
                "{op:?}",
            );
        }
    }

    /// The composite: one assignment per subject that runs on both periods and
    /// holds interrogations, copying what the previous period says — including
    /// when what it says is « no list at all ». The fixture perturbs three
    /// coordinates and lets the duplication put two of them back.
    ///
    /// One of the loop's three filters is structurally shadowed, and no fixture
    /// can catch it: dropping the `interrogation_parameters.is_none()` skip
    /// changes nothing. A subject without interrogations may hold no
    /// association on *any* period — that is
    /// `Conv:AssociationForSubjectWithoutInterrogations` — so the previous
    /// period has nothing to copy for it and the assignment it would then write
    /// is `None` onto a coordinate that is already empty: a perfect no-op. The
    /// filter is kept because the old body had it, not because it guards
    /// anything.
    #[test]
    fn duplicating_copies_the_previous_periods_associations_including_the_absent_ones() {
        let mut base = hogwarts();
        let first = period_at(base.get_data(), 0);
        let second = period_at(base.get_data(), 1);
        let potions = subject_by_name(base.get_data(), "Potions");
        let arithmancie = subject_by_name(base.get_data(), "Arithmancie");
        let divination = subject_by_name(base.get_data(), "Divination");
        let main = group_list_by_name(base.get_data(), "Liste principale");
        let divination_list = group_list_by_name(base.get_data(), "Divination");

        // Arithmancie uses no list on the first period, so the duplication has
        // to *remove* the second period's; the two others are copied back onto
        // coordinates the preparation moved away.
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(first, arithmancie, None)),
        );
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(second, potions, None)),
        );
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(second, divination, Some(main))),
        );

        let op = GroupListsUpdateOp::DuplicatePreviousPeriod(second);
        let (state, warnings) = apply_alone(&base, &op);

        assert!(warnings.is_empty(), "nothing to repair: {warnings:?}");
        // The six subjects with interrogations, in list order: the composite
        // writes every one of them, even where the value does not move.
        assert_eq!(
            state.get_data(),
            expected_document(
                &base,
                vec![
                    Op::GroupList(GroupListOp::AssignToSubject(second, potions, Some(main))),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        second,
                        subject_by_name(base.get_data(), "Défense contre les forces du Mal"),
                        Some(main),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        second,
                        subject_by_name(base.get_data(), "Métamorphose"),
                        Some(main),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(second, arithmancie, None)),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        second,
                        divination,
                        Some(divination_list),
                    )),
                    Op::GroupList(GroupListOp::AssignToSubject(
                        second,
                        subject_by_name(base.get_data(), "Potions - TP"),
                        Some(main),
                    )),
                ],
            )
            .get_data(),
        );
    }

    /// The composite's assignments cascade like any other: a copied list with
    /// fewer groups than the colles of the target period name trims them.
    #[test]
    fn duplicating_trims_the_colles_the_copied_lists_no_longer_bound() {
        let mut base = hogwarts();
        let second = period_at(base.get_data(), 1);
        let divination = subject_by_name(base.get_data(), "Divination");
        let main = group_list_by_name(base.get_data(), "Liste principale");
        let divination_list = group_list_by_name(base.get_data(), "Divination");

        // Divination runs on the eight-group list for the second period, and a
        // colle there names its group 6. The first period's five-group list is
        // what the duplication copies back.
        prepare(
            &mut base,
            Op::GroupList(GroupListOp::AssignToSubject(second, divination, Some(main))),
        );
        let slot = slots_of_subject(base.get_data(), divination)[0];
        let week = writable_weeks(base.get_data(), slot, second)[0];
        prepare(
            &mut base,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                slot,
                week,
                BTreeSet::from([1, 6]),
            )),
        );

        let op = GroupListsUpdateOp::DuplicatePreviousPeriod(second);
        let (state, warnings) = apply_alone(&base, &op);

        assert_eq!(
            fixes(&warnings),
            vec![Fix::RemoveGroupsFromInterrogationCell {
                slot,
                week,
                groups: BTreeSet::from([6]),
                rebuilt: BTreeSet::from([1]),
            }],
        );
        assert_eq!(
            state
                .get_data()
                .get_inner_data()
                .colloscope
                .interrogation(slot, week),
            Some(&BTreeSet::from([1])),
        );
        assert_eq!(
            state
                .get_data()
                .get_inner_data()
                .params
                .group_lists
                .subjects_associations
                .get(&(second, divination)),
            Some(&divination_list),
        );
    }

    /// The composite's two ops-level prechecks: a period that does not exist,
    /// and the first period, which has no previous one to copy.
    #[test]
    fn duplicating_the_first_period_or_a_dead_one_is_refused() {
        let base = hogwarts();
        let first = period_at(base.get_data(), 0);
        let mut session = CascadeSession::new(base.clone());

        assert_eq!(
            GroupListsUpdateOp::DuplicatePreviousPeriod(dangling_period())
                .apply_to_session(&mut session)
                .unwrap_err(),
            GroupListsUpdateError::DuplicatePreviousPeriod(
                DuplicatePreviousPeriodAssociationsError::InvalidPeriodId(dangling_period())
            ),
        );
        assert_eq!(
            GroupListsUpdateOp::DuplicatePreviousPeriod(first)
                .apply_to_session(&mut session)
                .unwrap_err(),
            GroupListsUpdateError::DuplicatePreviousPeriod(
                DuplicatePreviousPeriodAssociationsError::FirstPeriodHasNoPreviousPeriod(first)
            ),
        );
    }
}
