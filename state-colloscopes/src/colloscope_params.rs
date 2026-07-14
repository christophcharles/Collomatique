//! Colloscope parameters submodule
//!
//! This module defines the relevant types to describes the full set of parameters for colloscopes

use crate::ids::{
    GroupListId, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekPatternId,
};

use super::*;

use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    #[serde(default)]
    pub pairings: pairings::Pairings,
    #[serde(default)]
    pub slot_pairings: slot_pairings::SlotPairings,
    #[serde(default)]
    pub balancing: balancing::Balancing,
}

impl Parameters {
    pub(crate) fn merge_pattern(&self, pattern: &[bool]) -> Vec<bool> {
        let mut current_week = 0usize;
        let mut output = Vec::new();
        for (_period_id, period_desc) in self.periods.ordered_period_list.entries() {
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
        for (period_id, _) in self.periods.ordered_period_list.entries() {
            if period_id.inner() == id {
                return Some(period_id);
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
        for (subject_id, _) in self.subjects.ordered_subject_list.entries() {
            if subject_id.inner() == id {
                return Some(subject_id);
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
        for subject_slots in self.slots.subject_map.values() {
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
            .entries()
            .map(|(id, _d)| id.inner());
        let subject_ids = self
            .subjects
            .ordered_subject_list
            .entries()
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
        let pairing_rule_ids = self.pairings.pairing_rule_map.keys().map(|x| x.inner());
        let slot_pairing_rule_ids = self
            .slot_pairings
            .slot_pairing_rule_map
            .keys()
            .map(|x| x.inner());

        student_ids
            .chain(period_ids)
            .chain(subject_ids)
            .chain(teacher_ids)
            .chain(week_patterns_ids)
            .chain(slot_ids)
            .chain(incompat_ids)
            .chain(group_list_ids)
            .chain(pairing_rule_ids)
            .chain(slot_pairing_rule_ids)
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
        for (_subject_id, subject) in self.subjects.ordered_subject_list.entries() {
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
        for teacher in self.teachers.teacher_map.values() {
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
        for student in self.students.student_map.values() {
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
            for (subject_id, subject) in self.subjects.ordered_subject_list.entries() {
                if subject.excluded_periods.contains(period_id) {
                    continue;
                }
                subject_count_for_period += 1;

                let subject_assignments = period_assignments
                    .subject_map
                    .get(&subject_id)
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
        if let Some(week_pattern_id) = &slot.week_pattern
            && !week_pattern_ids.contains(week_pattern_id)
        {
            return Err(SlotError::InvalidWeekPatternId(*week_pattern_id));
        }
        let Some(subject) = subjects.find_subject(subject_id) else {
            return Err(SlotError::InvalidSubjectId(subject_id));
        };
        let Some(params) = &subject.parameters.interrogation_parameters else {
            return Err(SlotError::SubjectHasNoInterrogation(subject_id));
        };
        if collomatique_time::SlotWithDuration::new(slot.start_time.clone(), params.duration)
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
            .entries()
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
        if let Some(week_pattern_id) = &incompat.week_pattern_id
            && !week_pattern_ids.contains(week_pattern_id)
        {
            return Err(IncompatError::InvalidWeekPatternId(*week_pattern_id));
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
        for incompat in self.incompats.incompat_map.values() {
            if Self::validate_incompat_internal(incompat, week_pattern_ids, subject_ids).is_err() {
                return Err(InvariantError::InvalidIncompat);
            }
        }

        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that group list parameters are valid
    fn validate_group_list_params_internal(
        params: &group_lists::GroupListParameters,
    ) -> Result<(), GroupListError> {
        if params.students_per_group.is_empty() {
            return Err(GroupListError::StudentsPerGroupRangeIsEmpty);
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that group list filling is valid
    fn validate_group_list_filling_internal(
        filling: &group_lists::GroupListFilling,
        students: &students::Students,
        group_names_len: usize,
    ) -> Result<(), GroupListError> {
        match filling {
            group_lists::GroupListFilling::Prefilled { groups } => {
                if groups.len() != group_names_len {
                    return Err(GroupListError::PrefillGroupCountMismatch {
                        expected: group_names_len,
                        actual: groups.len(),
                    });
                }
                if !filling.check_duplicated_student() {
                    return Err(GroupListError::DuplicatedStudentInPrefilledGroups);
                }
                for group in groups {
                    for student_id in &group.students {
                        if !students.student_map.contains_key(student_id) {
                            return Err(GroupListError::InvalidStudentId(*student_id));
                        }
                    }
                }
            }
            group_lists::GroupListFilling::Automatic { excluded_students } => {
                for student_id in excluded_students {
                    if !students.student_map.contains_key(student_id) {
                        return Err(GroupListError::InvalidStudentId(*student_id));
                    }
                }
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that a group list is valid
    fn validate_group_list_internal(
        group_list: &group_lists::GroupList,
        students: &students::Students,
    ) -> Result<(), GroupListError> {
        Self::validate_group_list_params_internal(&group_list.params)?;
        Self::validate_group_list_filling_internal(
            &group_list.filling,
            students,
            group_list.params.group_names.len(),
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
        for group_list in self.group_lists.group_list_map.values() {
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
        for student_id in settings.students.keys() {
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
    /// used to check balancing before commiting a balancing op
    pub(crate) fn validate_balancing(
        &self,
        balancing: &balancing::Balancing,
    ) -> Result<(), BalancingError> {
        for subject_id in balancing.subjects.keys() {
            let Some(subject) = self.subjects.find_subject(*subject_id) else {
                return Err(BalancingError::InvalidSubjectId(*subject_id));
            };
            if subject.parameters.interrogation_parameters.is_none() {
                return Err(BalancingError::SubjectHasNoInterrogation(*subject_id));
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// Checks that a pairing rule is valid
    fn validate_pairing_rule_internal(
        rule: &pairings::PairingRule,
        subject_ids: &BTreeSet<SubjectId>,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), PairingError> {
        if rule.antecedent.subject_id == rule.consequent.subject_id {
            return Err(PairingError::SameSubjectInBothParts(
                rule.antecedent.subject_id,
            ));
        }
        if !subject_ids.contains(&rule.antecedent.subject_id) {
            return Err(PairingError::InvalidSubjectId(rule.antecedent.subject_id));
        }
        if !subject_ids.contains(&rule.consequent.subject_id) {
            return Err(PairingError::InvalidSubjectId(rule.consequent.subject_id));
        }
        for period_id in &rule.excluded_periods {
            if !period_ids.contains(period_id) {
                return Err(PairingError::InvalidPeriodId(*period_id));
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a pairing rule before commiting a pairing op
    pub(crate) fn validate_pairing_rule(
        &self,
        rule: &pairings::PairingRule,
    ) -> Result<(), PairingError> {
        let subject_ids = self.build_subject_ids();
        let period_ids = self.build_period_ids();

        Self::validate_pairing_rule_internal(rule, &subject_ids, &period_ids)
    }

    /// Promotes an u64 to a [PairingRuleId] if it is valid
    pub fn validate_pairing_rule_id(&self, id: u64) -> Option<PairingRuleId> {
        let temp_id = unsafe { PairingRuleId::new(id) };
        if self.pairings.pairing_rule_map.contains_key(&temp_id) {
            return Some(temp_id);
        }

        None
    }

    /// Builds a map from SlotId to SubjectId for all slots
    fn build_slot_subject_map(&self) -> BTreeMap<SlotId, SubjectId> {
        let mut map = BTreeMap::new();
        for (subject_id, subject_slots) in &self.slots.subject_map {
            for (slot_id, _slot) in &subject_slots.ordered_slots {
                map.insert(*slot_id, *subject_id);
            }
        }
        map
    }

    /// USED INTERNALLY
    ///
    /// Checks that a slot pairing rule is valid
    fn validate_slot_pairing_rule_internal(
        rule: &slot_pairings::SlotPairingRule,
        slot_subject_map: &BTreeMap<SlotId, SubjectId>,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), SlotPairingError> {
        if rule.antecedent.slot_id == rule.consequent.slot_id {
            return Err(SlotPairingError::SameSlotInBothParts(
                rule.antecedent.slot_id,
            ));
        }
        let Some(ant_subject) = slot_subject_map.get(&rule.antecedent.slot_id) else {
            return Err(SlotPairingError::InvalidSlotId(rule.antecedent.slot_id));
        };
        let Some(con_subject) = slot_subject_map.get(&rule.consequent.slot_id) else {
            return Err(SlotPairingError::InvalidSlotId(rule.consequent.slot_id));
        };
        if ant_subject != con_subject {
            return Err(SlotPairingError::SlotsNotInSameSubject(
                rule.antecedent.slot_id,
                rule.consequent.slot_id,
            ));
        }
        for period_id in &rule.excluded_periods {
            if !period_ids.contains(period_id) {
                return Err(SlotPairingError::InvalidPeriodId(*period_id));
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// used to check a slot pairing rule before commiting a slot pairing op
    pub(crate) fn validate_slot_pairing_rule(
        &self,
        rule: &slot_pairings::SlotPairingRule,
    ) -> Result<(), SlotPairingError> {
        let slot_subject_map = self.build_slot_subject_map();
        let period_ids: BTreeSet<PeriodId> = self
            .periods
            .ordered_period_list
            .entries()
            .map(|(id, _)| id)
            .collect();
        Self::validate_slot_pairing_rule_internal(rule, &slot_subject_map, &period_ids)
    }

    /// Promotes an u64 to a [SlotPairingRuleId] if it is valid
    pub fn validate_slot_pairing_rule_id(&self, id: u64) -> Option<SlotPairingRuleId> {
        let id = unsafe { SlotPairingRuleId::new(id) };
        if self.slot_pairings.slot_pairing_rule_map.contains_key(&id) {
            Some(id)
        } else {
            None
        }
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in slot pairings data
    fn check_slot_pairings_data_consistency(
        &self,
        slot_subject_map: &BTreeMap<SlotId, SubjectId>,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), InvariantError> {
        for rule in self.slot_pairings.slot_pairing_rule_map.values() {
            if Self::validate_slot_pairing_rule_internal(rule, slot_subject_map, period_ids)
                .is_err()
            {
                return Err(InvariantError::InvalidSlotPairingRule);
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in pairings data
    fn check_pairings_data_consistency(
        &self,
        subject_ids: &BTreeSet<SubjectId>,
        period_ids: &BTreeSet<PeriodId>,
    ) -> Result<(), InvariantError> {
        for rule in self.pairings.pairing_rule_map.values() {
            if Self::validate_pairing_rule_internal(rule, subject_ids, period_ids).is_err() {
                return Err(InvariantError::InvalidPairingRule);
            }
        }
        Ok(())
    }

    /// USED INTERNALLY
    ///
    /// checks all the invariants in balancing data
    fn check_balancing_data_consistency(&self) -> Result<(), InvariantError> {
        match self.validate_balancing(&self.balancing) {
            Ok(()) => Ok(()),
            Err(BalancingError::InvalidSubjectId(_id)) => {
                Err(InvariantError::InvalidSubjectIdInBalancing)
            }
            Err(BalancingError::SubjectHasNoInterrogation(_id)) => {
                Err(InvariantError::BalancingForSubjectWithoutInterrogations)
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
            .entries()
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
        for week_pattern in self.week_patterns.week_pattern_map.values() {
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
        for (id, _) in self.periods.ordered_period_list.entries() {
            ids.insert(id);
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
            .entries()
            .map(|(id, _)| id)
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
            .entries()
            .map(|(_period_id, desc)| desc.len())
            .sum();

        self.check_subjects_data_consistency(&period_ids)?;
        self.check_teachers_data_consistency()?;
        self.check_students_data_consistency(&period_ids)?;
        self.check_assignments_data_consistency(&period_ids)?;
        self.check_slots_data_consistency(&week_pattern_ids)?;
        self.check_incompats_data_consistency(&week_pattern_ids, &subject_ids)?;
        self.check_pairings_data_consistency(&subject_ids, &period_ids)?;
        let slot_subject_map = self.build_slot_subject_map();
        self.check_slot_pairings_data_consistency(&slot_subject_map, &period_ids)?;
        self.check_group_lists_data_consistency()?;
        self.check_settings_data_consistency()?;
        self.check_balancing_data_consistency()?;
        self.check_week_pattern_data_consistency(total_week_count)?;

        Ok(())
    }
}

/// Invariant violations in [Parameters]
///
/// These errors can be returned when checking the internal
/// consistency of a full set of parameters.
#[derive(Clone, Debug, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum InvariantError {
    #[error("duplicated id")]
    DuplicatedId,
    #[error("invalid subject")]
    InvalidSubject,
    #[error("invalid teacher")]
    InvalidTeacher,
    #[error("invalid student")]
    InvalidStudent,
    #[error("invalid period id in assignments")]
    InvalidPeriodIdInAssignements,
    #[error("invalid subject id in assignments")]
    InvalidSubjectIdInAssignments,
    #[error("invalid student id in assignments")]
    InvalidStudentIdInAssignments,
    #[error("student assigned but not present")]
    AssignedStudentNotPresentForPeriod,
    #[error("wrong number of subjects in a period for assignments")]
    WrongSubjectCountInAssignments,
    #[error("wrong number of subjects in slots")]
    WrongSubjectCountInSlots,
    #[error("invalid slot")]
    InvalidSlot,
    #[error("invalid incompat")]
    InvalidIncompat,
    #[error("invalid group list")]
    InvalidGroupList,
    #[error("wrong number of periods in subject associations for group lists")]
    WrongPeriodCountInSubjectAssociationsForGroupLists,
    #[error("invalid group list id in subject associations")]
    InvalidGroupListIdInSubjectAssociations,
    #[error("invalid subject id in subject associations")]
    InvalidSubjectIdInSubjectAssociations,
    #[error("subject association given but subject does not have interrogations")]
    SubjectAssociationForSubjectWithoutInterrogations,
    #[error("subject association given but subject does not run on given period")]
    SubjectAssociationForSubjectNotRunningOnPeriod,
    #[error("invalid student id in settings")]
    InvalidStudentIdInSettings,
    #[error("week pattern is invalid")]
    InvalidWeekPattern,
    #[error("invalid subject id in balancing")]
    InvalidSubjectIdInBalancing,
    #[error("balancing options given for subject without interrogations")]
    BalancingForSubjectWithoutInterrogations,
    #[error("invalid pairing rule")]
    InvalidPairingRule,
    #[error("invalid slot pairing rule")]
    InvalidSlotPairingRule,
}
