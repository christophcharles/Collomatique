//! Colloscopes submodule
//!
//! This module defines the relevant types to describes the colloscopes

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{GroupListId, PeriodId, SlotId, StudentId, SubjectId};

/// Description of a colloscope
///
/// the ids should be valid with respect to the corresponding params
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Colloscope {
    pub period_map: BTreeMap<PeriodId, ColloscopePeriod>,
    pub group_lists: BTreeMap<GroupListId, ColloscopeGroupList>,
}

impl Colloscope {
    /// Builds an empty colloscope compatible with the given parameters
    ///
    /// The function might panic if the parameters do not satisfy parameters invariants
    /// You should check this before hand with [super::colloscope_params::Parameters::check_invariants].
    pub fn new_empty_from_params(params: &super::colloscope_params::Parameters) -> Self {
        let group_lists = params
            .group_lists
            .group_list_map
            .iter()
            .map(|(group_list_id, group_list)| {
                let collo_group_list = ColloscopeGroupList {
                    groups_for_students: params
                        .students
                        .student_map
                        .iter()
                        .filter_map(|(student_id, _student)| {
                            if group_list.params.excluded_students.contains(student_id) {
                                return None;
                            }

                            for (i, group) in group_list.prefilled_groups.groups.iter().enumerate()
                            {
                                if group.students.contains(student_id) {
                                    return Some((*student_id, Some(i as u32)));
                                }
                            }

                            Some((*student_id, None))
                        })
                        .collect(),
                };

                (*group_list_id, collo_group_list)
            })
            .collect();

        let mut period_map = BTreeMap::new();
        let mut first_week = 0usize;
        for (period_id, period) in &params.periods.ordered_period_list {
            let mut subject_map = BTreeMap::new();

            for (subject_id, subject) in &params.subjects.ordered_subject_list {
                if subject.excluded_periods.contains(period_id) {
                    continue;
                }
                if subject.parameters.interrogation_parameters.is_none() {
                    continue;
                }

                let orig_slots = params
                    .slots
                    .subject_map
                    .get(subject_id)
                    .expect("Subject ID should be valid");

                let mut slots = BTreeMap::new();

                for (slot_id, slot) in &orig_slots.ordered_slots {
                    let mut collo_slot = vec![];

                    let week_pattern_id_opt = &slot.week_pattern;
                    let week_pattern_opt = match week_pattern_id_opt {
                        None => None,
                        Some(id) => Some(
                            params
                                .week_patterns
                                .week_pattern_map
                                .get(id)
                                .expect("Week pattern id should be valid"),
                        ),
                    };
                    for i in 0..period.len() {
                        let current_week = first_week + i;
                        let is_week_active = match week_pattern_opt {
                            None => true,
                            Some(week_pattern) => {
                                if current_week >= week_pattern.weeks.len() {
                                    true
                                } else {
                                    week_pattern.weeks[current_week]
                                }
                            }
                        };
                        collo_slot.push(if is_week_active {
                            Some(ColloscopeInterrogation {
                                assigned_groups: BTreeSet::new(),
                            })
                        } else {
                            None
                        });
                    }

                    slots.insert(*slot_id, collo_slot);
                }

                let collo_subject = ColloscopeSubject { slots };

                subject_map.insert(*subject_id, collo_subject);
            }

            let collo_period = ColloscopePeriod { subject_map };

            period_map.insert(*period_id, collo_period);
            first_week += period.len();
        }

        Colloscope {
            period_map,
            group_lists,
        }
    }
}

impl Colloscope {
    pub(crate) fn validate_against_params(
        &self,
        params: &super::colloscope_params::Parameters,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        if self.period_map.len() != params.periods.ordered_period_list.len() {
            return Err(ColloscopeError::WrongPeriodCountInColloscopeData);
        }

        for (period_id, period) in &self.period_map {
            if params.periods.find_period(*period_id).is_none() {
                return Err(ColloscopeError::InvalidPeriodId(*period_id));
            }
            period.validate_against_params(*period_id, params)?;
        }

        if self.group_lists.len() != params.group_lists.group_list_map.len() {
            return Err(ColloscopeError::WrongPeriodCountInColloscopeData);
        }

        for (group_list_id, group_list) in &self.group_lists {
            if !params
                .group_lists
                .group_list_map
                .contains_key(group_list_id)
            {
                return Err(ColloscopeError::InvalidGroupListId(*group_list_id));
            }
            group_list.validate_against_params(*group_list_id, params)?;
        }

        Ok(())
    }
}

