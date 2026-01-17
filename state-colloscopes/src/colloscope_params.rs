//! Colloscope parameters submodule
//!
//! This module defines the relevant types to describes the full set of parameters for colloscopes

use crate::ids::{
    GroupListId, IncompatId, PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekPatternId,
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameters {
    pub periods: periods::Periods,
    pub subjects: subjects::Subjects,
    pub teachers: teachers::Teachers,
    pub students: students::Students,
    pub assignments: assignments::Assignments,
    pub week_patterns: week_patterns::WeekPatterns,
    pub slots: slots::Slots,
    pub incompats: incompats::Incompats,
    pub group_lists: group_lists::GroupLists,
    pub settings: settings::Settings,
    pub main_script: Option<String>,
}

impl Parameters {
    pub(crate) fn merge_pattern(&self, pattern: &[bool]) -> Vec<bool> {
        let mut current_week = 0usize;
        let mut output = Vec::new();
        for (_period_id, period_desc) in &self.periods.ordered_period_list {
            for week_desk in period_desc {
                if !week_desk.interrogations {
                    output.push(false);
                } else {
                    output.push(pattern[current_week]);
                }
                current_week += 1;
            }
        }
        output
    }

    pub(crate) fn get_merged_pattern(
        &self,
        week_pattern_id_opt: Option<WeekPatternId>,
    ) -> Vec<bool> {
        let pattern = match week_pattern_id_opt {
            Some(week_pattern_id) => self.week_patterns.get_pattern(week_pattern_id),
            None => {
                vec![true; self.periods.count_weeks()]
            }
        };

        self.merge_pattern(&pattern)
    }
}

impl Parameters {
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
}

impl Parameters {
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

        let existing_ids = student_ids
            .chain(period_ids)
            .chain(subject_ids)
            .chain(teacher_ids)
            .chain(week_patterns_ids)
            .chain(slot_ids)
            .chain(incompat_ids)
            .chain(group_list_ids);

        existing_ids
    }

    /// USED INTERNALLY
    ///
    /// Checks that a subject is valid
    fn validate_subject_internal(
        subject: &subjects::Subject,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), SubjectError> {
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
    pub(crate) fn validate_subject(&self, subject: &subjects::Subject) -> Result<(), SubjectError> {
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
        teacher: &teachers::Teacher,
        subjects: &subjects::Subjects,
    ) -> Result<(), TeacherError> {
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
    pub(crate) fn validate_teacher(&self, teacher: &teachers::Teacher) -> Result<(), TeacherError> {
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
        student: &students::Student,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), StudentError> {
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
    pub(crate) fn validate_student(&self, student: &students::Student) -> Result<(), StudentError> {
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
        slot: &slots::Slot,
        subject_id: SubjectId,
        week_pattern_ids: &BTreeSet<WeekPatternId>,
        teachers: &teachers::Teachers,
        subjects: &subjects::Subjects,
    ) -> Result<(), SlotError> {
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
        slot: &slots::Slot,
        subject_id: SubjectId,
    ) -> Result<(), SlotError> {
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
        incompat: &incompats::Incompatibility,
        week_pattern_ids: &BTreeSet<WeekPatternId>,
        subject_ids: &BTreeSet<SubjectId>,
    ) -> Result<(), IncompatError> {
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
        incompat: &incompats::Incompatibility,
    ) -> Result<(), IncompatError> {
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
        params: &group_lists::GroupListParameters,
        students: &students::Students,
    ) -> Result<(), GroupListError> {
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
        prefilled_groups: &group_lists::GroupListPrefilledGroups,
        students: &students::Students,
        excluded_students: &BTreeSet<StudentId>,
    ) -> Result<(), GroupListError> {
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
        group_list: &group_lists::GroupList,
        students: &students::Students,
    ) -> Result<(), GroupListError> {
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
        group_list: &group_lists::GroupList,
    ) -> Result<(), GroupListError> {
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
    /// used to check settings before commiting a settings op
    pub(crate) fn validate_settings(
        &self,
        settings: &settings::Settings,
    ) -> Result<(), SettingsError> {
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
    /// used to check week patterns
    fn validate_week_pattern_internal(
        week_pattern: &week_patterns::WeekPattern,
        total_week_count: usize,
    ) -> Result<(), WeekPatternError> {
        if week_pattern.weeks.len() != total_week_count {
            return Err(WeekPatternError::BadWeekPatternLength);
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check settings before commiting a settings op
    pub(crate) fn validate_week_pattern(
        &self,
        week_pattern: &week_patterns::WeekPattern,
    ) -> Result<(), WeekPatternError> {
        let total_week_count: usize = self
            .periods
            .ordered_period_list
            .iter()
            .map(|(_period_id, desc)| desc.len())
            .sum();

        Self::validate_week_pattern_internal(week_pattern, total_week_count)
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in rules data
    fn check_week_pattern_data_consistency(
        &self,
        total_week_count: usize,
    ) -> Result<(), InvariantError> {
        for (_week_pattern_id, week_pattern) in &self.week_patterns.week_pattern_map {
            if Self::validate_week_pattern_internal(week_pattern, total_week_count).is_err() {
                return Err(InvariantError::InvalidWeekPattern);
            }
        }
        Ok(())
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
        let total_week_count = self
            .periods
            .ordered_period_list
            .iter()
            .map(|(_period_id, desc)| desc.len())
            .sum();

        self.check_subjects_data_consistency(&period_ids)?;
        self.check_teachers_data_consistency()?;
        self.check_students_data_consistency(&period_ids)?;
        self.check_assignments_data_consistency(&period_ids)?;
        self.check_slots_data_consistency(&week_pattern_ids)?;
        self.check_incompats_data_consistency(&week_pattern_ids, &subject_ids)?;
        self.check_group_lists_data_consistency()?;
        self.check_settings_data_consistency()?;
        self.check_week_pattern_data_consistency(total_week_count)?;

        Ok(())
    }
}
