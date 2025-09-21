//! Colloscopes submodule
//!
//! This module defines the relevant types to describes the colloscopes

use std::collections::{BTreeMap, BTreeSet};

use crate::ids::{
    ColloscopeGroupListId, ColloscopeId, ColloscopeIncompatId, ColloscopePeriodId,
    ColloscopeRuleId, ColloscopeSlotId, ColloscopeStudentId, ColloscopeSubjectId,
    ColloscopeTeacherId, ColloscopeWeekPatternId,
};
use crate::ids::{
    GroupListId, IncompatId, PeriodId, RuleId, SlotId, StudentId, SubjectId, TeacherId,
    WeekPatternId,
};

/// Description of the colloscopes
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Colloscopes {
    /// List of colloscopes
    ///
    /// Each item associates an id to a colloscope description
    pub colloscope_map: BTreeMap<ColloscopeId, Colloscope>,
}

/// Description of a single colloscope
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colloscope {
    /// Name for the colloscope
    pub name: String,
    /// Parameters at the time of the colloscope creation
    pub params: super::colloscope_params::ColloscopeParameters<
        ColloscopePeriodId,
        ColloscopeSubjectId,
        ColloscopeTeacherId,
        ColloscopeStudentId,
        ColloscopeWeekPatternId,
        ColloscopeSlotId,
        ColloscopeIncompatId,
        ColloscopeGroupListId,
        ColloscopeRuleId,
    >,
    /// ids map between the old ids and the one in the parameters copy
    pub id_maps: super::colloscope_params::ColloscopeIdMaps<
        PeriodId,
        SubjectId,
        TeacherId,
        StudentId,
        WeekPatternId,
        SlotId,
        IncompatId,
        GroupListId,
        RuleId,
    >,
    /// Actual colloscope data
    pub data: ColloscopeData,
}

impl Colloscope {
    pub(crate) fn check_invariants(
        &self,
        main_params: &super::colloscope_params::ColloscopeParameters<
            PeriodId,
            SubjectId,
            TeacherId,
            StudentId,
            WeekPatternId,
            SlotId,
            IncompatId,
            GroupListId,
            RuleId,
        >,
    ) -> Result<(), super::ColloscopeError> {
        self.id_maps.validate_source_ids(main_params)?;
        self.id_maps.validate_new_ids(&self.params)?;
        self.params.check_invariants()?;
        self.data.validate_against_params(&self.params)?;

        Ok(())
    }
}

/// Description of actual colloscope data
///
/// the ids should be valid with respect to the corresponding params
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeData {
    pub period_map: BTreeMap<ColloscopePeriodId, ColloscopePeriod>,
    pub group_lists: BTreeMap<ColloscopeGroupListId, ColloscopeGroupList>,
}

impl ColloscopeData {
    pub fn new_empty_from_params(
        _params: &super::colloscope_params::ColloscopeParameters<
            ColloscopePeriodId,
            ColloscopeSubjectId,
            ColloscopeTeacherId,
            ColloscopeStudentId,
            ColloscopeWeekPatternId,
            ColloscopeSlotId,
            ColloscopeIncompatId,
            ColloscopeGroupListId,
            ColloscopeRuleId,
        >,
    ) -> Self {
        todo!()
    }

    pub(crate) fn validate_against_params(
        &self,
        params: &super::colloscope_params::ColloscopeParameters<
            ColloscopePeriodId,
            ColloscopeSubjectId,
            ColloscopeTeacherId,
            ColloscopeStudentId,
            ColloscopeWeekPatternId,
            ColloscopeSlotId,
            ColloscopeIncompatId,
            ColloscopeGroupListId,
            ColloscopeRuleId,
        >,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        if self.period_map.len() != params.periods.ordered_period_list.len() {
            return Err(ColloscopeError::WrongPeriodCountInColloscopeData);
        }

        for (period_id, period) in &self.period_map {
            if params.periods.find_period(*period_id).is_none() {
                return Err(ColloscopeError::InvalidColloscopePeriodId(*period_id));
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
                return Err(ColloscopeError::InvalidColloscopeGroupListId(
                    *group_list_id,
                ));
            }
            group_list.validate_against_params(*group_list_id, params)?;
        }

        Ok(())
    }
}

/// Description of a single period in a colloscope
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopePeriod {
    /// Map between subjects and interrogations for these subjects
    ///
    /// Only subjects with interrogations are represented here
    pub subject_map: BTreeMap<ColloscopeSubjectId, ColloscopeSubject>,
}