/// Description of a single period in a colloscope
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColloscopePeriod {
    /// Map between subjects and interrogations for these subjects
    ///
    /// Only subjects with interrogations are represented here
    pub subject_map: BTreeMap<SubjectId, ColloscopeSubject>,
}

impl ColloscopePeriod {
    pub(crate) fn validate_against_params(
        &self,
        period_id: PeriodId,
        params: &super::colloscope_params::Parameters,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let mut subject_with_interrogation_count = 0usize;

        for (_subject_id, subject) in &params.subjects.ordered_subject_list {
            if subject.excluded_periods.contains(&period_id) {
                continue;
            }
            if subject.parameters.interrogation_parameters.is_none() {
                continue;
            }
            subject_with_interrogation_count += 1;
        }

        if self.subject_map.len() != subject_with_interrogation_count {
            return Err(ColloscopeError::WrongSubjectCountInPeriodInColloscopeData(
                period_id,
            ));
        }

        for (subject_id, subject) in &self.subject_map {
            let Some(param_subject) = params.subjects.find_subject(*subject_id) else {
                return Err(ColloscopeError::InvalidSubjectId(*subject_id));
            };

            if param_subject.excluded_periods.contains(&period_id) {
                return Err(ColloscopeError::InvalidSubjectId(*subject_id));
            }

            if param_subject.parameters.interrogation_parameters.is_none() {
                return Err(ColloscopeError::InvalidSubjectId(*subject_id));
            }

            subject.validate_against_params(period_id, *subject_id, params)?;
        }

        Ok(())
    }
}

/// Description of a single subject in a period in a colloscope
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColloscopeSubject {
    /// Map between slots and list of interrogations
    ///
    /// Each relevant slot are mapped to vec containing a cell
    /// for each week in the period. If there cannot be an interrogation
    /// there is is still a cell, but the option is set to None.
    ///
    /// If however there is a possible interrogation, it will be a Some
    /// value, even if no group is actually assigned. This will rather
    /// be described within the [ColloscopeInterrogation] struct.
    pub slots: BTreeMap<SlotId, Vec<Option<ColloscopeInterrogation>>>,
}

