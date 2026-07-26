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
    LooseBalancingOptionsForSubject(collomatique_state_colloscopes::SubjectId),
    LoosePairingRulesForSubject(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PairingRuleId,
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
                    subject.parameters.name,
                    group_list.params().name,
                    period_num + 1
                ))
            }
            Self::LooseBalancingOptionsForSubject(subject_id) => {
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
                    "Perte des paramètres d'équilibrage pour la matière \"{}\"",
                    subject.parameters.name,
                ))
            }
            Self::LoosePairingRulesForSubject(subject_id, _rule_id) => {
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
                    "Suppression d'un appariement référençant la matière \"{}\"",
                    subject.parameters.name,
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
pub enum UpdateSubjectError {
    #[error("Subject ID {0:?} is invalid")]
    InvalidSubjectId(collomatique_state_colloscopes::SubjectId),
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
                    for (teacher_id, teacher) in data
                        .get_data()
                        .get_inner_data()
                        .params
                        .teachers
                        .teacher_map
                        .iter()
                    {
                        let teacher_id = &teacher_id;
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

                    for ((period_id, assoc_subject), group_list_id) in data
                        .get_data()
                        .get_inner_data()
                        .params
                        .group_lists
                        .subjects_associations
                        .iter()
                    {
                        if assoc_subject == *subject_id {
                            return Some(CleaningOp {
                                warning: SubjectsUpdateWarning::LooseGroupListAssociation(
                                    *subject_id,
                                    *group_list_id,
                                    period_id,
                                ),
                                op: UpdateOp::GroupLists(
                                    GroupListsUpdateOp::AssignGroupListToSubject(
                                        period_id,
                                        *subject_id,
                                        None,
                                    ),
                                ),
                            });
                        }
                    }

                    // Sparse slots ordering: a subject with interrogations but
                    // no slots yet has no row, so `first_slot_id_for_subject`
                    // returns `None` (no cleaning needed) rather than panicking.
                    if let Some(slot_id) = data
                        .get_data()
                        .get_inner_data()
                        .params
                        .slots
                        .first_slot_id_for_subject(*subject_id)
                    {
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LooseInterrogationSlots(*subject_id),
                            op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(slot_id)),
                        });
                    }

                    if data
                        .get_data()
                        .get_inner_data()
                        .params
                        .balancing
                        .subjects
                        .contains(subject_id)
                    {
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LooseBalancingOptionsForSubject(
                                *subject_id,
                            ),
                            op: UpdateOp::Balancing(BalancingUpdateOp::RemoveSubjectOptions(
                                *subject_id,
                            )),
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
                    if let Some(assigned_students) = data
                        .get_data()
                        .get_inner_data()
                        .params
                        .assignments
                        .students(*period_id, *subject_id)
                    {
                        if let Some(student_id) = assigned_students.iter().next() {
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
                        let inner = data.get_data().get_inner_data();
                        // Sparse slots ordering: a zero-slot subject has no row,
                        // so flatten `None` to an empty list instead of panicking.
                        let subject_slots: Vec<_> = inner
                            .params
                            .slots
                            .slots_for_subject(*subject_id)
                            .into_iter()
                            .flatten()
                            .map(|(slot_id, _slot)| *slot_id)
                            .collect();

                        for slot_id in subject_slots {
                            for (week_id, _groups) in
                                inner.colloscope.interrogations_for_slot(slot_id)
                            {
                                let (row_period, _pos) = inner
                                    .params
                                    .weeks
                                    .week_position(week_id)
                                    .expect("week id from a live colloscope row is valid");
                                if row_period != *period_id {
                                    continue;
                                }
                                return Some(CleaningOp {
                                    warning: SubjectsUpdateWarning::LooseColloscopeSlotsForPeriod(
                                        *subject_id,
                                        *period_id,
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

                    if let Some(group_list_id) = data
                        .get_data()
                        .get_inner_data()
                        .params
                        .group_lists
                        .subjects_associations
                        .get(&(*period_id, *subject_id))
                    {
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

                None
            }
            Self::DeleteSubject(subject_id) => {
                for (teacher_id, teacher) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .teachers
                    .teacher_map
                    .iter()
                {
                    let teacher_id = &teacher_id;
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

                for ((period_id, assoc_subject), group_list_id) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                {
                    if assoc_subject == *subject_id {
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LooseGroupListAssociation(
                                *subject_id,
                                *group_list_id,
                                period_id,
                            ),
                            op: UpdateOp::GroupLists(GroupListsUpdateOp::AssignGroupListToSubject(
                                period_id,
                                *subject_id,
                                None,
                            )),
                        });
                    }
                }

                for (incompat_id, incompat) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .incompats
                    .incompat_map
                    .iter()
                {
                    let incompat_id = &incompat_id;
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

                if let Some(slot_id) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slots
                    .first_slot_id_for_subject(*subject_id)
                {
                    return Some(CleaningOp {
                        warning: SubjectsUpdateWarning::LooseInterrogationSlots(*subject_id),
                        op: UpdateOp::Slots(SlotsUpdateOp::DeleteSlot(slot_id)),
                    });
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

                for (period_id, assoc_subject, assigned_students) in
                    data.get_data().get_inner_data().params.assignments.iter()
                {
                    if assoc_subject != *subject_id || excluded_periods.contains(&period_id) {
                        continue;
                    }

                    if let Some(student_id) = assigned_students.iter().next() {
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LooseStudentsAssignmentsForPeriod(
                                period_id,
                                *subject_id,
                            ),
                            op: UpdateOp::Assignments(AssignmentsUpdateOp::Assign(
                                period_id,
                                *student_id,
                                *subject_id,
                                false,
                            )),
                        });
                    }
                }

                if data
                    .get_data()
                    .get_inner_data()
                    .params
                    .balancing
                    .subjects
                    .contains(subject_id)
                {
                    return Some(CleaningOp {
                        warning: SubjectsUpdateWarning::LooseBalancingOptionsForSubject(
                            *subject_id,
                        ),
                        op: UpdateOp::Balancing(BalancingUpdateOp::RemoveSubjectOptions(
                            *subject_id,
                        )),
                    });
                }

                for (rule_id, rule) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .pairings
                    .pairing_rule_map
                    .iter()
                {
                    let rule_id = &rule_id;
                    if rule.antecedent().subject_id == *subject_id
                        || rule.consequent().subject_id == *subject_id
                    {
                        return Some(CleaningOp {
                            warning: SubjectsUpdateWarning::LoosePairingRulesForSubject(
                                *subject_id,
                                *rule_id,
                            ),
                            op: UpdateOp::Pairings(PairingsUpdateOp::DeletePairingRule(*rule_id)),
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
                                data.get_data()
                                    .get_inner_data()
                                    .params
                                    .subjects
                                    .ordered_subject_list
                                    .iter()
                                    .last()
                                    .map(|(id, _)| id),
                                collomatique_state_colloscopes::Subject {
                                    parameters: params.clone(),
                                    excluded_periods: BTreeSet::new(),
                                },
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("All data should be valid at this point");
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
                                },
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("All data should be valid at this point");

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
                        use collomatique_state_colloscopes::{
                            Error, PrecheckError, SubjectPrecheckError,
                        };
                        match e {
                            Error::Precheck(PrecheckError::Subject(
                                SubjectPrecheckError::InvalidSubjectId(id),
                            )) => DeleteSubjectError::InvalidSubjectId(id),
                            // Every reference to this subject (teacher subjects,
                            // group-list associations, incompats, slots,
                            // assignments, balancing options, pairing rules) is
                            // stripped by get_next_cleaning_op before Remove reaches
                            // apply, so a leftover dangle here is a cleaning-contract
                            // breach, not user input.
                            _ => panic!("Unexpected error during DeleteSubject: {e:?}"),
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
