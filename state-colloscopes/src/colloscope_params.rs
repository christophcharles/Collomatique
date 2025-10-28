//! Colloscope parameters submodule
//!
//! This module defines the relevant types to describes the full set of parameters for colloscopes

use crate::ids::{
    ColloscopeGroupListId, ColloscopeIncompatId, ColloscopePeriodId, ColloscopeRuleId,
    ColloscopeSlotId, ColloscopeStudentId, ColloscopeSubjectId, ColloscopeTeacherId,
    ColloscopeWeekPatternId,
};

use super::*;

use serde::{Deserialize, Serialize};

/// Full set of parameters to describe the constraints for colloscopes
///
/// This structure contains all the parameters we might want to adjust
/// to define the constraints for a colloscope.
///
/// This structure is used in two ways:
/// - a main version is used in [InnerData] to represent the currently edited parameters
/// - another version is used for each colloscope to store the parameters used for its generation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameters<
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
    pub settings: settings::Settings<StudentId>,
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
    > Default
    for Parameters<
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
    fn default() -> Self {
        Parameters {
            periods: periods::Periods::default(),
            subjects: subjects::Subjects::default(),
            teachers: teachers::Teachers::default(),
            students: students::Students::default(),
            assignments: assignments::Assignments::default(),
            week_patterns: week_patterns::WeekPatterns::default(),
            slots: slots::Slots::default(),
            incompats: incompats::Incompats::default(),
            group_lists: group_lists::GroupLists::default(),
            rules: rules::Rules::default(),
            settings: settings::Settings::default(),
        }
    }
}

pub type GeneralParameters = Parameters<
    PeriodId,
    SubjectId,
    TeacherId,
    StudentId,
    WeekPatternId,
    SlotId,
    IncompatId,
    GroupListId,
    RuleId,
>;
pub type ColloscopeParameters = Parameters<
    ColloscopePeriodId,
    ColloscopeSubjectId,
    ColloscopeTeacherId,
    ColloscopeStudentId,
    ColloscopeWeekPatternId,
    ColloscopeSlotId,
    ColloscopeIncompatId,
    ColloscopeGroupListId,
    ColloscopeRuleId,
>;