impl ColloscopeSubject {
    pub(crate) fn validate_against_params(
        &self,
        period_id: PeriodId,
        subject_id: SubjectId,
        params: &super::colloscope_params::Parameters,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let Some(period_pos) = params.periods.find_period_position(period_id) else {
            return Err(ColloscopeError::InvalidPeriodId(period_id));
        };
        let period = &params.periods.ordered_period_list[period_pos].1;

        let first_week_num: usize = (0..period_pos)
            .into_iter()
            .map(|i| params.periods.ordered_period_list[i].1.len())
            .sum();

        let Some(subject_slots) = params.slots.subject_map.get(&subject_id) else {
            return Err(ColloscopeError::InvalidSubjectId(subject_id));
        };

        if subject_slots.ordered_slots.len() != self.slots.len() {
            return Err(
                ColloscopeError::WrongSlotCountForSubjectInPeriodInColloscopeData(
                    period_id, subject_id,
                ),
            );
        }

        for (slot_id, slot) in &self.slots {
            let Some(orig_slot) = subject_slots.find_slot(*slot_id) else {
                return Err(ColloscopeError::InvalidSlotId(*slot_id));
            };

            if period.len() != slot.len() {
                return Err(
                    ColloscopeError::WrongInterrogationCountForSlotInPeriodInColloscopeData(
                        period_id, *slot_id,
                    ),
                );
            }

            let week_pattern = match &orig_slot.week_pattern {
                Some(id) => Some(
                    params
                        .week_patterns
                        .week_pattern_map
                        .get(id)
                        .expect("Week pattern id should be valid"),
                ),
                None => None,
            };

            for (i, interrogation_opt) in slot.iter().enumerate() {
                let current_week = first_week_num + i;
                let is_week_active = match week_pattern {
                    None => true,
                    Some(week_pattern) => {
                        if current_week >= week_pattern.weeks.len() {
                            true
                        } else {
                            week_pattern.weeks[current_week]
                        }
                    }
                };

                if !is_week_active {
                    if interrogation_opt.is_some() {
                        return Err(ColloscopeError::InterrogationOnNonInterrogationWeek(
                            period_id,
                            *slot_id,
                            current_week,
                        ));
                    }
                    continue;
                }

                let Some(interrogation) = interrogation_opt else {
                    return Err(ColloscopeError::MissingInterrogationOnInterrogationWeek(
                        period_id,
                        *slot_id,
                        current_week,
                    ));
                };

                interrogation.validate_against_params(
                    period_id,
                    subject_id,
                    *slot_id,
                    current_week,
                    params,
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColloscopeInterrogation {
    /// List of groups assigned to the interrogation
    pub assigned_groups: BTreeSet<u32>,
}

impl ColloscopeInterrogation {
    pub(crate) fn validate_against_params(
        &self,
        period_id: PeriodId,
        subject_id: SubjectId,
        slot_id: SlotId,
        week: usize,
        params: &super::colloscope_params::Parameters,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let Some(subject_association) = params.group_lists.subjects_associations.get(&period_id)
        else {
            return Err(ColloscopeError::InvalidPeriodId(period_id));
        };

        let group_list_id_opt = subject_association.get(&subject_id);

        let first_forbidden_value = match group_list_id_opt {
            None => 0u32,
            Some(group_list_id) => {
                let group_list = params
                    .group_lists
                    .group_list_map
                    .get(group_list_id)
                    .expect("Group list id should be valid");

                group_list.params.group_count.end() + 1
            }
        };

        for group_num in &self.assigned_groups {
            if *group_num >= first_forbidden_value {
                return Err(ColloscopeError::InvalidGroupNumInInterrogation(
                    period_id, slot_id, week,
                ));
            }
        }

        Ok(())
    }
}

/// Description of a group list in a colloscope
///
/// This is basically map between students that are in the group lists
/// and actual group numbers
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColloscopeGroupList {
    pub groups_for_students: BTreeMap<StudentId, Option<u32>>,
}

impl ColloscopeGroupList {
    pub(crate) fn validate_against_params(
        &self,
        group_list_id: GroupListId,
        params: &super::colloscope_params::Parameters,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let Some(group_list) = params.group_lists.group_list_map.get(&group_list_id) else {
            return Err(ColloscopeError::InvalidGroupListId(group_list_id));
        };

        let first_forbidden_value = group_list.params.group_count.end() + 1;

        let expected_student_count = params
            .students
            .student_map
            .iter()
            .filter_map(|(student_id, _student)| {
                if group_list.params.excluded_students.contains(student_id) {
                    return None;
                }

                Some(student_id)
            })
            .count();

        if expected_student_count != self.groups_for_students.len() {
            return Err(ColloscopeError::WrongStudentCountInGroupList(group_list_id));
        }

        for (student_id, group_num_opt) in &self.groups_for_students {
            if group_list.params.excluded_students.contains(student_id) {
                return Err(ColloscopeError::ExcludedStudentInGroupList(
                    group_list_id,
                    *student_id,
                ));
            }

            if !params.students.student_map.contains_key(student_id) {
                return Err(ColloscopeError::InvalidStudentId(*student_id));
            }

            if let Some(group_num) = group_num_opt {
                if *group_num >= first_forbidden_value {
                    return Err(ColloscopeError::InvalidGroupNumForStudentInGroupList(
                        group_list_id,
                        *student_id,
                    ));
                }
            }
        }

        Ok(())
    }
}
