use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GeneralPlanningUpdateWarning {
    LooseStudentExclusionForPeriod(
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseStudentAssignmentsForPeriod(collomatique_state_colloscopes::PeriodId),
    LooseSubjectDataForPeriod(
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseSubjectAssociation(
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseWeekPatternDataForPeriod(
        collomatique_state_colloscopes::WeekPatternId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LoosePeriodDataInColloscope(collomatique_state_colloscopes::PeriodId),
    LoosePairingRuleExclusionForPeriod(
        collomatique_state_colloscopes::PairingRuleId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseSlotPairingRuleExclusionForPeriod(
        collomatique_state_colloscopes::SlotPairingRuleId,
        collomatique_state_colloscopes::PeriodId,
    ),
}

impl GeneralPlanningUpdateWarning {
    pub(crate) fn build_desc_from_data<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<String> {
        match self {
            GeneralPlanningUpdateWarning::LooseStudentExclusionForPeriod(student_id, period_id) => {
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
                    "Perte des informations de présence de l'élève {} {} sur la période {}",
                    student.desc.firstname,
                    student.desc.surname,
                    period_index + 1
                ))
            }
            GeneralPlanningUpdateWarning::LooseStudentAssignmentsForPeriod(period_id) => {
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
                    "Perte des inscriptions des élèves sur la période {}",
                    period_index + 1
                ))
            }
            GeneralPlanningUpdateWarning::LooseSubjectDataForPeriod(subject_id, period_id) => {
                let Some(subject) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .find_subject(*subject_id)
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
                    "Perte des informations de la matière \"{}\" sur la période {}",
                    subject.parameters.name,
                    period_index + 1
                ))
            }
            GeneralPlanningUpdateWarning::LooseSubjectAssociation(
                group_list_id,
                subject_id,
                period_id,
            ) => {
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
                    group_list.params.name,
                    period_num + 1
                ))
            }
            GeneralPlanningUpdateWarning::LooseWeekPatternDataForPeriod(
                week_pattern_id,
                period_id,
            ) => {
                let Some(week_pattern) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .week_patterns
                    .week_pattern_map
                    .get(week_pattern_id)
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
                    "Perte des informations de modèle de périodicité \"{}\" sur la période {}",
                    week_pattern.name,
                    period_index + 1
                ))
            }
            GeneralPlanningUpdateWarning::LoosePeriodDataInColloscope(period_id) => {
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
                    "Perte de tout ou d'une partie du colloscope sur la période {}",
                    period_index + 1
                ))
            }
            GeneralPlanningUpdateWarning::LoosePairingRuleExclusionForPeriod(
                _rule_id,
                period_id,
            ) => {
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
                    "Modification d'un appariement (retrait de la période exclue {})",
                    period_index + 1
                ))
            }
            GeneralPlanningUpdateWarning::LooseSlotPairingRuleExclusionForPeriod(
                _rule_id,
                period_id,
            ) => {
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
                    "Modification d'un appariement de créneaux (retrait de la période exclue {})",
                    period_index + 1
                ))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeneralPlanningUpdateOp {
    DeleteFirstWeek,
    UpdateFirstWeek(collomatique_time::WeekStart),
    AddNewPeriod(usize),
    UpdatePeriodWeekCount(collomatique_state_colloscopes::PeriodId, usize),
    DeletePeriod(collomatique_state_colloscopes::PeriodId),
    CutPeriod(collomatique_state_colloscopes::PeriodId, usize),
    MergeWithPreviousPeriod(collomatique_state_colloscopes::PeriodId),
    UpdateWeekStatus(collomatique_state_colloscopes::PeriodId, usize, bool),
    UpdateWeekAnnotation(
        collomatique_state_colloscopes::PeriodId,
        usize,
        Option<non_empty_string::NonEmptyString>,
    ),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeneralPlanningUpdateError {
    #[error(transparent)]
    UpdatePeriodWeekCount(#[from] UpdatePeriodWeekCountError),
    #[error(transparent)]
    DeletePeriod(#[from] DeletePeriodError),
    #[error(transparent)]
    CutPeriod(#[from] CutPeriodError),
    #[error(transparent)]
    MergeWithPreviousPeriod(#[from] MergeWithPreviousPeriodError),
    #[error(transparent)]
    UpdateWeekStatus(#[from] UpdateWeekStatusError),
    #[error(transparent)]
    UpdateWeekAnnotation(#[from] UpdateWeekAnnotationError),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdatePeriodWeekCountError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Subject {0:?} implies a minimum total number of weeks of {1}")]
    SubjectImpliesMinimumWeekCount(collomatique_state_colloscopes::SubjectId, usize),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeletePeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum CutPeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Remaining week count ({0}) is larger than available week count ({1})")]
    RemainingWeekCountTooBig(usize, usize),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum MergeWithPreviousPeriodError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("This is the first period and cannot be merged with the non-existent previous one")]
    NoPreviousPeriodToMergeWith,
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateWeekStatusError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Week number {0} is larger that the number of available weeks ({1})")]
    InvalidWeekNumber(usize, usize),
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum UpdateWeekAnnotationError {
    #[error("Period ID {0:?} is invalid")]
    InvalidPeriodId(collomatique_state_colloscopes::PeriodId),
    #[error("Week number {0} is larger that the number of available weeks ({1})")]
    InvalidWeekNumber(usize, usize),
}

impl GeneralPlanningUpdateOp {
    pub(crate) fn get_next_cleaning_op<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &T,
    ) -> Option<CleaningOp<GeneralPlanningUpdateWarning>> {
        match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => None,
            GeneralPlanningUpdateOp::UpdateFirstWeek(_) => None,
            GeneralPlanningUpdateOp::AddNewPeriod(_) => None,
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period_id, week_count) => {
                let Some((_pos, first_week)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position_and_first_week(*period_id)
                else {
                    return None;
                };
                let old_week_count = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .week_count_of(*period_id)
                    .expect("period id is valid");

                if *week_count >= old_week_count {
                    return None;
                }

                let colloscope_period = data
                    .get_data()
                    .get_inner_data()
                    .colloscope
                    .period_map
                    .get(period_id)
                    .expect("Period ID should be valid at this point");

                if !colloscope_period.is_empty() {
                    for (slot_id, collo_slot) in &colloscope_period.slot_map {
                        for week in *week_count..old_week_count {
                            if let Some(interrogation) = &collo_slot.interrogations[week]
                                && !interrogation.is_empty()
                            {
                                return Some(CleaningOp {
                                        warning: GeneralPlanningUpdateWarning::LoosePeriodDataInColloscope(*period_id),
                                        op: UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                            *period_id,
                                            *slot_id,
                                            week,
                                            collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation::default(),
                                        )),
                                    });
                            }
                        }
                    }
                }

                let first_week_to_remove = first_week + *week_count;
                let weeks_to_remove = old_week_count - *week_count;

                for (week_pattern_id, week_pattern) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .week_patterns
                    .week_pattern_map
                    .iter()
                {
                    let week_pattern_id = &week_pattern_id;
                    if !week_pattern.can_remove_weeks(first_week_to_remove, weeks_to_remove) {
                        let mut new_week_patten = week_pattern.clone();
                        new_week_patten.clean_weeks(first_week_to_remove, weeks_to_remove);

                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseWeekPatternDataForPeriod(
                                *week_pattern_id,
                                *period_id,
                            ),
                            op: UpdateOp::WeekPatterns(WeekPatternsUpdateOp::UpdateWeekPattern(
                                *week_pattern_id,
                                new_week_patten,
                            )),
                        });
                    }
                }

                None
            }
            GeneralPlanningUpdateOp::CutPeriod(_, _) => None,
            GeneralPlanningUpdateOp::UpdateWeekStatus(period_id, week, status) => {
                if *status {
                    return None;
                }

                let Some(colloscope_period) = data
                    .get_data()
                    .get_inner_data()
                    .colloscope
                    .period_map
                    .get(period_id)
                else {
                    return None;
                };

                if !colloscope_period.is_empty() {
                    for (slot_id, collo_slot) in &colloscope_period.slot_map {
                        let Some(interrogation_opt) = collo_slot.interrogations.get(*week) else {
                            return None;
                        };
                        if let Some(interrogation) = interrogation_opt
                            && !interrogation.is_empty()
                        {
                            return Some(CleaningOp {
                                    warning: GeneralPlanningUpdateWarning::LoosePeriodDataInColloscope(*period_id),
                                    op: UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                        *period_id,
                                        *slot_id,
                                        *week,
                                        collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation::default(),
                                    )),
                                });
                        }
                    }
                }

                None
            }
            GeneralPlanningUpdateOp::UpdateWeekAnnotation(_, _, _) => None,
            GeneralPlanningUpdateOp::DeletePeriod(period_id) => {
                let Some((_pos, first_week)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position_and_first_week(*period_id)
                else {
                    return None;
                };
                let week_count = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .week_count_of(*period_id)
                    .expect("period id is valid");

                let colloscope_period = data
                    .get_data()
                    .get_inner_data()
                    .colloscope
                    .period_map
                    .get(period_id)
                    .expect("Period ID should be valid at this point");

                if !colloscope_period.is_empty() {
                    for (slot_id, collo_slot) in &colloscope_period.slot_map {
                        for week in 0..collo_slot.interrogations.len() {
                            let interrogation_opt = &collo_slot.interrogations[week];
                            let Some(interrogation) = interrogation_opt else {
                                continue;
                            };
                            if interrogation.is_empty() {
                                continue;
                            }
                            return Some(CleaningOp {
                                warning: GeneralPlanningUpdateWarning::LoosePeriodDataInColloscope(*period_id),
                                op: UpdateOp::Colloscope(ColloscopeUpdateOp::UpdateColloscopeInterrogation(
                                    *period_id,
                                    *slot_id,
                                    week,
                                    collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation::default(),
                                )),
                            });
                        }
                    }
                }

                for (week_pattern_id, week_pattern) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .week_patterns
                    .week_pattern_map
                    .iter()
                {
                    let week_pattern_id = &week_pattern_id;
                    if !week_pattern.can_remove_weeks(first_week, week_count) {
                        let mut new_week_patten = week_pattern.clone();
                        new_week_patten.clean_weeks(first_week, week_count);

                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseWeekPatternDataForPeriod(
                                *week_pattern_id,
                                *period_id,
                            ),
                            op: UpdateOp::WeekPatterns(WeekPatternsUpdateOp::UpdateWeekPattern(
                                *week_pattern_id,
                                new_week_patten,
                            )),
                        });
                    }
                }

                for (subject_id, subject) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                    .iter()
                {
                    let subject_id = &subject_id;
                    if subject.excluded_periods.contains(period_id) {
                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseSubjectDataForPeriod(
                                *subject_id,
                                *period_id,
                            ),
                            op: UpdateOp::Subjects(SubjectsUpdateOp::UpdatePeriodStatus(
                                *subject_id,
                                *period_id,
                                true,
                            )),
                        });
                    }
                }

                for (student_id, student) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .iter()
                {
                    let student_id = &student_id;
                    if student.excluded_periods.contains(period_id) {
                        let mut new_student = student.clone();
                        new_student.excluded_periods.remove(period_id);
                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseStudentExclusionForPeriod(
                                *student_id,
                                *period_id,
                            ),
                            op: UpdateOp::Students(StudentsUpdateOp::UpdateStudent(
                                *student_id,
                                new_student,
                            )),
                        });
                    }
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
                    if rule.excluded_periods.contains(period_id) {
                        let mut new_rule = rule.clone();
                        new_rule.excluded_periods.remove(period_id);
                        return Some(CleaningOp {
                            warning:
                                GeneralPlanningUpdateWarning::LoosePairingRuleExclusionForPeriod(
                                    *rule_id, *period_id,
                                ),
                            op: UpdateOp::Pairings(PairingsUpdateOp::UpdatePairingRule(
                                *rule_id, new_rule,
                            )),
                        });
                    }
                }

                for (rule_id, rule) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .iter()
                {
                    let rule_id = &rule_id;
                    if rule.excluded_periods.contains(period_id) {
                        let mut new_rule = rule.clone();
                        new_rule.excluded_periods.remove(period_id);
                        return Some(CleaningOp {
                            warning:
                                GeneralPlanningUpdateWarning::LooseSlotPairingRuleExclusionForPeriod(
                                    *rule_id, *period_id,
                                ),
                            op: UpdateOp::SlotPairings(
                                SlotPairingsUpdateOp::UpdateSlotPairingRule(*rule_id, new_rule),
                            ),
                        });
                    }
                }

                for (subject_id, assigned_students) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .subjects_for_period(*period_id)
                {
                    if let Some(student_id) = assigned_students.first() {
                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseStudentAssignmentsForPeriod(
                                *period_id,
                            ),
                            op: UpdateOp::Assignments(AssignmentsUpdateOp::Assign(
                                *period_id,
                                *student_id,
                                subject_id,
                                false,
                            )),
                        });
                    }
                }

                if let Some(((_period, subject_id), group_list_id)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                    .find(|((period, _subject), _)| *period == *period_id)
                {
                    return Some(CleaningOp {
                        warning: GeneralPlanningUpdateWarning::LooseSubjectAssociation(
                            *group_list_id,
                            subject_id,
                            *period_id,
                        ),
                        op: UpdateOp::GroupLists(GroupListsUpdateOp::AssignGroupListToSubject(
                            *period_id, subject_id, None,
                        )),
                    });
                }

                None
            }
            GeneralPlanningUpdateOp::MergeWithPreviousPeriod(period_id) => {
                let Some(pos) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return None;
                };
                if pos == 0 {
                    return None;
                }
                let previous_id = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .period_id_at(pos - 1)
                    .expect("pos > 0 checked above");

                for (subject_id, subject) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                    .iter()
                {
                    let subject_id = &subject_id;
                    if subject.excluded_periods.contains(period_id)
                        != subject.excluded_periods.contains(&previous_id)
                    {
                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseSubjectDataForPeriod(
                                *subject_id,
                                *period_id,
                            ),
                            op: UpdateOp::Subjects(SubjectsUpdateOp::UpdatePeriodStatus(
                                *subject_id,
                                *period_id,
                                !subject.excluded_periods.contains(&previous_id),
                            )),
                        });
                    }
                }

                for (student_id, student) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .iter()
                {
                    let student_id = &student_id;
                    if student.excluded_periods.contains(period_id)
                        != student.excluded_periods.contains(&previous_id)
                    {
                        let mut new_student = student.clone();
                        if student.excluded_periods.contains(&previous_id) {
                            new_student.excluded_periods.insert(*period_id);
                        } else {
                            new_student.excluded_periods.remove(period_id);
                        }
                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseStudentExclusionForPeriod(
                                *student_id,
                                *period_id,
                            ),
                            op: UpdateOp::Students(StudentsUpdateOp::UpdateStudent(
                                *student_id,
                                new_student,
                            )),
                        });
                    }
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
                    if rule.excluded_periods.contains(period_id) {
                        let mut new_rule = rule.clone();
                        new_rule.excluded_periods.remove(period_id);
                        return Some(CleaningOp {
                            warning:
                                GeneralPlanningUpdateWarning::LoosePairingRuleExclusionForPeriod(
                                    *rule_id, *period_id,
                                ),
                            op: UpdateOp::Pairings(PairingsUpdateOp::UpdatePairingRule(
                                *rule_id, new_rule,
                            )),
                        });
                    }
                }

                for (rule_id, rule) in data
                    .get_data()
                    .get_inner_data()
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .iter()
                {
                    let rule_id = &rule_id;
                    if rule.excluded_periods.contains(period_id) {
                        let mut new_rule = rule.clone();
                        new_rule.excluded_periods.remove(period_id);
                        return Some(CleaningOp {
                            warning:
                                GeneralPlanningUpdateWarning::LooseSlotPairingRuleExclusionForPeriod(
                                    *rule_id, *period_id,
                                ),
                            op: UpdateOp::SlotPairings(
                                SlotPairingsUpdateOp::UpdateSlotPairingRule(*rule_id, new_rule),
                            ),
                        });
                    }
                }

                let period_assignments: std::collections::BTreeMap<_, _> = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .subjects_for_period(*period_id)
                    .map(|(subject_id, students)| (subject_id, students.clone()))
                    .collect();

                let previous_assignments: std::collections::BTreeMap<_, _> = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .subjects_for_period(previous_id)
                    .map(|(subject_id, students)| (subject_id, students.clone()))
                    .collect();

                for (subject_id, assigned_students) in &period_assignments {
                    match previous_assignments.get(subject_id) {
                        None => {
                            if let Some(student_id) = assigned_students.iter().next() {
                                return Some(CleaningOp {
                                    warning: GeneralPlanningUpdateWarning::LooseStudentAssignmentsForPeriod(*period_id),
                                    op: UpdateOp::Assignments(
                                            AssignmentsUpdateOp::Assign(*period_id, *student_id, *subject_id, false)
                                        ),
                                });
                            }
                        }
                        Some(previous_students) => {
                            for student_id in data
                                .get_data()
                                .get_inner_data()
                                .params
                                .students
                                .student_map
                                .keys()
                            {
                                if assigned_students.contains(&student_id)
                                    != previous_students.contains(&student_id)
                                {
                                    return Some(CleaningOp {
                                        warning: GeneralPlanningUpdateWarning::LooseStudentAssignmentsForPeriod(*period_id),
                                        op: UpdateOp::Assignments(
                                            AssignmentsUpdateOp::Assign(*period_id, student_id, *subject_id, previous_students.contains(&student_id))
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }

                if let Some(((_period, subject_id), group_list_id)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                    .find(|((period, _subject), _)| *period == *period_id)
                {
                    return Some(CleaningOp {
                        warning: GeneralPlanningUpdateWarning::LooseSubjectAssociation(
                            *group_list_id,
                            subject_id,
                            *period_id,
                        ),
                        op: UpdateOp::GroupLists(GroupListsUpdateOp::AssignGroupListToSubject(
                            *period_id, subject_id, None,
                        )),
                    });
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
    ) -> Result<Option<collomatique_state_colloscopes::PeriodId>, GeneralPlanningUpdateError> {
        match self {
            GeneralPlanningUpdateOp::DeleteFirstWeek => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::ChangeStartDate(None),
                        ),
                        self.get_desc(),
                    )
                    .expect("Deleting first week should always work");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateFirstWeek(date) => {
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::ChangeStartDate(Some(
                                date.clone(),
                            )),
                        ),
                        self.get_desc(),
                    )
                    .expect("Updating first week should always work");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::AddNewPeriod(week_count) => {
                // Create the period empty, then grow it one week at a time so
                // the week ops are the sole authority on week data.
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            match data
                                .get_data()
                                .get_inner_data()
                                .params
                                .periods
                                .period_ids()
                                .last()
                            {
                                Some(id) => {
                                    collomatique_state_colloscopes::PeriodOp::AddAfter(id, vec![])
                                }
                                None => collomatique_state_colloscopes::PeriodOp::AddFront(vec![]),
                            },
                        ),
                        self.get_desc(),
                    )
                    .expect("Adding a period should never fail");
                let new_id = match result {
                    Some(collomatique_state_colloscopes::NewId::PeriodId(id)) => id,
                    _ => panic!("Unexpected result! {:?}", result),
                };

                let mut prev_week_id: Option<collomatique_state_colloscopes::WeekId> = None;
                for _ in 0..*week_count {
                    let week_desc = collomatique_state_colloscopes::periods::WeekDesc::new(true);
                    let week_op = match prev_week_id {
                        None => collomatique_state_colloscopes::WeekOp::AddFront(new_id, week_desc),
                        Some(prev) => {
                            collomatique_state_colloscopes::WeekOp::AddAfter(prev, week_desc)
                        }
                    };
                    let result = data
                        .apply(
                            collomatique_state_colloscopes::Op::Week(week_op),
                            self.get_desc(),
                        )
                        .expect("Adding a week to a fresh period should never fail");
                    match result {
                        Some(collomatique_state_colloscopes::NewId::WeekId(id)) => {
                            prev_week_id = Some(id)
                        }
                        _ => panic!("Unexpected result! {:?}", result),
                    }
                }

                Ok(Some(new_id))
            }
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period_id, week_count) => {
                let old_week_count = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .week_count_of(*period_id)
                    .ok_or(UpdatePeriodWeekCountError::InvalidPeriodId(*period_id))?;

                if *week_count > old_week_count {
                    // Grow: append weeks off the last one (front if the period
                    // is empty), copying the last week's description — the shape
                    // the old whole-period update produced via `Vec::resize`.
                    let fill_desc = data
                        .get_data()
                        .get_inner_data()
                        .params
                        .periods
                        .weeks_vec_of(*period_id)
                        .expect("period id valid")
                        .last()
                        .cloned()
                        .unwrap_or(collomatique_state_colloscopes::periods::WeekDesc::new(true));

                    let mut prev_week_id = if old_week_count == 0 {
                        None
                    } else {
                        data.get_data()
                            .get_inner_data()
                            .params
                            .periods
                            .week_id_at(*period_id, old_week_count - 1)
                    };

                    for _ in old_week_count..*week_count {
                        let week_op = match prev_week_id {
                            None => collomatique_state_colloscopes::WeekOp::AddFront(
                                *period_id,
                                fill_desc.clone(),
                            ),
                            Some(prev) => collomatique_state_colloscopes::WeekOp::AddAfter(
                                prev,
                                fill_desc.clone(),
                            ),
                        };
                        let result = data
                            .apply(
                                collomatique_state_colloscopes::Op::Week(week_op),
                                self.get_desc(),
                            )
                            .expect("Growing a period should never fail");
                        match result {
                            Some(collomatique_state_colloscopes::NewId::WeekId(id)) => {
                                prev_week_id = Some(id)
                            }
                            _ => panic!("Unexpected result! {:?}", result),
                        }
                    }
                } else if *week_count < old_week_count {
                    // Shrink: the cleaning cascade has already emptied the
                    // doomed weeks' colloscope cells and made their pattern bits
                    // trivial, so removing them last-to-first cannot fail.
                    for pos in (*week_count..old_week_count).rev() {
                        let week_id = data
                            .get_data()
                            .get_inner_data()
                            .params
                            .periods
                            .week_id_at(*period_id, pos)
                            .expect("position in range");
                        let result = data
                            .apply(
                                collomatique_state_colloscopes::Op::Week(
                                    collomatique_state_colloscopes::WeekOp::Remove(week_id),
                                ),
                                self.get_desc(),
                            )
                            .expect("Cleaning made the removed weeks trivial");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                Ok(None)
            }
            GeneralPlanningUpdateOp::DeletePeriod(period_id) => {
                // Empty the period one week at a time — the cleaning cascade has
                // already made every week trivial (empty cells, removable
                // pattern bits) — then remove the now-empty period.
                if let Some(week_count) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .week_count_of(*period_id)
                {
                    for pos in (0..week_count).rev() {
                        let week_id = data
                            .get_data()
                            .get_inner_data()
                            .params
                            .periods
                            .week_id_at(*period_id, pos)
                            .expect("position in range");
                        let result = data
                            .apply(
                                collomatique_state_colloscopes::Op::Week(
                                    collomatique_state_colloscopes::WeekOp::Remove(week_id),
                                ),
                                self.get_desc(),
                            )
                            .expect("Cleaning made every week of the period trivial");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Remove(*period_id),
                        ),
                        self.get_desc(),
                    )
                    .expect("All data should be valid at this point");

                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                Ok(None)
            }
            GeneralPlanningUpdateOp::CutPeriod(period_id, new_week_count) => {
                let old_week_count = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .week_count_of(*period_id)
                    .ok_or(CutPeriodError::InvalidPeriodId(*period_id))?;

                if *new_week_count > old_week_count {
                    Err(CutPeriodError::RemainingWeekCountTooBig(
                        *new_week_count,
                        old_week_count,
                    ))?;
                }

                // Create the tail period empty; the tail weeks are moved into it
                // below. Content (colloscope cells + week-pattern bits) travels
                // with each week, so no save/clean/restore dance is needed.
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::AddAfter(*period_id, vec![]),
                        ),
                        self.get_desc(),
                    )
                    .expect("At this point, period id should be valid");
                let new_id = match result {
                    Some(collomatique_state_colloscopes::NewId::PeriodId(id)) => id,
                    _ => panic!("Unexpected result! {:?}", result),
                };

                // Propagate period-level references to the new period *before*
                // moving weeks: `WeekOp::Move`'s guard needs the destination
                // subject exclusions (which slots exist) and group-list
                // associations (which group numbers fit) in place before a
                // non-empty colloscope cell can travel.
                let ordered_subject_list = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                    .clone();
                for (subject_id, subject) in ordered_subject_list.iter() {
                    if subject.excluded_periods.contains(period_id) {
                        let mut new_subject = subject.clone();
                        new_subject.excluded_periods.insert(new_id);
                        let result = data
                            .apply(
                                collomatique_state_colloscopes::Op::Subject(
                                    collomatique_state_colloscopes::SubjectOp::Update(
                                        subject_id,
                                        new_subject,
                                    ),
                                ),
                                self.get_desc(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let student_map = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .students
                    .student_map
                    .clone();
                for (student_id, student) in student_map.iter() {
                    if student.excluded_periods.contains(period_id) {
                        let mut new_student = student.clone();
                        new_student.excluded_periods.insert(new_id);
                        let result = data
                            .apply(
                                collomatique_state_colloscopes::Op::Student(
                                    collomatique_state_colloscopes::StudentOp::Update(
                                        student_id,
                                        new_student,
                                    ),
                                ),
                                self.get_desc(),
                            )
                            .expect("All data should be valid at this point");
                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let period_assignments: Vec<(_, std::collections::BTreeSet<_>)> = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .subjects_for_period(*period_id)
                    .map(|(subject_id, students)| (subject_id, students.clone()))
                    .collect();

                for (subject_id, assigned_students) in period_assignments {
                    for student_id in assigned_students {
                        let result = data
                            .apply(
                                collomatique_state_colloscopes::Op::Assignment(
                                    collomatique_state_colloscopes::AssignmentOp::Assign(
                                        new_id, student_id, subject_id, true,
                                    ),
                                ),
                                self.get_desc(),
                            )
                            .expect("All data should be valid at this point");

                        if result.is_some() {
                            panic!("Unexpected result! {:?}", result);
                        }
                    }
                }

                let period_associations: Vec<(_, _)> = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .iter()
                    .filter_map(|((period, subject), group_list)| {
                        (period == *period_id).then_some((subject, *group_list))
                    })
                    .collect();
                for (subject_id, group_list_id) in period_associations {
                    let result = data
                        .apply(
                            collomatique_state_colloscopes::Op::GroupList(
                                collomatique_state_colloscopes::GroupListOp::AssignToSubject(
                                    new_id,
                                    subject_id,
                                    Some(group_list_id),
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("All data should be valid at this point");
                    if result.is_some() {
                        panic!("Unexpected result! {:?}", result);
                    }
                }

                // Move the tail weeks into the new period, preserving order.
                // Detaching each week from the source automatically shortens it;
                // week ids are stable, so capture them before the first move.
                let tail_week_ids: Vec<collomatique_state_colloscopes::WeekId> = (*new_week_count
                    ..old_week_count)
                    .map(|pos| {
                        data.get_data()
                            .get_inner_data()
                            .params
                            .periods
                            .week_id_at(*period_id, pos)
                            .expect("tail week exists")
                    })
                    .collect();
                for (dest_pos, week_id) in tail_week_ids.into_iter().enumerate() {
                    let result = data
                        .apply(
                            collomatique_state_colloscopes::Op::Week(
                                collomatique_state_colloscopes::WeekOp::Move(
                                    week_id, new_id, dest_pos,
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("Moving a tail week into the fresh period must succeed");
                    if result.is_some() {
                        panic!("Unexpected result! {:?}", result);
                    }
                }

                Ok(Some(new_id))
            }
            GeneralPlanningUpdateOp::MergeWithPreviousPeriod(period_id) => {
                let pos = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .ok_or(MergeWithPreviousPeriodError::InvalidPeriodId(*period_id))?;
                if pos == 0 {
                    Err(MergeWithPreviousPeriodError::NoPreviousPeriodToMergeWith)?;
                }

                let previous_id = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .period_id_at(pos - 1)
                    .expect("pos > 0 checked above");

                // Append every week of this period to the end of the previous
                // one, preserving order. Content travels with each week (the
                // cleaning cascade has already reconciled exclusions,
                // assignments and associations, emptying any colloscope cell it
                // could not carry over). Detaching from the source shortens it;
                // week ids are stable, so capture them before the first move.
                let append_start = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .week_count_of(previous_id)
                    .expect("previous period id is valid");
                let week_count = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .week_count_of(*period_id)
                    .expect("period id is valid");
                let week_ids: Vec<collomatique_state_colloscopes::WeekId> = (0..week_count)
                    .map(|pos| {
                        data.get_data()
                            .get_inner_data()
                            .params
                            .periods
                            .week_id_at(*period_id, pos)
                            .expect("week exists")
                    })
                    .collect();
                for (offset, week_id) in week_ids.into_iter().enumerate() {
                    let result = data
                        .apply(
                            collomatique_state_colloscopes::Op::Week(
                                collomatique_state_colloscopes::WeekOp::Move(
                                    week_id,
                                    previous_id,
                                    append_start + offset,
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("Merging a week into the previous period must succeed");
                    if result.is_some() {
                        panic!("Unexpected result! {:?}", result);
                    }
                }

                let rec_result =
                    UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::DeletePeriod(*period_id))
                        .rec_apply_no_session(data)
                        .expect("All data should be valid at this point");

                let result = rec_result.new_id;

                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateWeekStatus(period_id, week_num, state) => {
                let desc = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .weeks_vec_of(*period_id)
                    .ok_or(UpdateWeekStatusError::InvalidPeriodId(*period_id))?;

                if *week_num >= desc.len() {
                    Err(UpdateWeekStatusError::InvalidWeekNumber(
                        *week_num,
                        desc.len(),
                    ))?;
                }

                let week_id = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .week_id_at(*period_id, *week_num)
                    .expect("week number checked in range above");
                let mut new_desc = desc[*week_num].clone();
                new_desc.interrogations = *state;

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Week(
                            collomatique_state_colloscopes::WeekOp::Update(week_id, new_desc),
                        ),
                        self.get_desc(),
                    )
                    .expect("At this point, parameters should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateWeekAnnotation(period_id, week_num, annotation) => {
                let desc = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .weeks_vec_of(*period_id)
                    .ok_or(UpdateWeekAnnotationError::InvalidPeriodId(*period_id))?;

                if *week_num >= desc.len() {
                    Err(UpdateWeekAnnotationError::InvalidWeekNumber(
                        *week_num,
                        desc.len(),
                    ))?;
                }

                let week_id = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .week_id_at(*period_id, *week_num)
                    .expect("week number checked in range above");
                let mut new_desc = desc[*week_num].clone();
                new_desc.annotation = annotation.clone();

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Week(
                            collomatique_state_colloscopes::WeekOp::Update(week_id, new_desc),
                        ),
                        self.get_desc(),
                    )
                    .expect("At this point, parameters should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
        }
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::GeneralPlanning,
            match self {
                GeneralPlanningUpdateOp::DeleteFirstWeek => "Effacer le début des colles".into(),
                GeneralPlanningUpdateOp::UpdateFirstWeek(_date) => {
                    "Changer le début des colles".into()
                }
                GeneralPlanningUpdateOp::AddNewPeriod(_week_count) => "Ajouter une période".into(),
                GeneralPlanningUpdateOp::UpdatePeriodWeekCount(_period_id, _week_count) => {
                    "Modifier une période".into()
                }
                GeneralPlanningUpdateOp::DeletePeriod(_period_id) => "Supprimer une période".into(),
                GeneralPlanningUpdateOp::CutPeriod(_period_id, _new_week_count) => {
                    "Découper une période".into()
                }
                GeneralPlanningUpdateOp::MergeWithPreviousPeriod(_period_id) => {
                    "Fusionner deux périodes".into()
                }
                GeneralPlanningUpdateOp::UpdateWeekStatus(_period_id, _week_num, state) => {
                    if *state {
                        "Ajouter une semaine de colle".into()
                    } else {
                        "Supprimer une semaine de colle".into()
                    }
                }
                GeneralPlanningUpdateOp::UpdateWeekAnnotation(
                    _period_id,
                    _week_num,
                    annotation,
                ) => {
                    if annotation.is_some() {
                        "Annoter une semaine de colle".into()
                    } else {
                        "Effacer l'annotation d'une semaine de colle".into()
                    }
                }
            },
        )
    }
}