/// Maps between global ids and colloscope specific ids
///
/// Params for a specific colloscope are stored when the colloscope is produced (even if empty).
/// To avoid issues with ids, the parameter set is given new ids (with new types to avoid some programming errors).
/// But it is useful to know to what ids the new ids correspond. This stores the map between the old ids and the new ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColloscopeIdMaps<
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
    pub periods: BTreeMap<PeriodId, ColloscopePeriodId>,
    pub subjects: BTreeMap<SubjectId, ColloscopeSubjectId>,
    pub teachers: BTreeMap<TeacherId, ColloscopeTeacherId>,
    pub students: BTreeMap<StudentId, ColloscopeStudentId>,
    pub week_patterns: BTreeMap<WeekPatternId, ColloscopeWeekPatternId>,
    pub slots: BTreeMap<SlotId, ColloscopeSlotId>,
    pub incompats: BTreeMap<IncompatId, ColloscopeIncompatId>,
    pub group_lists: BTreeMap<GroupListId, ColloscopeGroupListId>,
    pub rules: BTreeMap<RuleId, ColloscopeRuleId>,
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
    ColloscopeIdMaps<
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
    /// generate a complete set of new ids for some parameters
    /// and returns the complete map for it
    pub(crate) fn generate_for(
        params: &Parameters<
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
        id_issuer: &mut ids::IdIssuer,
    ) -> Self {
        ColloscopeIdMaps {
            periods: params
                .periods
                .ordered_period_list
                .iter()
                .map(|(period_id, _)| (*period_id, id_issuer.get_colloscope_period_id()))
                .collect(),
            subjects: params
                .subjects
                .ordered_subject_list
                .iter()
                .map(|(subject_id, _)| (*subject_id, id_issuer.get_colloscope_subject_id()))
                .collect(),
            teachers: params
                .teachers
                .teacher_map
                .iter()
                .map(|(teacher_id, _)| (*teacher_id, id_issuer.get_colloscope_teacher_id()))
                .collect(),
            students: params
                .students
                .student_map
                .iter()
                .map(|(student_id, _)| (*student_id, id_issuer.get_colloscope_student_id()))
                .collect(),
            week_patterns: params
                .week_patterns
                .week_pattern_map
                .iter()
                .map(|(week_pattern_id, _)| {
                    (*week_pattern_id, id_issuer.get_colloscope_week_pattern_id())
                })
                .collect(),
            slots: params
                .slots
                .subject_map
                .iter()
                .flat_map(|(_subject_id, subject_slots)| {
                    subject_slots
                        .ordered_slots
                        .iter()
                        .map(|(slot_id, _)| *slot_id)
                })
                .map(|id| (id, id_issuer.get_colloscope_slot_id()))
                .collect(),
            incompats: params
                .incompats
                .incompat_map
                .iter()
                .map(|(incompat_id, _)| (*incompat_id, id_issuer.get_colloscope_incompat_id()))
                .collect(),
            group_lists: params
                .group_lists
                .group_list_map
                .iter()
                .map(|(group_list_id, _)| {
                    (*group_list_id, id_issuer.get_colloscope_group_list_id())
                })
                .collect(),
            rules: params
                .rules
                .rule_map
                .iter()
                .map(|(rule_id, _)| (*rule_id, id_issuer.get_colloscope_rule_id()))
                .collect(),
        }
    }

    pub(crate) fn duplicate_with_id_maps(
        &self,
        collo_id_maps: &ColloscopeIdMaps<
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
    ) -> Option<Self> {
        Some(ColloscopeIdMaps {
            periods: self
                .periods
                .iter()
                .map(|(id, collo_id)| {
                    Some((id.clone(), collo_id_maps.periods.get(collo_id).cloned()?))
                })
                .collect::<Option<_>>()?,
            subjects: self
                .subjects
                .iter()
                .map(|(id, collo_id)| {
                    Some((id.clone(), collo_id_maps.subjects.get(collo_id).cloned()?))
                })
                .collect::<Option<_>>()?,
            teachers: self
                .teachers
                .iter()
                .map(|(id, collo_id)| {
                    Some((id.clone(), collo_id_maps.teachers.get(collo_id).cloned()?))
                })
                .collect::<Option<_>>()?,
            students: self
                .students
                .iter()
                .map(|(id, collo_id)| {
                    Some((id.clone(), collo_id_maps.students.get(collo_id).cloned()?))
                })
                .collect::<Option<_>>()?,
            week_patterns: self
                .week_patterns
                .iter()
                .map(|(id, collo_id)| {
                    Some((
                        id.clone(),
                        collo_id_maps.week_patterns.get(collo_id).cloned()?,
                    ))
                })
                .collect::<Option<_>>()?,
            slots: self
                .slots
                .iter()
                .map(|(id, collo_id)| {
                    Some((id.clone(), collo_id_maps.slots.get(collo_id).cloned()?))
                })
                .collect::<Option<_>>()?,
            incompats: self
                .incompats
                .iter()
                .map(|(id, collo_id)| {
                    Some((id.clone(), collo_id_maps.incompats.get(collo_id).cloned()?))
                })
                .collect::<Option<_>>()?,
            group_lists: self
                .group_lists
                .iter()
                .map(|(id, collo_id)| {
                    Some((
                        id.clone(),
                        collo_id_maps.group_lists.get(collo_id).cloned()?,
                    ))
                })
                .collect::<Option<_>>()?,
            rules: self
                .rules
                .iter()
                .map(|(id, collo_id)| {
                    Some((id.clone(), collo_id_maps.rules.get(collo_id).cloned()?))
                })
                .collect::<Option<_>>()?,
        })
    }
}