impl ColloscopePeriod {
    pub(crate) fn validate_against_params(
        &self,
        period_id: ColloscopePeriodId,
        params: &super::colloscope_params::ColloscopeParameters<
            ColloscopePeriodId,
            ColloscopeSubjectId,
            ColloscopeTeacherId,
            ColloscopeStudentId,
            ColloscopeWeekPatternId,
            ColloscopeSlotId,
            ColloscopeIncompatId,
            ColloscopeGroupListId,
            ColloscopeRuleId,
        >,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let Some(period_assignments) = params.assignments.period_map.get(&period_id) else {
            return Err(ColloscopeError::InvalidColloscopePeriodId(period_id));
        };
        if period_assignments.subject_map.len() != self.subject_map.len() {
            return Err(ColloscopeError::WrongSubjectCountInPeriodInColloscopeData(
                period_id,
            ));
        }

        for (subject_id, subject) in &self.subject_map {
            let Some(param_subject) = params.subjects.find_subject(*subject_id) else {
                return Err(ColloscopeError::InvalidColloscopeSubjectId(*subject_id));
            };

            if param_subject.excluded_periods.contains(&period_id) {
                return Err(ColloscopeError::InvalidColloscopeSubjectId(*subject_id));
            }

            if param_subject.parameters.interrogation_parameters.is_none() {
                return Err(ColloscopeError::InvalidColloscopeSubjectId(*subject_id));
            }

            subject.validate_against_params(period_id, *subject_id, params)?;
        }

        Ok(())
    }
}

/// Description of a single subject in a period in a colloscope
#[derive(Clone, Debug, PartialEq, Eq)]
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
    pub slots: BTreeMap<ColloscopeSlotId, Vec<Option<ColloscopeInterrogation>>>,
}

impl ColloscopeSubject {
    pub(crate) fn validate_against_params(
        &self,
        period_id: ColloscopePeriodId,
        subject_id: ColloscopeSubjectId,
        params: &super::colloscope_params::ColloscopeParameters<
            ColloscopePeriodId,
            ColloscopeSubjectId,
            ColloscopeTeacherId,
            ColloscopeStudentId,
            ColloscopeWeekPatternId,
            ColloscopeSlotId,
            ColloscopeIncompatId,
            ColloscopeGroupListId,
            ColloscopeRuleId,
        >,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let Some(period_pos) = params.periods.find_period_position(period_id) else {
            return Err(ColloscopeError::InvalidColloscopePeriodId(period_id));
        };
        let period = &params.periods.ordered_period_list[period_pos].1;

        let first_week_num: usize = (0..period_pos)
            .into_iter()
            .map(|i| params.periods.ordered_period_list[i].1.len())
            .sum();

        let Some(subject_slots) = params.slots.subject_map.get(&subject_id) else {
            return Err(ColloscopeError::InvalidColloscopeSubjectId(subject_id));
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
                return Err(ColloscopeError::InvalidColloscopeSlotId(*slot_id));
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
                        .ok_or(ColloscopeError::InvalidColloscopeWeekPatternId(*id))?,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeInterrogation {
    /// List of groups assigned to the interrogation
    pub assigned_groups: BTreeSet<u32>,
}

impl ColloscopeInterrogation {
    pub(crate) fn validate_against_params(
        &self,
        period_id: ColloscopePeriodId,
        subject_id: ColloscopeSubjectId,
        slot_id: ColloscopeSlotId,
        week: usize,
        params: &super::colloscope_params::ColloscopeParameters<
            ColloscopePeriodId,
            ColloscopeSubjectId,
            ColloscopeTeacherId,
            ColloscopeStudentId,
            ColloscopeWeekPatternId,
            ColloscopeSlotId,
            ColloscopeIncompatId,
            ColloscopeGroupListId,
            ColloscopeRuleId,
        >,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let Some(subject_association) = params.group_lists.subjects_associations.get(&period_id)
        else {
            return Err(ColloscopeError::InvalidColloscopePeriodId(period_id));
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ColloscopeGroupList {
    pub groups_for_students: BTreeMap<ColloscopeStudentId, Option<u32>>,
}

impl ColloscopeGroupList {
    pub(crate) fn validate_against_params(
        &self,
        group_list_id: ColloscopeGroupListId,
        params: &super::colloscope_params::ColloscopeParameters<
            ColloscopePeriodId,
            ColloscopeSubjectId,
            ColloscopeTeacherId,
            ColloscopeStudentId,
            ColloscopeWeekPatternId,
            ColloscopeSlotId,
            ColloscopeIncompatId,
            ColloscopeGroupListId,
            ColloscopeRuleId,
        >,
    ) -> Result<(), super::ColloscopeError> {
        use super::ColloscopeError;

        let Some(group_list) = params.group_lists.group_list_map.get(&group_list_id) else {
            return Err(ColloscopeError::InvalidColloscopeGroupListId(group_list_id));
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
                return Err(ColloscopeError::InvalidColloscopeStudentId(*student_id));
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
