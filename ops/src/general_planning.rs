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
    LooseRuleDataForPeriod(
        collomatique_state_colloscopes::RuleId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LooseWeekPatternDataForPeriod(
        collomatique_state_colloscopes::WeekPatternId,
        collomatique_state_colloscopes::PeriodId,
    ),
    LoosePeriodDataInColloscope(collomatique_state_colloscopes::PeriodId),
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
                    subject.parameters.name, group_list.params.name, period_num+1
                ))
            }
            GeneralPlanningUpdateWarning::LooseRuleDataForPeriod(rule_id, period_id) => {
                let Some(rule) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .rules
                    .rule_map
                    .get(rule_id)
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
                    "Perte des informations de la règle \"{}\" sur la période {}",
                    rule.name,
                    period_index + 1
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GeneralPlanningUpdateOp {
    DeleteFirstWeek,
    UpdateFirstWeek(collomatique_time::NaiveMondayDate),
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
                let Some((pos, first_week)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position_and_first_week(*period_id)
                else {
                    return None;
                };
                let period = &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[pos]
                    .1;
                let old_week_count = period.len();

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
                        for week in old_week_count..*week_count {
                            if let Some(interrogation) = &collo_slot.interrogations[week] {
                                if !interrogation.is_empty() {
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
                }

                let first_week_to_remove = first_week + *week_count;
                let weeks_to_remove = old_week_count - *week_count;

                for (week_pattern_id, week_pattern) in &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .week_patterns
                    .week_pattern_map
                {
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
                        if let Some(interrogation) = interrogation_opt {
                            if !interrogation.is_empty() {
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
                }

                None
            }
            GeneralPlanningUpdateOp::UpdateWeekAnnotation(_, _, _) => None,
            GeneralPlanningUpdateOp::DeletePeriod(period_id) => {
                let Some((pos, first_week)) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position_and_first_week(*period_id)
                else {
                    return None;
                };
                let period = &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[pos]
                    .1;
                let week_count = period.len();

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

                for (week_pattern_id, week_pattern) in &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .week_patterns
                    .week_pattern_map
                {
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

                for (subject_id, subject) in &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                {
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

                for (rule_id, rule) in &data.get_data().get_inner_data().params.rules.rule_map {
                    if rule.excluded_periods.contains(period_id) {
                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseRuleDataForPeriod(
                                *rule_id, *period_id,
                            ),
                            op: UpdateOp::Rules(RulesUpdateOp::UpdatePeriodStatusForRule(
                                *rule_id, *period_id, true,
                            )),
                        });
                    }
                }

                for (student_id, student) in
                    &data.get_data().get_inner_data().params.students.student_map
                {
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

                let Some(period_assignments) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .period_map
                    .get(period_id)
                else {
                    return None;
                };

                for (subject_id, assigned_students) in &period_assignments.subject_map {
                    if let Some(student_id) = assigned_students.first() {
                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseStudentAssignmentsForPeriod(
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

                if let Some(subject_map) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .get(period_id)
                {
                    for (subject_id, group_list_id) in subject_map {
                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseSubjectAssociation(
                                *group_list_id,
                                *subject_id,
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
                    .ordered_period_list[pos - 1]
                    .0;

                for (subject_id, subject) in &data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                {
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

                for (rule_id, rule) in &data.get_data().get_inner_data().params.rules.rule_map {
                    if rule.excluded_periods.contains(period_id)
                        != rule.excluded_periods.contains(&previous_id)
                    {
                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseRuleDataForPeriod(
                                *rule_id, *period_id,
                            ),
                            op: UpdateOp::Rules(RulesUpdateOp::UpdatePeriodStatusForRule(
                                *rule_id,
                                *period_id,
                                !rule.excluded_periods.contains(&previous_id),
                            )),
                        });
                    }
                }

                for (student_id, student) in
                    &data.get_data().get_inner_data().params.students.student_map
                {
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

                let Some(period_assignments) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .period_map
                    .get(period_id)
                    .cloned()
                else {
                    return None;
                };

                let Some(previous_assignments) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .period_map
                    .get(&previous_id)
                else {
                    return None;
                };

                for (subject_id, assigned_students) in &period_assignments.subject_map {
                    match previous_assignments.subject_map.get(subject_id) {
                        None => {
                            for student_id in assigned_students {
                                return Some(CleaningOp {
                                    warning: GeneralPlanningUpdateWarning::LooseStudentAssignmentsForPeriod(*period_id),
                                    op: UpdateOp::Assignments(
                                            AssignmentsUpdateOp::Assign(*period_id, *student_id, *subject_id, false)
                                        ),
                                });
                            }
                        }
                        Some(previous_students) => {
                            for (student_id, _student) in
                                &data.get_data().get_inner_data().params.students.student_map
                            {
                                if assigned_students.contains(student_id)
                                    != previous_students.contains(student_id)
                                {
                                    return Some(CleaningOp {
                                        warning: GeneralPlanningUpdateWarning::LooseStudentAssignmentsForPeriod(*period_id),
                                        op: UpdateOp::Assignments(
                                            AssignmentsUpdateOp::Assign(*period_id, *student_id, *subject_id, previous_students.contains(student_id))
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
                    for (subject_id, group_list_id) in subject_map {
                        return Some(CleaningOp {
                            warning: GeneralPlanningUpdateWarning::LooseSubjectAssociation(
                                *group_list_id,
                                *subject_id,
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
                let new_desc =
                    vec![collomatique_state_colloscopes::periods::WeekDesc::new(true); *week_count];
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            match data
                                .get_data()
                                .get_inner_data()
                                .params
                                .periods
                                .ordered_period_list
                                .last()
                            {
                                Some((id, _)) => {
                                    collomatique_state_colloscopes::PeriodOp::AddAfter(
                                        *id, new_desc,
                                    )
                                }
                                None => {
                                    collomatique_state_colloscopes::PeriodOp::AddFront(new_desc)
                                }
                            },
                        ),
                        self.get_desc(),
                    )
                    .expect("Adding a period should never fail");
                match result {
                    Some(collomatique_state_colloscopes::NewId::PeriodId(id)) => Ok(Some(id)),
                    _ => panic!("Unexpected result! {:?}", result),
                }
            }
            GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period_id, week_count) => {
                let pos = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .ok_or(UpdatePeriodWeekCountError::InvalidPeriodId(*period_id))?;
                let mut desc = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[pos]
                    .1
                    .clone();

                desc.resize(
                    *week_count,
                    desc.last()
                        .cloned()
                        .unwrap_or(collomatique_state_colloscopes::periods::WeekDesc::new(true)),
                );

                let result = match data.apply(
                    collomatique_state_colloscopes::Op::Period(
                        collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                    ),
                    self.get_desc(),
                ) {
                    Ok(r) => r,
                    Err(collomatique_state_colloscopes::Error::Period(
                        collomatique_state_colloscopes::PeriodError::InvalidPeriodId(_),
                    )) => {
                        panic!(
                                "Period Id {:?} should be valid at this point but InvalidPeriodId received", *period_id
                            )
                    }
                    Err(e) => {
                        panic!("Unexpected error for UpdatePeriodWeekCount! {:?}", e);
                    }
                };
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
                Ok(None)
            }
            GeneralPlanningUpdateOp::DeletePeriod(period_id) => {
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
                let pos = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .ok_or(CutPeriodError::InvalidPeriodId(*period_id))?;
                let mut desc = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[pos]
                    .1
                    .clone();

                if *new_week_count > desc.len() {
                    Err(CutPeriodError::RemainingWeekCountTooBig(
                        *new_week_count,
                        desc.len(),
                    ))?;
                }

                let (saved_week_patterns, saved_colloscope_period) =
                    self.save_then_clean_end_of_period(data, *period_id, *new_week_count)?;

                let new_desc = desc.split_off(*new_week_count);

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::AddAfter(
                                *period_id, new_desc,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("At this point, period id should be valid");
                let new_id = match result {
                    Some(collomatique_state_colloscopes::NewId::PeriodId(id)) => id,
                    _ => panic!("Unexpected result! {:?}", result),
                };

                let ordered_subject_list = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .subjects
                    .ordered_subject_list
                    .clone();
                for (subject_id, subject) in &ordered_subject_list {
                    if subject.excluded_periods.contains(period_id) {
                        let mut new_subject = subject.clone();
                        new_subject.excluded_periods.insert(new_id.clone());
                        let result = data
                            .apply(
                                collomatique_state_colloscopes::Op::Subject(
                                    collomatique_state_colloscopes::SubjectOp::Update(
                                        *subject_id,
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

                let rule_map = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .rules
                    .rule_map
                    .clone();
                for (rule_id, rule) in &rule_map {
                    if rule.excluded_periods.contains(period_id) {
                        let mut new_rule = rule.clone();
                        new_rule.excluded_periods.insert(new_id.clone());
                        let result = data
                            .apply(
                                collomatique_state_colloscopes::Op::Rule(
                                    collomatique_state_colloscopes::RuleOp::Update(
                                        *rule_id, new_rule,
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
                for (student_id, student) in &student_map {
                    if student.excluded_periods.contains(period_id) {
                        let mut new_student = student.clone();
                        new_student.excluded_periods.insert(new_id.clone());
                        let result = data
                            .apply(
                                collomatique_state_colloscopes::Op::Student(
                                    collomatique_state_colloscopes::StudentOp::Update(
                                        *student_id,
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

                let period_assignments = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .assignments
                    .period_map
                    .get(period_id)
                    .expect("Period id should be valid at this point")
                    .clone();

                for (subject_id, assigned_students) in period_assignments.subject_map {
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

                if let Some(subject_map) = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .group_lists
                    .subjects_associations
                    .get(period_id)
                    .cloned()
                {
                    for (subject_id, group_list_id) in subject_map {
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
                }

                // Shorten the first period
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
                        ),
                        self.get_desc(),
                    )
                    .expect("At this point, period id should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                self.restore_end_of_period(
                    data,
                    new_id,
                    0,
                    *new_week_count,
                    saved_week_patterns,
                    saved_colloscope_period,
                )?;

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

                let (saved_week_patterns, saved_colloscope_period) =
                    self.save_then_clean_end_of_period(data, *period_id, 0)?;

                let previous_id = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[pos - 1]
                    .0;

                let mut prev_desc = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[pos - 1]
                    .1
                    .clone();
                let old_previous_week_count = prev_desc.len();
                let desc = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[pos]
                    .1
                    .clone();

                prev_desc.extend(desc);

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(
                                previous_id,
                                prev_desc,
                            ),
                        ),
                        (
                            OpCategory::GeneralPlanning,
                            "Prolongement d'une période".to_string(),
                        ),
                    )
                    .expect("At this point, period id should be valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                let rec_result =
                    UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::DeletePeriod(*period_id))
                        .rec_apply_no_session(data)
                        .expect("All data should be valid at this point");

                let result = rec_result.new_id;

                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }

                self.restore_end_of_period(
                    data,
                    previous_id,
                    old_previous_week_count,
                    0,
                    saved_week_patterns,
                    saved_colloscope_period,
                )?;

                Ok(None)
            }
            GeneralPlanningUpdateOp::UpdateWeekStatus(period_id, week_num, state) => {
                let pos = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .ok_or(UpdateWeekStatusError::InvalidPeriodId(*period_id))?;
                let mut desc = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[pos]
                    .1
                    .clone();

                if *week_num >= desc.len() {
                    Err(UpdateWeekStatusError::InvalidWeekNumber(
                        *week_num,
                        desc.len(),
                    ))?;
                }

                desc[*week_num].interrogations = *state;

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
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
                let pos = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .ok_or(UpdateWeekAnnotationError::InvalidPeriodId(*period_id))?;
                let mut desc = data
                    .get_data()
                    .get_inner_data()
                    .params
                    .periods
                    .ordered_period_list[pos]
                    .1
                    .clone();

                if *week_num >= desc.len() {
                    Err(UpdateWeekAnnotationError::InvalidWeekNumber(
                        *week_num,
                        desc.len(),
                    ))?;
                }

                desc[*week_num].annotation = annotation.clone();

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Period(
                            collomatique_state_colloscopes::PeriodOp::Update(*period_id, desc),
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

    fn save_then_clean_end_of_period<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        data: &mut T,
        period_id: collomatique_state_colloscopes::PeriodId,
        first_week_to_clean: usize,
    ) -> Result<
        (
            std::collections::BTreeMap<
                collomatique_state_colloscopes::WeekPatternId,
                collomatique_state_colloscopes::week_patterns::WeekPattern,
            >,
            collomatique_state_colloscopes::colloscopes::ColloscopePeriod,
        ),
        GeneralPlanningUpdateError,
    > {
        let saved_week_patterns = data
            .get_data()
            .get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .clone();
        let saved_colloscope_period = data
            .get_data()
            .get_inner_data()
            .colloscope
            .period_map
            .get(&period_id)
            .expect("Period ID should be valid at this point")
            .clone();

        let (first_week, week_count) = data
            .get_data()
            .get_inner_data()
            .params
            .periods
            .get_first_week_and_length_for_period(period_id)
            .expect("Period ID should be valid at this point");
        if week_count <= first_week_to_clean {
            // Nothing to clean
            return Ok((saved_week_patterns, saved_colloscope_period));
        }

        // Clean the colloscope for the end of the period
        if !saved_colloscope_period.is_empty() {
            for (slot_id, collo_slot) in &saved_colloscope_period.slot_map {
                for week in first_week_to_clean..collo_slot.interrogations.len() {
                    let interrogation_opt = &collo_slot.interrogations[week];
                    let Some(interrogation) = interrogation_opt else {
                        continue;
                    };
                    if interrogation.is_empty() {
                        continue;
                    }
                    let result = data
                        .apply(
                            collomatique_state_colloscopes::Op::Colloscope(
                                collomatique_state_colloscopes::ColloscopeOp::UpdateInterrogation(
                                    period_id, *slot_id, week,
                                    collomatique_state_colloscopes::colloscopes::ColloscopeInterrogation::default(),
                                ),
                            ),
                            self.get_desc(),
                        )
                        .expect("At this point, all IDS should be valid");
                    assert!(result.is_none());
                }
            }
        }

        let first_week_to_remove = first_week + first_week_to_clean;
        let weeks_to_remove = week_count - first_week_to_clean;

        // Clean the week patterns for the end of the period
        for (week_pattern_id, week_pattern) in &saved_week_patterns {
            if !week_pattern.can_remove_weeks(first_week_to_remove, weeks_to_remove) {
                let mut new_week_patten = week_pattern.clone();
                new_week_patten.clean_weeks(first_week_to_remove, weeks_to_remove);

                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::WeekPattern(
                            collomatique_state_colloscopes::WeekPatternOp::Update(
                                *week_pattern_id,
                                new_week_patten,
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("At this point, all data should be valid");
                assert!(result.is_none());
            }
        }

        Ok((saved_week_patterns, saved_colloscope_period))
    }

    fn restore_end_of_period<T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>>(
        &self,
        data: &mut T,
        period_id: collomatique_state_colloscopes::PeriodId,
        first_week_in_new_period: usize,
        first_week_in_old_period: usize,
        saved_week_patterns: std::collections::BTreeMap<
            collomatique_state_colloscopes::WeekPatternId,
            collomatique_state_colloscopes::week_patterns::WeekPattern,
        >,
        saved_colloscope_period: collomatique_state_colloscopes::colloscopes::ColloscopePeriod,
    ) -> Result<(), GeneralPlanningUpdateError> {
        // Restore week patterns
        for (week_pattern_id, week_pattern) in saved_week_patterns {
            let result = data
                .apply(
                    collomatique_state_colloscopes::Op::WeekPattern(
                        collomatique_state_colloscopes::WeekPatternOp::Update(
                            week_pattern_id,
                            week_pattern,
                        ),
                    ),
                    self.get_desc(),
                )
                .expect("Week patterns should all be perfectly valid");
            if result.is_some() {
                panic!("Unexpected result! {:?}", result);
            }
        }

        // Restore colloscope
        for (slot_id, collo_slot) in saved_colloscope_period.slot_map {
            for old_week in first_week_in_old_period..collo_slot.interrogations.len() {
                let interrogiation_opt = &collo_slot.interrogations[old_week];
                let Some(interrogation) = interrogiation_opt else {
                    continue;
                };
                if interrogation.is_empty() {
                    continue;
                }
                let new_week = old_week - first_week_in_old_period + first_week_in_new_period;
                let result = data
                    .apply(
                        collomatique_state_colloscopes::Op::Colloscope(
                            collomatique_state_colloscopes::ColloscopeOp::UpdateInterrogation(
                                period_id,
                                slot_id,
                                new_week,
                                interrogation.clone(),
                            ),
                        ),
                        self.get_desc(),
                    )
                    .expect("Interrogations should all be perfectly valid");
                if result.is_some() {
                    panic!("Unexpected result! {:?}", result);
                }
            }
        }

        Ok(())
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