impl
    ColloscopeIdMaps<
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
    /// Validate original ids against a parameter set
    ///
    /// This functions checks that all the original ids do indeed appear
    pub(crate) fn validate_source_ids(
        &self,
        params: &Parameters<
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
    ) -> Result<(), ColloscopeError> {
        for (period_id, _) in &self.periods {
            if params.periods.find_period(*period_id).is_none() {
                return Err(ColloscopeError::InvalidPeriodId(*period_id));
            }
        }

        for (subject_id, _) in &self.subjects {
            if params.subjects.find_subject(*subject_id).is_none() {
                return Err(ColloscopeError::InvalidSubjectId(*subject_id));
            }
        }

        for (teacher_id, _) in &self.teachers {
            if !params.teachers.teacher_map.contains_key(teacher_id) {
                return Err(ColloscopeError::InvalidTeacherId(*teacher_id));
            }
        }

        for (student_id, _) in &self.students {
            if !params.students.student_map.contains_key(student_id) {
                return Err(ColloscopeError::InvalidStudentId(*student_id));
            }
        }

        for (week_pattern_id, _) in &self.week_patterns {
            if !params
                .week_patterns
                .week_pattern_map
                .contains_key(week_pattern_id)
            {
                return Err(ColloscopeError::InvalidWeekPatternId(*week_pattern_id));
            }
        }

        for (slot_id, _) in &self.slots {
            if params.slots.find_slot(*slot_id).is_none() {
                return Err(ColloscopeError::InvalidSlotId(*slot_id));
            }
        }

        for (incompat_id, _) in &self.incompats {
            if !params.incompats.incompat_map.contains_key(incompat_id) {
                return Err(ColloscopeError::InvalidIncompatId(*incompat_id));
            }
        }

        for (group_list_id, _) in &self.group_lists {
            if !params
                .group_lists
                .group_list_map
                .contains_key(group_list_id)
            {
                return Err(ColloscopeError::InvalidGroupListId(*group_list_id));
            }
        }

        for (rule_id, _) in &self.rules {
            if !params.rules.rule_map.contains_key(rule_id) {
                return Err(ColloscopeError::InvalidRuleId(*rule_id));
            }
        }

        Ok(())
    }

    /// Validate original ids against a parameter set
    ///
    /// This functions checks that all the original ids do indeed appear
    pub(crate) fn validate_new_ids(
        &self,
        params: &Parameters<
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
    ) -> Result<(), ColloscopeError> {
        for (_, period_id) in &self.periods {
            if params.periods.find_period(*period_id).is_none() {
                return Err(ColloscopeError::InvalidColloscopePeriodId(*period_id));
            }
        }

        for (_, subject_id) in &self.subjects {
            if params.subjects.find_subject(*subject_id).is_none() {
                return Err(ColloscopeError::InvalidColloscopeSubjectId(*subject_id));
            }
        }

        for (_, teacher_id) in &self.teachers {
            if !params.teachers.teacher_map.contains_key(teacher_id) {
                return Err(ColloscopeError::InvalidColloscopeTeacherId(*teacher_id));
            }
        }

        for (_, student_id) in &self.students {
            if !params.students.student_map.contains_key(student_id) {
                return Err(ColloscopeError::InvalidColloscopeStudentId(*student_id));
            }
        }

        for (_, week_pattern_id) in &self.week_patterns {
            if !params
                .week_patterns
                .week_pattern_map
                .contains_key(week_pattern_id)
            {
                return Err(ColloscopeError::InvalidColloscopeWeekPatternId(
                    *week_pattern_id,
                ));
            }
        }

        for (_, slot_id) in &self.slots {
            if params.slots.find_slot(*slot_id).is_none() {
                return Err(ColloscopeError::InvalidColloscopeSlotId(*slot_id));
            }
        }

        for (_, incompat_id) in &self.incompats {
            if !params.incompats.incompat_map.contains_key(incompat_id) {
                return Err(ColloscopeError::InvalidColloscopeIncompatId(*incompat_id));
            }
        }

        for (_, group_list_id) in &self.group_lists {
            if !params
                .group_lists
                .group_list_map
                .contains_key(group_list_id)
            {
                return Err(ColloscopeError::InvalidColloscopeGroupListId(
                    *group_list_id,
                ));
            }
        }

        for (_, rule_id) in &self.rules {
            if !params.rules.rule_map.contains_key(rule_id) {
                return Err(ColloscopeError::InvalidColloscopeRuleId(*rule_id));
            }
        }

        Ok(())
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
    Parameters<
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
    pub(crate) fn duplicate_with_id_maps(
        &self,
        id_maps: &ColloscopeIdMaps<
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
    ) -> Option<
        Parameters<
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
    > {
        let periods = self.periods.duplicate_with_id_maps(&id_maps.periods)?;
        let subjects = self
            .subjects
            .duplicate_with_id_maps(&id_maps.periods, &id_maps.subjects)?;
        let teachers = self
            .teachers
            .duplicate_with_id_maps(&id_maps.teachers, &id_maps.subjects)?;
        let students = self
            .students
            .duplicate_with_id_maps(&id_maps.students, &id_maps.periods)?;
        let assignments = self.assignments.duplicate_with_id_maps(
            &id_maps.periods,
            &id_maps.subjects,
            &id_maps.students,
        )?;
        let week_patterns = self
            .week_patterns
            .duplicate_with_id_maps(&id_maps.week_patterns)?;
        let slots = self.slots.duplicate_with_id_maps(
            &id_maps.subjects,
            &id_maps.slots,
            &id_maps.teachers,
            &id_maps.week_patterns,
        )?;
        let incompats = self.incompats.duplicate_with_id_maps(
            &id_maps.incompats,
            &id_maps.subjects,
            &id_maps.week_patterns,
        )?;
        let group_lists = self.group_lists.duplicate_with_id_maps(
            &id_maps.group_lists,
            &id_maps.periods,
            &id_maps.subjects,
            &id_maps.students,
        )?;
        let rules =
            self.rules
                .duplicate_with_id_maps(&id_maps.rules, &id_maps.periods, &id_maps.slots)?;
        let settings = self.settings.duplicate_with_id_maps(&id_maps.students)?;

        Some(Parameters {
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
            settings,
        })
    }

    pub(crate) fn duplicate(
        &self,
        id_issuer: &mut ids::IdIssuer,
    ) -> (
        Parameters<
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
        ColloscopeIdMaps<
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
    ) {
        let id_maps = ColloscopeIdMaps::generate_for(self, id_issuer);
        let new_params = self
            .duplicate_with_id_maps(&id_maps)
            .expect("The id maps should be complete for this specific parameters set");

        (new_params, id_maps)
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
    Parameters<
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
    Parameters<
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
    /// USED INTERNALLY
    ///
    /// Returns an iterator on all ids that appear in the colloscope params
    pub(crate) fn ids(&self) -> impl Iterator<Item = u64> {
        let student_ids = self.students.student_map.keys().map(|x| x.inner());
        let period_ids = self
            .periods
            .ordered_period_list
            .iter()
            .map(|(id, _d)| id.inner());
        let subject_ids = self
            .subjects
            .ordered_subject_list
            .iter()
            .map(|(id, _d)| id.inner());
        let teacher_ids = self.teachers.teacher_map.keys().map(|x| x.inner());
        let week_patterns_ids = self
            .week_patterns
            .week_pattern_map
            .keys()
            .map(|x| x.inner());
        let slot_ids = self
            .slots
            .subject_map
            .iter()
            .flat_map(|(_subject_id, subject_slots)| {
                subject_slots
                    .ordered_slots
                    .iter()
                    .map(|(id, _d)| id.inner())
            });
        let incompat_ids = self.incompats.incompat_map.keys().map(|x| x.inner());
        let group_list_ids = self.group_lists.group_list_map.keys().map(|x| x.inner());
        let rule_ids = self.rules.rule_map.keys().map(|x| x.inner());

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

    /// USED INTERNALLY
    ///
    /// Checks that a subject is valid
    fn validate_subject_internal(
        subject: &subjects::Subject<PeriodId>,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), SubjectError<SubjectId, PeriodId, TeacherId, IncompatId, GroupListId>> {
        for period_id in &subject.excluded_periods {
            if !period_ids.contains(period_id) {
                return Err(SubjectError::InvalidPeriodId(*period_id));
            }
        }

        let Some(interrogation_parameters) = &subject.parameters.interrogation_parameters else {
            return Ok(());
        };

        if interrogation_parameters.students_per_group.is_empty() {
            return Err(SubjectError::StudentsPerGroupRangeIsEmpty);
        }
        if interrogation_parameters.groups_per_interrogation.is_empty() {
            return Err(SubjectError::GroupsPerInterrogationRangeIsEmpty);
        }

        match &interrogation_parameters.periodicity {
            SubjectPeriodicity::AmountForEveryArbitraryBlock {
                blocks,
                minimum_week_separation: _,
            } => {
                for block in blocks {
                    if block.interrogation_count_in_block.is_empty() {
                        return Err(SubjectError::InterrogationCountRangeIsEmpty);
                    }
                }
            }
            SubjectPeriodicity::AmountInYear {
                interrogation_count_in_year,
                minimum_week_separation: _,
            } => {
                if interrogation_count_in_year.is_empty() {
                    return Err(SubjectError::InterrogationCountRangeIsEmpty);
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a subject before commiting a subject op
    pub(crate) fn validate_subject(
        &self,
        subject: &subjects::Subject<PeriodId>,
    ) -> Result<(), SubjectError<SubjectId, PeriodId, TeacherId, IncompatId, GroupListId>> {
        let period_ids = self.build_period_ids();

        Self::validate_subject_internal(subject, &period_ids)
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in subject data
    fn check_subjects_data_consistency(
        &self,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), InvariantError> {
        for (_subject_id, subject) in &self.subjects.ordered_subject_list {
            if Self::validate_subject_internal(subject, period_ids).is_err() {
                return Err(InvariantError::InvalidSubject);
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that a subject is valid
    fn validate_teacher_internal(
        teacher: &teachers::Teacher<SubjectId>,
        subjects: &subjects::Subjects<SubjectId, PeriodId>,
    ) -> Result<(), TeacherError<TeacherId, SubjectId, SlotId>> {
        for subject_id in &teacher.subjects {
            let Some(subject) = subjects.find_subject(*subject_id) else {
                return Err(TeacherError::InvalidSubjectId(*subject_id));
            };
            if subject.parameters.interrogation_parameters.is_none() {
                return Err(TeacherError::SubjectHasNoInterrogation(*subject_id));
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a teacher before commiting a teacher op
    pub(crate) fn validate_teacher(
        &self,
        teacher: &teachers::Teacher<SubjectId>,
    ) -> Result<(), TeacherError<TeacherId, SubjectId, SlotId>> {
        Self::validate_teacher_internal(teacher, &self.subjects)
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in subject data
    fn check_teachers_data_consistency(&self) -> Result<(), InvariantError> {
        for (_teacher_id, teacher) in &self.teachers.teacher_map {
            if Self::validate_teacher_internal(teacher, &self.subjects).is_err() {
                return Err(InvariantError::InvalidTeacher);
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that a subject is valid
    fn validate_student_internal(
        student: &students::Student<PeriodId>,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), StudentError<StudentId, PeriodId, SubjectId, GroupListId>> {
        for period_id in &student.excluded_periods {
            if !period_ids.contains(period_id) {
                return Err(StudentError::InvalidPeriodId(*period_id));
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a teacher before commiting a teacher op
    pub(crate) fn validate_student(
        &self,
        student: &students::Student<PeriodId>,
    ) -> Result<(), StudentError<StudentId, PeriodId, SubjectId, GroupListId>> {
        let period_ids = self.build_period_ids();

        Self::validate_student_internal(student, &period_ids)
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in subject data
    fn check_students_data_consistency(
        &self,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), InvariantError> {
        for (_student_id, student) in &self.students.student_map {
            if Self::validate_student_internal(student, period_ids).is_err() {
                return Err(InvariantError::InvalidStudent);
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in assignments data
    fn check_assignments_data_consistency(
        &self,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), InvariantError> {
        assert!(self.assignments.period_map.len() == period_ids.len());
        for (period_id, period_assignments) in &self.assignments.period_map {
            if !period_ids.contains(period_id) {
                return Err(InvariantError::InvalidPeriodIdInAssignements);
            }

            let mut subject_count_for_period = 0usize;
            for (subject_id, subject) in &self.subjects.ordered_subject_list {
                if subject.excluded_periods.contains(period_id) {
                    continue;
                }
                subject_count_for_period += 1;

                let subject_assignments = period_assignments
                    .subject_map
                    .get(subject_id)
                    .ok_or(InvariantError::InvalidSubjectIdInAssignments)?;

                for student_id in subject_assignments {
                    let student = self
                        .students
                        .student_map
                        .get(student_id)
                        .ok_or(InvariantError::InvalidStudentIdInAssignments)?;

                    if student.excluded_periods.contains(period_id) {
                        return Err(InvariantError::AssignedStudentNotPresentForPeriod);
                    }
                }
            }
            if subject_count_for_period != period_assignments.subject_map.len() {
                return Err(InvariantError::WrongSubjectCountInAssignments);
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that a slot is valid
    fn validate_slot_internal(
        slot: &slots::Slot<TeacherId, WeekPatternId>,
        subject_id: SubjectId,
        week_pattern_ids: &BTreeSet<WeekPatternId>,
        teachers: &teachers::Teachers<TeacherId, SubjectId>,
        subjects: &subjects::Subjects<SubjectId, PeriodId>,
    ) -> Result<(), SlotError<SlotId, SubjectId, TeacherId, WeekPatternId, RuleId>> {
        let Some(teacher) = teachers.teacher_map.get(&slot.teacher_id) else {
            return Err(SlotError::InvalidTeacherId(slot.teacher_id));
        };
        if !teacher.subjects.contains(&subject_id) {
            return Err(SlotError::TeacherDoesNotTeachInSubject(
                slot.teacher_id,
                subject_id,
            ));
        }
        if let Some(week_pattern_id) = &slot.week_pattern {
            if !week_pattern_ids.contains(week_pattern_id) {
                return Err(SlotError::InvalidWeekPatternId(*week_pattern_id));
            }
        }
        let Some(subject) = subjects.find_subject(subject_id) else {
            return Err(SlotError::InvalidSubjectId(subject_id));
        };
        let Some(params) = &subject.parameters.interrogation_parameters else {
            return Err(SlotError::SubjectHasNoInterrogation(subject_id));
        };
        if collomatique_time::SlotWithDuration::new(
            slot.start_time.clone(),
            params.duration.clone(),
        )
        .is_none()
        {
            return Err(SlotError::SlotOverlapsWithNextDay);
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a teacher before commiting a teacher op
    pub(crate) fn validate_slot(
        &self,
        slot: &slots::Slot<TeacherId, WeekPatternId>,
        subject_id: SubjectId,
    ) -> Result<(), SlotError<SlotId, SubjectId, TeacherId, WeekPatternId, RuleId>> {
        let week_pattern_ids = self.build_week_pattern_ids();

        Self::validate_slot_internal(
            slot,
            subject_id,
            &week_pattern_ids,
            &self.teachers,
            &self.subjects,
        )
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in assignments data
    fn check_slots_data_consistency(
        &self,
        week_pattern_ids: &BTreeSet<WeekPatternId>,
    ) -> Result<(), InvariantError> {
        let subjects_with_interrogations_count = self
            .subjects
            .ordered_subject_list
            .iter()
            .filter(|(_id, subject)| subject.parameters.interrogation_parameters.is_some())
            .count();
        if self.slots.subject_map.len() != subjects_with_interrogations_count {
            return Err(InvariantError::WrongSubjectCountInSlots);
        }

        for (subject_id, subject_slots) in &self.slots.subject_map {
            for (_slot_id, slot) in &subject_slots.ordered_slots {
                if Self::validate_slot_internal(
                    slot,
                    *subject_id,
                    week_pattern_ids,
                    &self.teachers,
                    &self.subjects,
                )
                .is_err()
                {
                    return Err(InvariantError::InvalidSlot);
                }
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that an incompat is valid
    fn validate_incompat_internal(
        incompat: &incompats::Incompatibility<SubjectId, WeekPatternId>,
        week_pattern_ids: &BTreeSet<WeekPatternId>,
        subject_ids: &BTreeSet<SubjectId>,
    ) -> Result<(), IncompatError<IncompatId, SubjectId, WeekPatternId>> {
        if !subject_ids.contains(&incompat.subject_id) {
            return Err(IncompatError::InvalidSubjectId(incompat.subject_id));
        }
        if let Some(week_pattern_id) = &incompat.week_pattern_id {
            if !week_pattern_ids.contains(week_pattern_id) {
                return Err(IncompatError::InvalidWeekPatternId(*week_pattern_id));
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a teacher before commiting a teacher op
    pub(crate) fn validate_incompat(
        &self,
        incompat: &incompats::Incompatibility<SubjectId, WeekPatternId>,
    ) -> Result<(), IncompatError<IncompatId, SubjectId, WeekPatternId>> {
        let week_pattern_ids = self.build_week_pattern_ids();
        let subject_ids = self.build_subject_ids();

        Self::validate_incompat_internal(incompat, &week_pattern_ids, &subject_ids)
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in assignments data
    fn check_incompats_data_consistency(
        &self,
        week_pattern_ids: &BTreeSet<WeekPatternId>,
        subject_ids: &BTreeSet<SubjectId>,
    ) -> Result<(), InvariantError> {
        for (_incompat_id, incompat) in &self.incompats.incompat_map {
            if Self::validate_incompat_internal(incompat, week_pattern_ids, subject_ids).is_err() {
                return Err(InvariantError::InvalidIncompat);
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that an incompat is valid
    fn validate_group_list_params_internal(
        params: &group_lists::GroupListParameters<StudentId>,
        students: &students::Students<StudentId, PeriodId>,
    ) -> Result<(), GroupListError<GroupListId, StudentId, SubjectId, PeriodId>> {
        if params.group_count.is_empty() {
            return Err(GroupListError::GroupCountRangeIsEmpty);
        }
        if params.students_per_group.is_empty() {
            return Err(GroupListError::StudentsPerGroupRangeIsEmpty);
        }
        for student_id in &params.excluded_students {
            if !students.student_map.contains_key(student_id) {
                return Err(GroupListError::InvalidStudentId(*student_id));
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that an incompat is valid
    fn validate_group_list_prefilled_groups_internal(
        prefilled_groups: &group_lists::GroupListPrefilledGroups<StudentId>,
        students: &students::Students<StudentId, PeriodId>,
        excluded_students: &BTreeSet<StudentId>,
    ) -> Result<(), GroupListError<GroupListId, StudentId, SubjectId, PeriodId>> {
        if !prefilled_groups.check_duplicated_student() {
            return Err(GroupListError::DuplicatedStudentInPrefilledGroups);
        }
        for group in &prefilled_groups.groups {
            for student_id in &group.students {
                if !students.student_map.contains_key(student_id) {
                    return Err(GroupListError::InvalidStudentId(*student_id));
                }
                if excluded_students.contains(student_id) {
                    return Err(GroupListError::StudentBothIncludedAndExcluded(*student_id));
                }
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that an incompat is valid
    fn validate_group_list_internal(
        group_list: &group_lists::GroupList<StudentId>,
        students: &students::Students<StudentId, PeriodId>,
    ) -> Result<(), GroupListError<GroupListId, StudentId, SubjectId, PeriodId>> {
        Self::validate_group_list_params_internal(&group_list.params, students)?;
        Self::validate_group_list_prefilled_groups_internal(
            &group_list.prefilled_groups,
            students,
            &group_list.params.excluded_students,
        )?;
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a teacher before commiting a teacher op
    pub(crate) fn validate_group_list(
        &self,
        group_list: &group_lists::GroupList<StudentId>,
    ) -> Result<(), GroupListError<GroupListId, StudentId, SubjectId, PeriodId>> {
        Self::validate_group_list_internal(group_list, &self.students)
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in assignments data
    fn check_group_lists_data_consistency(&self) -> Result<(), InvariantError> {
        if self.group_lists.subjects_associations.len() != self.periods.ordered_period_list.len() {
            return Err(InvariantError::WrongPeriodCountInSubjectAssociationsForGroupLists);
        }
        for (period_id, subject_map) in &self.group_lists.subjects_associations {
            for (subject_id, group_list_id) in subject_map {
                if !self.group_lists.group_list_map.contains_key(group_list_id) {
                    return Err(InvariantError::InvalidGroupListIdInSubjectAssociations);
                }
                let subject = self
                    .subjects
                    .find_subject(*subject_id)
                    .ok_or(InvariantError::InvalidSubjectIdInSubjectAssociations)?;

                if subject.parameters.interrogation_parameters.is_none() {
                    return Err(InvariantError::SubjectAssociationForSubjectWithoutInterrogations);
                };
                if subject.excluded_periods.contains(period_id) {
                    return Err(InvariantError::SubjectAssociationForSubjectNotRunningOnPeriod);
                }
            }
        }
        for (_group_list_id, group_list) in &self.group_lists.group_list_map {
            if Self::validate_group_list_internal(group_list, &self.students).is_err() {
                return Err(InvariantError::InvalidGroupList);
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that a rule is valid
    fn validate_logic_rule_internal(
        logic_rule: &rules::LogicRule<SlotId>,
        slot_ids: &BTreeSet<SlotId>,
    ) -> Result<(), RuleError<RuleId, PeriodId, SlotId>> {
        match logic_rule {
            rules::LogicRule::And(l1, l2) => {
                Self::validate_logic_rule_internal(l1.as_ref(), slot_ids)?;
                Self::validate_logic_rule_internal(l2.as_ref(), slot_ids)?;
            }
            rules::LogicRule::Or(l1, l2) => {
                Self::validate_logic_rule_internal(l1.as_ref(), slot_ids)?;
                Self::validate_logic_rule_internal(l2.as_ref(), slot_ids)?;
            }
            rules::LogicRule::Not(l) => {
                Self::validate_logic_rule_internal(l.as_ref(), slot_ids)?;
            }
            rules::LogicRule::Variable(slot_id) => {
                if !slot_ids.contains(slot_id) {
                    return Err(RuleError::InvalidSlotId(*slot_id));
                }
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that a rule is valid
    fn validate_rule_internal(
        rule: &rules::Rule<PeriodId, SlotId>,
        period_ids: &BTreeSet<PeriodId>,
        slot_ids: &BTreeSet<SlotId>,
    ) -> Result<(), RuleError<RuleId, PeriodId, SlotId>> {
        for period_id in &rule.excluded_periods {
            if !period_ids.contains(period_id) {
                return Err(RuleError::InvalidPeriodId(*period_id));
            }
        }

        Self::validate_logic_rule_internal(&rule.desc, slot_ids)?;

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a rule before commiting a rule op
    pub(crate) fn validate_rule(
        &self,
        rule: &rules::Rule<PeriodId, SlotId>,
    ) -> Result<(), RuleError<RuleId, PeriodId, SlotId>> {
        let period_ids = self.build_period_ids();
        let slot_ids = self.build_slot_ids();
        Self::validate_rule_internal(rule, &period_ids, &slot_ids)
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in rules data
    fn check_rules_data_consistency(
        &self,
        period_ids: &BTreeSet<PeriodId>,
        slot_ids: &BTreeSet<SlotId>,
    ) -> Result<(), InvariantError> {
        for (_rule_id, rule) in &self.rules.rule_map {
            if Self::validate_rule_internal(rule, period_ids, slot_ids).is_err() {
                return Err(InvariantError::InvalidRule);
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check settings before commiting a settings op
    pub(crate) fn validate_settings(
        &self,
        settings: &settings::Settings<StudentId>,
    ) -> Result<(), SettingsError<StudentId>> {
        for (student_id, _limits) in &settings.students {
            if !self.students.student_map.contains_key(student_id) {
                return Err(SettingsError::InvalidStudentId(*student_id));
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in rules data
    fn check_settings_data_consistency(&self) -> Result<(), InvariantError> {
        match self.validate_settings(&self.settings) {
            Ok(()) => Ok(()),
            Err(SettingsError::InvalidStudentId(_id)) => {
                Err(InvariantError::InvalidStudentIdInSettings)
            }
        }
    }

    /// USED INTERNALLY
    ///
    /// Build the set of PeriodIds
    ///
    /// This is useful to check that references are valid
    fn build_period_ids(&self) -> BTreeSet<PeriodId> {
        let mut ids = BTreeSet::new();
        for (id, _) in &self.periods.ordered_period_list {
            ids.insert(*id);
        }
        ids
    }

    /// USED INTERNALLY
    ///
    /// Build the set of WeekPatternId
    ///
    /// This is useful to check that references are valid
    fn build_week_pattern_ids(&self) -> BTreeSet<WeekPatternId> {
        self.week_patterns
            .week_pattern_map
            .keys()
            .copied()
            .collect()
    }

    /// USED INTERNALLY
    ///
    /// Build the set of SubjectId
    ///
    /// This is useful to check that references are valid
    fn build_subject_ids(&self) -> BTreeSet<SubjectId> {
        self.subjects
            .ordered_subject_list
            .iter()
            .map(|(id, _)| *id)
            .collect()
    }

    /// USED INTERNALLY
    ///
    /// Build the set of SlotId
    ///
    /// This is useful to check that references are valid
    fn build_slot_ids(&self) -> BTreeSet<SlotId> {
        self.slots
            .subject_map
            .iter()
            .flat_map(|(_subject_id, subject_slots)| {
                subject_slots.ordered_slots.iter().map(|(id, _)| *id)
            })
            .collect()
    }

    /// USED INTERNALLY
    ///
    /// Checks that there are no duplicate ids in this specific colloscope params
    fn check_no_duplicate_ids(&self) -> bool {
        let mut ids_so_far = BTreeSet::new();

        for id in self.ids() {
            if !ids_so_far.insert(id) {
                return false;
            }
        }

        true
    }

    /// USED INTERNALLY
    ///
    /// Checks all the invariants of data
    pub fn check_invariants(&self) -> Result<(), InvariantError> {
        if !self.check_no_duplicate_ids() {
            return Err(InvariantError::DuplicatedId);
        }

        let period_ids = self.build_period_ids();
        let week_pattern_ids = self.build_week_pattern_ids();
        let subject_ids = self.build_subject_ids();
        let slot_ids = self.build_slot_ids();

        self.check_subjects_data_consistency(&period_ids)?;
        self.check_teachers_data_consistency()?;
        self.check_students_data_consistency(&period_ids)?;
        self.check_assignments_data_consistency(&period_ids)?;
        self.check_slots_data_consistency(&week_pattern_ids)?;
        self.check_incompats_data_consistency(&week_pattern_ids, &subject_ids)?;
        self.check_group_lists_data_consistency()?;
        self.check_rules_data_consistency(&period_ids, &slot_ids)?;
        self.check_settings_data_consistency()?;

        Ok(())
    }
}
