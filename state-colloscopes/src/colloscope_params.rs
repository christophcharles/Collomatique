//! Colloscope parameters submodule
//!
//! This module defines the relevant types to describes the full set of parameters for colloscopes

use super::*;

/// Full set of parameters to describe the constraints for colloscopes
///
/// This structure contains all the parameters we might want to adjust
/// to define the constraints for a colloscope.
///
/// This structure is used in two ways:
/// - a main version is used in [InnerData] to represent the currently edited parameters
/// - another version is used for each colloscope to store the parameters used for its generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColloscopeParameters<
    PeriodId: Id,
    SubjectId: Id,
    TeacherId: Id,
    StudentId: Id,
    WeekPatternId: Id,
    SlotId: Id,
    IncompatId: Id,
    GroupListId: Id,
    RuleId: Id,
> {
    pub periods: periods::Periods<PeriodId>,
    pub subjects: subjects::Subjects<SubjectId, PeriodId>,
    pub teachers: teachers::Teachers<TeacherId, SubjectId>,
    pub students: students::Students<StudentId, PeriodId>,
    pub assignments: assignments::Assignments<PeriodId, SubjectId, StudentId>,
    pub week_patterns: week_patterns::WeekPatterns<WeekPatternId>,
    pub slots: slots::Slots<SubjectId, SlotId, TeacherId, WeekPatternId>,
    pub incompats: incompats::Incompats<IncompatId, SubjectId, WeekPatternId>,
    pub group_lists: group_lists::GroupLists<GroupListId, PeriodId, SubjectId, StudentId>,
    pub rules: rules::Rules<RuleId, PeriodId, SlotId>,
    pub settings: settings::GeneralSettings,
}

impl<
        PeriodId: Id,
        SubjectId: Id,
        TeacherId: Id,
        StudentId: Id,
        WeekPatternId: Id,
        SlotId: Id,
        IncompatId: Id,
        GroupListId: Id,
        RuleId: Id,
    >
    ColloscopeParameters<
        PeriodId,
        SubjectId,
        TeacherId,
        StudentId,
        WeekPatternId,
        SlotId,
        IncompatId,
        GroupListId,
        RuleId,
    >
{
    pub unsafe fn from_external_data(external_data: ColloscopeParametersExternalData) -> Self {
        let students = unsafe { Students::from_external_data(external_data.students) };
        let periods = unsafe { Periods::from_external_data(external_data.periods) };
        let subjects = unsafe { Subjects::from_external_data(external_data.subjects) };
        let teachers = unsafe { Teachers::from_external_data(external_data.teachers) };
        let assignments = unsafe { Assignments::from_external_data(external_data.assignments) };
        let week_patterns =
            unsafe { WeekPatterns::from_external_data(external_data.week_patterns) };
        let slots = unsafe { Slots::from_external_data(external_data.slots) };
        let incompats = unsafe { Incompats::from_external_data(external_data.incompats) };
        let group_lists = unsafe { GroupLists::from_external_data(external_data.group_lists) };
        let rules = unsafe { Rules::from_external_data(external_data.rules) };

        colloscope_params::ColloscopeParameters {
            periods,
            subjects,
            teachers,
            students,
            assignments,
            week_patterns,
            slots,
            incompats,
            group_lists,
            rules,
            settings: external_data.settings,
        }
    }
}

/// External data version of [ColloscopeParameters]
///
/// This is equivalent to [ColloscopeParameters] but with unchecked ids
/// This is useful when loading from file for instance
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ColloscopeParametersExternalData {
    pub periods: periods::PeriodsExternalData,
    pub subjects: subjects::SubjectsExternalData,
    pub teachers: teachers::TeachersExternalData,
    pub students: students::StudentsExternalData,
    pub assignments: assignments::AssignmentsExternalData,
    pub week_patterns: week_patterns::WeekPatternsExternalData,
    pub slots: slots::SlotsExternalData,
    pub incompats: incompats::IncompatsExternalData,
    pub group_lists: group_lists::GroupListsExternalData,
    pub rules: rules::RulesExternalData,
    pub settings: settings::GeneralSettings,
}

impl ColloscopeParametersExternalData {
    pub fn validate(&self) -> Result<(), FromDataError> {
        let period_ids: std::collections::BTreeSet<_> = self
            .periods
            .ordered_period_list
            .iter()
            .map(|(id, _d)| *id)
            .collect();
        let week_pattern_ids: std::collections::BTreeSet<_> = self
            .week_patterns
            .week_pattern_map
            .keys()
            .copied()
            .collect();
        let subject_ids: std::collections::BTreeSet<_> = self
            .subjects
            .ordered_subject_list
            .iter()
            .map(|(id, _)| *id)
            .collect();
        let student_ids = self.students.student_map.keys().copied().collect();
        let slot_ids = self
            .slots
            .subject_map
            .iter()
            .flat_map(|(_subject_id, subject_slots)| {
                subject_slots.ordered_slots.iter().map(|(id, _)| *id)
            })
            .collect();
        if !self.subjects.validate_all(&period_ids) {
            return Err(tools::IdError::InvalidId.into());
        }
        if !self.teachers.validate_all(&self.subjects) {
            return Err(tools::IdError::InvalidId.into());
        }
        if !self.students.validate_all(&period_ids) {
            return Err(tools::IdError::InvalidId.into());
        }
        if !self
            .assignments
            .validate_all(&period_ids, &self.students, &self.subjects)
        {
            return Err(FromDataError::InconsistentAssignments);
        }
        if !self
            .slots
            .validate_all(&self.subjects, &week_pattern_ids, &self.teachers)
        {
            return Err(FromDataError::InconsistentSlots);
        }
        if !self.incompats.validate_all(&subject_ids, &week_pattern_ids) {
            return Err(tools::IdError::InvalidId.into());
        }
        if !self
            .group_lists
            .validate_all(&self.subjects, &student_ids, &period_ids)
        {
            return Err(FromDataError::InconsistentGroupLists);
        }
        if !self.rules.validate_all(&period_ids, &slot_ids) {
            return Err(FromDataError::InconsistentRules);
        }
        Ok(())
    }

    pub fn ids(&self) -> impl Iterator<Item = u64> {
        let student_ids = self.students.student_map.keys().copied();
        let period_ids = self.periods.ordered_period_list.iter().map(|(id, _d)| *id);
        let subject_ids = self
            .subjects
            .ordered_subject_list
            .iter()
            .map(|(id, _d)| *id);
        let teacher_ids = self.teachers.teacher_map.keys().copied();
        let week_patterns_ids = self.week_patterns.week_pattern_map.keys().copied();
        let slot_ids = self
            .slots
            .subject_map
            .iter()
            .flat_map(|(_subject_id, subject_slots)| {
                subject_slots.ordered_slots.iter().map(|(id, _d)| *id)
            });
        let incompat_ids = self.incompats.incompat_map.keys().copied();
        let group_list_ids = self.group_lists.group_list_map.keys().copied();
        let rule_ids = self.rules.rule_map.keys().copied();

        let existing_ids = student_ids
            .chain(period_ids)
            .chain(subject_ids)
            .chain(teacher_ids)
            .chain(week_patterns_ids)
            .chain(slot_ids)
            .chain(incompat_ids)
            .chain(group_list_ids)
            .chain(rule_ids);

        existing_ids
    }
}
