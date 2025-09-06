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
    /// Promotes an u64 to a [PeriodId] if it is valid
    pub fn validate_period_id(&self, id: u64) -> Option<PeriodId> {
        for (period_id, _) in &self.periods.ordered_period_list {
            if period_id.inner() == id {
                return Some(*period_id);
            }
        }

        None
    }

    /// Promotes an u64 to a [StudentId] if it is valid
    pub fn validate_student_id(&self, id: u64) -> Option<StudentId> {
        let student_id = unsafe { StudentId::new(id) };

        if !self.students.student_map.contains_key(&student_id) {
            return None;
        }

        Some(student_id)
    }

    /// Promotes an u64 to a [SubjectId] if it is valid
    pub fn validate_subject_id(&self, id: u64) -> Option<SubjectId> {
        for (subject_id, _) in &self.subjects.ordered_subject_list {
            if subject_id.inner() == id {
                return Some(*subject_id);
            }
        }

        None
    }

    /// Promotes an u64 to a [TeacherId] if it is valid
    pub fn validate_teacher_id(&self, id: u64) -> Option<TeacherId> {
        let temp_teacher_id = unsafe { TeacherId::new(id) };
        if self.teachers.teacher_map.contains_key(&temp_teacher_id) {
            return Some(temp_teacher_id);
        }

        None
    }

    /// Promotes an u64 to a [WeekPatternId] if it is valid
    pub fn validate_week_pattern_id(&self, id: u64) -> Option<WeekPatternId> {
        let temp_week_pattern_id = unsafe { WeekPatternId::new(id) };
        if self
            .week_patterns
            .week_pattern_map
            .contains_key(&temp_week_pattern_id)
        {
            return Some(temp_week_pattern_id);
        }

        None
    }

    /// Promotes an u64 to a [SlotId] if it is valid
    pub fn validate_slot_id(&self, id: u64) -> Option<SlotId> {
        for (_subject_id, subject_slots) in &self.slots.subject_map {
            for (slot_id, _slot) in &subject_slots.ordered_slots {
                if slot_id.inner() == id {
                    return Some(*slot_id);
                }
            }
        }

        None
    }

    /// Promotes an u64 to a [IncompatId] if it is valid
    pub fn validate_incompat_id(&self, id: u64) -> Option<IncompatId> {
        let temp_incompat_id = unsafe { IncompatId::new(id) };
        if self.incompats.incompat_map.contains_key(&temp_incompat_id) {
            return Some(temp_incompat_id);
        }

        None
    }

    /// Promotes an u64 to a [GroupListId] if it is valid
    pub fn validate_group_list_id(&self, id: u64) -> Option<GroupListId> {
        let temp_group_list_id = unsafe { GroupListId::new(id) };
        if self
            .group_lists
            .group_list_map
            .contains_key(&temp_group_list_id)
        {
            return Some(temp_group_list_id);
        }

        None
    }

    /// Promotes an u64 to a [RuleId] if it is valid
    pub fn validate_rule_id(&self, id: u64) -> Option<RuleId> {
        let temp_rule_id = unsafe { RuleId::new(id) };
        if self.rules.rule_map.contains_key(&temp_rule_id) {
            return Some(temp_rule_id);
        }

        None
    }

    /// Promotes a [teachers::TeacherExternalData] to a [teachers::Teacher] if it is valid
    pub fn promote_teacher(
        &self,
        teacher: teachers::TeacherExternalData,
    ) -> Result<teachers::Teacher<SubjectId>, u64> {
        let mut new_subjects = BTreeSet::new();

        for subject_id in teacher.subjects {
            let Some(validated_id) = self.validate_subject_id(subject_id) else {
                return Err(subject_id);
            };
            new_subjects.insert(validated_id);
        }

        Ok(teachers::Teacher {
            desc: teacher.desc,
            subjects: new_subjects,
        })
    }

    /// Promotes a [students::StudentExternalData] to a [students::Student] if it is valid
    pub fn promote_student(
        &self,
        student: students::StudentExternalData,
    ) -> Result<students::Student<PeriodId>, u64> {
        let mut new_excluded_periods = BTreeSet::new();

        for period_id in student.excluded_periods {
            let Some(validated_id) = self.validate_period_id(period_id) else {
                return Err(period_id);
            };
            new_excluded_periods.insert(validated_id);
        }

        Ok(students::Student {
            desc: student.desc,
            excluded_periods: new_excluded_periods,
        })
    }

    /// Promotes a [slots::SlotExternalData] to a [slots::Slot] if it is valid
    pub fn promote_slot(
        &self,
        slot: slots::SlotExternalData,
    ) -> Result<slots::Slot<TeacherId, WeekPatternId>, PromoteSlotError> {
        let teacher_id = self
            .validate_teacher_id(slot.teacher_id)
            .ok_or(PromoteSlotError::InvalidTeacherId(slot.teacher_id))?;
        let week_pattern = match slot.week_pattern {
            Some(id) => {
                let week_pattern_id = self
                    .validate_week_pattern_id(id)
                    .ok_or(PromoteSlotError::InvalidWeekPatternId(id))?;
                Some(week_pattern_id)
            }
            None => None,
        };
        let new_slot = slots::Slot {
            teacher_id,
            start_time: slot.start_time,
            extra_info: slot.extra_info,
            week_pattern,
            cost: slot.cost,
        };

        Ok(new_slot)
    }

    /// Promotes a [incompats::IncompatibilityExternalData] to a [incompats::Incompatibility] if it is valid
    pub fn promote_incompat(
        &self,
        incompat: incompats::IncompatibilityExternalData,
    ) -> Result<incompats::Incompatibility<SubjectId, WeekPatternId>, PromoteIncompatError> {
        let subject_id = self
            .validate_subject_id(incompat.subject_id)
            .ok_or(PromoteIncompatError::InvalidSubjectId(incompat.subject_id))?;
        let week_pattern_id = match incompat.week_pattern_id {
            Some(id) => {
                let week_pattern_id = self
                    .validate_week_pattern_id(id)
                    .ok_or(PromoteIncompatError::InvalidWeekPatternId(id))?;
                Some(week_pattern_id)
            }
            None => None,
        };

        let new_incompat = incompats::Incompatibility {
            subject_id,
            name: incompat.name,
            week_pattern_id,
            slots: incompat.slots,
            minimum_free_slots: incompat.minimum_free_slots,
        };

        Ok(new_incompat)
    }

    /// Promotes a [group_lists::GroupListParametersExternalData] to a [group_lists::GroupListParameters] if it is valid
    pub fn promote_group_list_params(
        &self,
        params: group_lists::GroupListParametersExternalData,
    ) -> Result<group_lists::GroupListParameters<StudentId>, PromoteGroupListParametersError> {
        let mut excluded_students = BTreeSet::new();

        for student_id in params.excluded_students {
            let Some(new_id) = self.validate_student_id(student_id) else {
                return Err(PromoteGroupListParametersError::InvalidStudentId(
                    student_id,
                ));
            };

            excluded_students.insert(new_id);
        }

        let new_params = group_lists::GroupListParameters {
            name: params.name,
            students_per_group: params.students_per_group,
            group_count: params.group_count,
            excluded_students,
        };

        Ok(new_params)
    }

    /// Promotes a [group_lists::GroupListPrefilledGroupsExternalData] to a [group_lists::GroupListPrefilledGroups] if it is valid
    pub fn promote_group_list_prefilled_groups(
        &self,
        prefilled_groups: group_lists::GroupListPrefilledGroupsExternalData,
    ) -> Result<
        group_lists::GroupListPrefilledGroups<StudentId>,
        PromoteGroupListPrefilledGroupsError,
    > {
        let mut groups = vec![];

        for group in prefilled_groups.groups {
            let mut students = BTreeSet::new();
            for student_id in group.students {
                let Some(new_id) = self.validate_student_id(student_id) else {
                    return Err(PromoteGroupListPrefilledGroupsError::InvalidStudentId(
                        student_id,
                    ));
                };

                students.insert(new_id);
            }
            groups.push(group_lists::PrefilledGroup {
                name: group.name,
                students,
                sealed: group.sealed,
            });
        }

        let new_prefilled_groups = group_lists::GroupListPrefilledGroups { groups };

        Ok(new_prefilled_groups)
    }

    /// Promotes a [rules::LogicRuleExternalData] to a [rules::LogicRule] if it is valid
    pub fn promote_logic_rule(
        &self,
        logic_rule: rules::LogicRuleExternalData,
    ) -> Result<rules::LogicRule<SlotId>, PromoteLogicRuleError> {
        use rules::{LogicRule, LogicRuleExternalData};
        let new_logic_rule = match logic_rule {
            LogicRuleExternalData::And(l1, l2) => LogicRule::And(
                Box::new(self.promote_logic_rule(*l1)?),
                Box::new(self.promote_logic_rule(*l2)?),
            ),
            LogicRuleExternalData::Or(l1, l2) => LogicRule::Or(
                Box::new(self.promote_logic_rule(*l1)?),
                Box::new(self.promote_logic_rule(*l2)?),
            ),
            LogicRuleExternalData::Not(l) => LogicRule::Not(Box::new(self.promote_logic_rule(*l)?)),
            LogicRuleExternalData::Variable(id) => {
                let Some(slot_id) = self.validate_slot_id(id) else {
                    return Err(PromoteLogicRuleError::InvalidSlotId(id));
                };

                LogicRule::Variable(slot_id)
            }
        };
        Ok(new_logic_rule)
    }
}

/// Error type for [Data::promote_slot]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PromoteSlotError {
    #[error("Teacher id {0:?} is invalid")]
    InvalidTeacherId(u64),
    #[error("WeekPattern id {0:?} is invalid")]
    InvalidWeekPatternId(u64),
}

/// Error type for [Data::promote_incompat]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PromoteIncompatError {
    #[error("Subject id {0:?} is invalid")]
    InvalidSubjectId(u64),
    #[error("WeekPattern id {0:?} is invalid")]
    InvalidWeekPatternId(u64),
}

/// Error type for [Data::promote_group_list_params]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PromoteGroupListParametersError {
    #[error("Student id {0:?} is invalid")]
    InvalidStudentId(u64),
}

/// Error type for [Data::promote_group_list_prefilled_groups]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PromoteGroupListPrefilledGroupsError {
    #[error("Student id {0:?} is invalid")]
    InvalidStudentId(u64),
}

/// Error type for [Data::promote_logic_rule]
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PromoteLogicRuleError {
    #[error("Slot id {0:?} is invalid")]
    InvalidSlotId(u64),
}
