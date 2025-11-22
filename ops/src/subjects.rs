use std::collections::BTreeSet;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubjectsUpdateWarning {
    LooseInterrogationDataForTeacher(
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::SubjectId,
    ),
    LooseStudentsAssignmentsForPeriod(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
    ),
    LooseInterrogationSlots(collomatique_state_colloscopes::SubjectId),
    LooseScheduleIncompat(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::IncompatId,
    ),
    LooseGroupListAssociation(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseColloscopeSlotsForPeriod(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

impl SubjectsUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            Self::LooseInterrogationDataForTeacher(teacher_id, subject_id) => {
                let Some(teacher) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .teachers
                    .teacher_map
                    .get(teacher_id)
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
                Some(format!(
                    "Désincription du colleur {} {} pour la matière \"{}\"",
                    teacher.desc.firstname, teacher.desc.surname, subject.parameters.name,
                ))
            }
            Self::LooseStudentsAssignmentsForPeriod(period_id, subject_id) => {
                let Some(period_index) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
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
                Some(format!(
                    "Perte des inscriptions des élèves pour la matière \"{}\" sur la période {}",
                    subject.parameters.name,
                    period_index + 1
                ))
            }
            Self::LooseInterrogationSlots(subject_id) => {
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                else {
                    return None;
                };
                Some(format!(
                    "Perte des créneaux de colles pour la matière \"{}\"",
                    subject.parameters.name,
                ))
            }
            Self::LooseScheduleIncompat(subject_id, incompat_id) => {
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                else {
                    return None;
                };
                let Some(incompat) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .incompats
                    .incompat_map
                    .get(incompat_id)
                else {
                    return None;
                };

                let slot_desc: Vec<_> = incompat
                    .slots
                    .iter()
                    .map(|slot| {
                        format!(
                            "le {} à {}",
                            slot.start().weekday,
                            slot.start().start_time.into_inner()
                        )
                    })
                    .collect();

                Some(format!(
                    "Perte d'une incompatibilité horaire pour la matière \"{}\" ({})",
                    subject.parameters.name,
                    slot_desc.join(", "),
                ))
            }
            Self::LooseGroupListAssociation(subject_id, group_list_id, period_id) => {
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
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
                    "Perte de l'association de la matière \"{}\" à la liste de groupes \"{}\" pour la période {}",
                    subject.parameters.name, group_list.params.name, period_num+1
                ))
            }
            Self::LooseColloscopeSlotsForPeriod(subject_id, period_id) => {
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
                    "Perte des colles de \"{}\" sur le colloscope pour la période {}",
                    subject.parameters.name,
                    period_num + 1
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubjectsUpdateOp {
    AddNewSubject(collomatique_state_colloscopes::subjects::SubjectParameters),
    UpdateSubject(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::subjects::SubjectParameters,
    ),
    DeleteSubject(collomatique_state_colloscopes::SubjectId),
    MoveSubjectUp(collomatique_state_colloscopes::SubjectId),
    MoveSubjectDown(collomatique_state_colloscopes::SubjectId),
    UpdatePeriodStatus(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
        bool,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum SubjectsUpdateError {
    #[error(transparent)]
    AddNewSubject(#[from] AddNewSubjectError),
    #[error(transparent)]
    UpdateSubject(#[from] UpdateSubjectError),
    #[error(transparent)]
    DeleteSubject(#[from] DeleteSubjectError),
    #[error(transparent)]
    MoveSubjectUp(#[from] MoveSubjectUpError),
    #[error(transparent)]
    MoveSubjectDown(#[from] MoveSubjectDownError),
    #[error(transparent)]
    UpdatePeriodStatus(#[from] UpdatePeriodStatusError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AddNewSubjectError {
    #[error("Students per group range should allow at least one value")]
    StudentsPerGroupRangeIsEmpty,
    #[error("Groups per interrogations range should allow at least one value")]
    GroupsPerInterrogationRangeIsEmpty,
    #[error("Interrogation count range should allow at least one value")]
    InterrogationCountRangeIsEmpty,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeleteSubjectError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveSubjectUpError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Subject is already the first subject")]
    NoUpperPosition,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum MoveSubjectDownError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Subject is already the last subject")]
    NoLowerPosition,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdatePeriodStatusError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

impl SubjectsUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<SubjectsUpdateWarning>> {
        match self {
            Self::AddNewSubject(_) => None,
            Self::MoveSubjectUp(_) => None,
            Self::MoveSubjectDown(_) => None,
            Self::UpdateSubject(subject_id, params) => {
                let Some(current_subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                else {
                    return None;
                };
                let previously_had_interrogations = current_subject
                    .parameters
                    .interrogation_parameters
                    .is_some();

                let no_more_interrogations = params.interrogation_parameters.is_none();

                if previously_had_interrogations && no_more_interrogations {
                    for (teacher_id, teacher) in
                        &data.get_data().get_inner_data().params.teachers.teacher_map
                    {
                        if teacher.subjects.contains(subject_id) {
                            let mut new_teacher = teacher.clone();
                            new_teacher.subjects.remove(subject_id);
                            return Some(CleaningOp {
                                warning: SubjectsUpdateWarning::LooseInterrogationDataForTeacher(
                                    *teacher_id,
                                    *subject_id,
                                ),
                                op: UpdateOp::Teachers(TeachersUpdateOp::UpdateTeacher(
                                    *teacher_id,
                                    new_teacher,
                                )),
                            });
                        }
                    }

                    for (period_id, subject_map) in &data
                        .get_data()
                        .get_inner_data()
                        .params
                        .group_lists
                        .subjects_associations
                    {
                        if let Some(group_list_id) = subject_map.get(subject_id) {
                            return Some(CleaningOp {
                                warning: SubjectsUpdateWarning::LooseGroupListAssociation(
                                    *subject_id,
                                    *group_list_id,
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

                    let subject_slots = data
                        .get_data()
                        .get_inner_data()
                        .params
                        .slots
                        .subject_map
                        .get(subject_id)
                        .expect("Subject should have associated slots at this point");
                    for (slot_id, _slot) in &subject_slots.ordered_slots {
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LooseInterrogationSlots(*subject_id),
                            op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(*slot_id)),
                        });
                    }
                }

                None
            }
            Self::UpdatePeriodStatus(subject_id, period_id, new_status) => {
                let Some(current_subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                else {
                    return None;
                };

                let old_status = !current_subject.excluded_periods.contains(period_id);

                if !*new_status && old_status {
                    if let Some(period_assignments) = data
                        .get_data()
                        .get_inner_data()
                        .params
                        .assignments
                        .period_map
                        .get(period_id)
                    {
                        let assigned_students = period_assignments
                            .subject_map
                            .get(subject_id)
                            .expect("subject_id should be available in subject map at this point");

                        for student_id in assigned_students {
                            return Some(CleaningOp {
                                warning: SubjectsUpdateWarning::LooseStudentsAssignmentsForPeriod(
                                    *period_id,
                                    *subject_id,
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

                    if current_subject
                        .parameters
                        .interrogation_parameters
                        .is_some()
                    {
                        let Some(colloscope_period) = data
                            .get_data()
                            .get_inner_data()
                            .colloscope
                            .period_map
                            .get(period_id)
                        else {
                            return None;
                        };

                        let subject_slots = data
                            .get_data()
                            .get_inner_data()
                            .params
                            .slots
                            .subject_map
                            .get(subject_id)
                            .expect("Subject should have slots at this point");

                        for (slot_id, _slot) in &subject_slots.ordered_slots {
                            let collo_slot = colloscope_period
                                .slot_map
                                .get(slot_id)
                                .expect("Slot should appear in colloscope at this point");

                            if !collo_slot.is_empty() {
                                for week in 0..collo_slot.interrogations.len() {
                                    let Some(interrogation) = &collo_slot.interrogations[week]
                                    else {
                                        continue;
                                    };
                                    if !interrogation.is_empty() {
                                        return Some(CleaningOp {
                                            warning: SubjectsUpdateWarning::LooseColloscopeSlotsForPeriod(
                                                *subject_id,
                                                *period_id,
                                            ),
                                            op: UpdateOp::Colloscope(
                                                ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                                    *period_id,
                                                    *slot_id,
                                                    week,
                                                    collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation::default(),
                                                ),
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }

                    if let Some(subject_map) = data
                        .get_data()
                        .get_inner_data()
                        .params
                        .group_lists
                        .subjects_associations
                        .get(period_id)
                    {
                        if let Some(group_list_id) = subject_map.get(subject_id) {
                            return Some(CleaningOp {
                                warning: SubjectsUpdateWarning::LooseGroupListAssociation(
                                    *subject_id,
                                    *group_list_id,
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
            Self::DeleteSubject(subject_id) => {
                for (teacher_id, teacher) in
                    &data.get_data().get_inner_data().params.teachers.teacher_map
                {
                    if teacher.subjects.contains(subject_id) {
                        let mut new_teacher = teacher.clone();
                        new_teacher.subjects.remove(subject_id);
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LooseInterrogationDataForTeacher(
                                *teacher_id,
                                *subject_id,
                            ),
                            op: UpdateOp::Teachers(TeachersUpdateOp::UpdateTeacher(
                                *teacher_id,
                                new_teacher,
                            )),
                        });
                    }
                }

                for (period_id, subject_map) in &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                {
                    if let Some(group_list_id) = subject_map.get(subject_id) {
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LooseGroupListAssociation(
                                *subject_id,
                                *group_list_id,
                                *period_id,
                            ),
                            op: UpdateOp::GroupLists(GroupListsUpdateOp::AssignGroupListToSubject(
                                *period_id,
                                *subject_id,
                                None,
                            )),
                        });
                    }
                }

                for (incompat_id, incompat) in &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .incompats
                    .incompat_map
                {
                    if incompat.subject_id == *subject_id {
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LooseScheduleIncompat(
                                *subject_id,
                                *incompat_id,
                            ),
                            op: UpdateOp::Incompatibilities(
                                IncompatibilitiesUpdateOp::DeleteIncompat(*incompat_id),
                            ),
                        });
                    }
                }

                if let Some(subject_slots) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .subject_map
                    .get(subject_id)
                {
                    for (slot_id, _slot) in &subject_slots.ordered_slots {
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LooseInterrogationSlots(*subject_id),
                            op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(*slot_id)),
                        });
                    }
                }

                let Some(subject) = &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                else {
                    return None;
                };

                let excluded_periods = &subject.excluded_periods;

                for (period_id, period_assignments) in &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .period_map
                {
                    if excluded_periods.contains(period_id) {
                        continue;
                    }
                    let assigned_students = period_assignments.subject_map.get(subject_id)
                        .expect("Assignment data is inconsistent and does not have a required subject entry");

                    for student_id in assigned_students {
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LooseStudentsAssignmentsForPeriod(
                                *period_id,
                                *subject_id,
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

                None
            }
        }
    }

    pub(crate) fn apply_no_cleaning<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
    ) -> Result<Option<collomatique_state_colloscopes::SubjectId>, SubjectsUpdateError> {
        match self {
            Self::AddNewSubject(params) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::AddAfter(
                                data.get_data().get_inner_data().params.subjects.ordered_subject_list.last().map(|x| x.0),
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
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .ok_or(UpdateSubjectError::InvalidSubjectId(*subject_id))?;

                let excluded_periods = current_subject.excluded_periods.clone();

                let result = data
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
                        self.get_desc(),
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

                Ok(None)
            }
            Self::DeleteSubject(subject_id) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Remove(*subject_id),
                        ),
                        self.get_desc(),
                    )
                    .map_err(|e| {
                        if let collomatique_state_colloscopes::Error::Subject(se) = e {
                            match se {
                                collomatique_state_colloscopes::SubjectError::InvalidSubjectId(
                                    id,
                                ) => DeleteSubjectError::InvalidSubjectId(id),
                                _ => panic!(
                                    "Unexpected subject error during DeleteSubject: {:?}",
                                    se
                                ),
                            }
                        } else {
                            panic!("Unexpected error during DeleteSubject: {:?}", e);
                        }
                    })?;

                assert!(result.is_none());

                Ok(None)
            }
            Self::MoveSubjectUp(subject_id) => {
                let current_position = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject_position(*subject_id)
                    .ok_or(MoveSubjectUpError::InvalidSubjectId(*subject_id))?;

                if current_position == 0 {
                    Err(MoveSubjectUpError::NoUpperPosition)?;
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
            Self::MoveSubjectDown(subject_id) => {
                let current_position = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject_position(*subject_id)
                    .ok_or(MoveSubjectDownError::InvalidSubjectId(*subject_id))?;

                if current_position
                    == data
                        .get_data()
                        .get_inner_data()
                        .params
                        .subjects
                        .ordered_subject_list
                        .len()
                        - 1
                {
                    Err(MoveSubjectDownError::NoLowerPosition)?;
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
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    Err(UpdatePeriodStatusError::InvalidPeriodId(*period_id))?;
                }

                let mut subject = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .ok_or(UpdatePeriodStatusError::InvalidSubjectId(*subject_id))?
                    .clone();

                if *new_status {
                    subject.excluded_periods.remove(period_id);
                } else {
                    subject.excluded_periods.insert(*period_id);
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Subject(
                            collomatique_state_colloscopes::SubjectOp::Update(*subject_id, subject),
                        ),
                        self.get_desc(),
                    )
                    .expect("No error should be possible at this point");
                assert!(result.is_none());

                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Subjects,
            match self {
                SubjectsUpdateOp::AddNewSubject(_desc) => "Ajouter une matière".into(),
                SubjectsUpdateOp::UpdateSubject(_id, _desc) => "Modifier une matière".into(),
                SubjectsUpdateOp::DeleteSubject(_id) => "Supprimer une matière".into(),
                SubjectsUpdateOp::MoveSubjectUp(_id) => "Remonter une matière".into(),
                SubjectsUpdateOp::MoveSubjectDown(_id) => "Descendre une matière".into(),
                Self::UpdatePeriodStatus(_subject_id, _period_id, status) => {
                    if *status {
                        "Dispenser une matière sur une période".into()
                    } else {
                        "Ne pas dispenser une matière sur une période".into()
                    }
                }
            },
        )
    }
}
