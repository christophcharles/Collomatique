//! Colloscope parameters submodule
//!
//! This module defines the relevant types to describes the full set of parameters for colloscopes

use crate::ids::{
    GroupListId, IncompatId, NewId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekPatternId,
};

use super::*;

use collomatique_state::Lookup;
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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    pub pairings: pairings::Pairings,
    pub slot_pairings: slot_pairings::SlotPairings,
    pub balancing: balancing::Balancing,
}

impl Parameters {
    pub(crate) fn merge_pattern(&self, pattern: &[bool]) -> Vec<bool> {
        let mut current_week = 0usize;
        let mut output = Vec::new();
        for (_period_id, period_desc) in self.periods.ordered_period_list.iter() {
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
        for (period_id, _) in self.periods.ordered_period_list.iter() {
            if period_id.inner() == id {
                return Some(period_id);
            }
        }

        None
    }

    /// Promotes an u64 to a [StudentId] if it is valid
    pub fn validate_student_id(&self, id: u64) -> Option<StudentId> {
        let student_id = unsafe { StudentId::new(id) };

        if !self.students.student_map.contains(&student_id) {
            return None;
        }

        Some(student_id)
    }

    /// Promotes an u64 to a [SubjectId] if it is valid
    pub fn validate_subject_id(&self, id: u64) -> Option<SubjectId> {
        for (subject_id, _) in self.subjects.ordered_subject_list.iter() {
            if subject_id.inner() == id {
                return Some(subject_id);
            }
        }

        None
    }

    /// Promotes an u64 to a [TeacherId] if it is valid
    pub fn validate_teacher_id(&self, id: u64) -> Option<TeacherId> {
        let temp_teacher_id = unsafe { TeacherId::new(id) };
        if self.teachers.teacher_map.contains(&temp_teacher_id) {
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
            .contains(&temp_week_pattern_id)
        {
            return Some(temp_week_pattern_id);
        }

        None
    }

    /// Promotes an u64 to a [SlotId] if it is valid
    pub fn validate_slot_id(&self, id: u64) -> Option<SlotId> {
        let slot_id = unsafe { SlotId::new(id) };
        if self.slots.find_slot(slot_id).is_some() {
            Some(slot_id)
        } else {
            None
        }
    }

    /// Promotes an u64 to a [IncompatId] if it is valid
    pub fn validate_incompat_id(&self, id: u64) -> Option<IncompatId> {
        let temp_incompat_id = unsafe { IncompatId::new(id) };
        if self.incompats.incompat_map.contains(&temp_incompat_id) {
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
            .contains(&temp_group_list_id)
        {
            return Some(temp_group_list_id);
        }

        None
    }
}

// --- Keyed read interface (SQL-like lookup) ---
//
// One [`Lookup`] impl per entity kind, keyed on the matching typed id and
// resolving to the entity type declared in that id's `#[entity(…)]` attribute
// (`ids.rs`). Each delegates to the container accessor already used elsewhere
// in this module, so lookup borrows straight out of the table — no clone, no
// rebuild. These are the context impls the `Join` derives resolve against.

impl Lookup<PeriodId> for Parameters {
    type Entity = Vec<periods::WeekDesc>;
    fn lookup(&self, id: PeriodId) -> Option<&Vec<periods::WeekDesc>> {
        self.periods.find_period(id)
    }
}

impl Lookup<SubjectId> for Parameters {
    type Entity = subjects::Subject;
    fn lookup(&self, id: SubjectId) -> Option<&subjects::Subject> {
        self.subjects.find_subject(id)
    }
}

impl Lookup<TeacherId> for Parameters {
    type Entity = teachers::Teacher;
    fn lookup(&self, id: TeacherId) -> Option<&teachers::Teacher> {
        self.teachers.teacher_map.get(&id)
    }
}

impl Lookup<StudentId> for Parameters {
    type Entity = students::Student;
    fn lookup(&self, id: StudentId) -> Option<&students::Student> {
        self.students.student_map.get(&id)
    }
}

impl Lookup<WeekPatternId> for Parameters {
    type Entity = week_patterns::WeekPattern;
    fn lookup(&self, id: WeekPatternId) -> Option<&week_patterns::WeekPattern> {
        self.week_patterns.week_pattern_map.get(&id)
    }
}

impl Lookup<SlotId> for Parameters {
    type Entity = slots::Slot;
    fn lookup(&self, id: SlotId) -> Option<&slots::Slot> {
        self.slots.find_slot(id)
    }
}

impl Lookup<IncompatId> for Parameters {
    type Entity = incompats::Incompatibility;
    fn lookup(&self, id: IncompatId) -> Option<&incompats::Incompatibility> {
        self.incompats.incompat_map.get(&id)
    }
}

impl Lookup<GroupListId> for Parameters {
    type Entity = group_lists::GroupList;
    fn lookup(&self, id: GroupListId) -> Option<&group_lists::GroupList> {
        self.group_lists.group_list_map.get(&id)
    }
}

impl Lookup<PairingRuleId> for Parameters {
    type Entity = pairings::PairingRule;
    fn lookup(&self, id: PairingRuleId) -> Option<&pairings::PairingRule> {
        self.pairings.pairing_rule_map.get(&id)
    }
}

impl Lookup<SlotPairingRuleId> for Parameters {
    type Entity = slot_pairings::SlotPairingRule;
    fn lookup(&self, id: SlotPairingRuleId) -> Option<&slot_pairings::SlotPairingRule> {
        self.slot_pairings.slot_pairing_rule_map.get(&id)
    }
}

impl Parameters {
    /// Typed keyed lookup — the fallible entry point.
    ///
    /// Resolves any typed id against its table, returning `None` when the id
    /// dangles. Use this for candidate/unvalidated data where a missing target
    /// is a legitimate outcome. The concrete entity type is inferred from the
    /// id kind through the [`Lookup`] impls above.
    pub fn lookup<I>(&self, id: I) -> Option<&<Self as Lookup<I>>::Entity>
    where
        Self: Lookup<I>,
    {
        <Self as Lookup<I>>::lookup(self, id)
    }

    /// Infallible resolution for already-validated data.
    ///
    /// The invariant checks guarantee no reference dangles once a document is
    /// committed, so on that data every id resolves. This variant unwraps that
    /// guarantee and **panics** (printing the offending id) if it is ever
    /// violated — a dangling id here is a bug, not an expected input.
    pub fn resolve<I: Id>(&self, id: I) -> &<Self as Lookup<I>>::Entity
    where
        Self: Lookup<I>,
    {
        <Self as Lookup<I>>::lookup(self, id)
            .unwrap_or_else(|| panic!("dangling {id:?} in validated data"))
    }
}

impl Parameters {
    /// Every primary-key id in the document, typed as [`NewId`], in the
    /// canonical table order.
    ///
    /// This is the single declared enumeration of the ten entity tables. The
    /// order — students, periods, subjects, teachers, week patterns, slots,
    /// incompats, group lists, pairing rules, slot pairing rules — is kept
    /// identical to the historical [`Parameters::ids`] chain, which now defers
    /// to this method.
    pub fn all_ids(&self) -> impl Iterator<Item = NewId> + '_ {
        self.students
            .student_map
            .keys()
            .map(NewId::from)
            .chain(self.periods.ordered_period_list.keys().map(NewId::from))
            .chain(self.subjects.ordered_subject_list.keys().map(NewId::from))
            .chain(self.teachers.teacher_map.keys().map(NewId::from))
            .chain(self.week_patterns.week_pattern_map.keys().map(NewId::from))
            .chain(self.slots.slot_ids().map(NewId::from))
            .chain(self.incompats.incompat_map.keys().map(NewId::from))
            .chain(self.group_lists.group_list_map.keys().map(NewId::from))
            .chain(self.pairings.pairing_rule_map.keys().map(NewId::from))
            .chain(
                self.slot_pairings
                    .slot_pairing_rule_map
                    .keys()
                    .map(NewId::from),
            )
    }

    /// USED INTERNALLY
    ///
    /// Returns an iterator on all ids that appear in the colloscope params, as
    /// raw `u64`. A thin numeric adapter over [`Parameters::all_ids`].
    pub(crate) fn ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.all_ids().map(|id| id.inner())
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
        for (_subject_id, subject) in self.subjects.ordered_subject_list.iter() {
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
        // Dense key set: exactly one `(period, subject)` entry per period and
        // per subject that runs on it (i.e. is not excluded on it).
        let mut expected_count = 0usize;
        for period_id in period_ids {
            for (subject_id, subject) in self.subjects.ordered_subject_list.iter() {
                if subject.excluded_periods.contains(period_id) {
                    continue;
                }
                expected_count += 1;

                let subject_assignments = self
                    .assignments
                    .students(*period_id, subject_id)
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
        }

        // Any extra key is either on an unknown period or on an excluded/unknown
        // subject; distinguish the two so each keeps its historical error.
        if self.assignments.map.len() != expected_count {
            for (period_id, _subject_id) in self.assignments.map.keys() {
                if !period_ids.contains(&period_id) {
                    return Err(InvariantError::InvalidPeriodIdInAssignements);
                }
            }
            return Err(InvariantError::WrongSubjectCountInAssignments);
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
    pub(crate) fn validate_slot(&self, slot: &slots::Slot) -> Result<(), SlotError> {
        let week_pattern_ids = self.build_week_pattern_ids();

        Self::validate_slot_internal(
            slot,
            slot.subject_id,
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
        // Dense-key semantics: the ordering sidecar has exactly one entry per
        // subject with interrogations.
        let subjects_with_interrogations: BTreeSet<SubjectId> = self
            .subjects
            .ordered_subject_list
            .iter()
            .filter(|(_id, subject)| subject.parameters.interrogation_parameters.is_some())
            .map(|(id, _)| id)
            .collect();
        let ordering_subjects: BTreeSet<SubjectId> = self.slots.subjects_with_slots().collect();
        if ordering_subjects != subjects_with_interrogations {
            return Err(InvariantError::WrongSubjectCountInSlots);
        }

        // Every slot referenced by the ordering must exist in the slot table,
        // sit under the subject naming it (matching `slot.subject_id`), appear
        // exactly once, and validate. `find_slot` is used (not `slots_for_subject`)
        // so a desynchronized ordering yields a clean error rather than panicking.
        let mut ordered_ids = BTreeSet::new();
        for (subject_id, order) in self.slots.ordering_entries() {
            for slot_id in order {
                let Some(slot) = self.slots.find_slot(*slot_id) else {
                    return Err(InvariantError::InvalidSlot);
                };
                if slot.subject_id != subject_id {
                    return Err(InvariantError::InvalidSlot);
                }
                if !ordered_ids.insert(*slot_id) {
                    return Err(InvariantError::InvalidSlot);
                }
                if Self::validate_slot_internal(
                    slot,
                    subject_id,
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

        // No orphan slots: every slot in the table is covered by the ordering.
        for slot_id in self.slots.slot_ids() {
            if !ordered_ids.contains(&slot_id) {
                return Err(InvariantError::InvalidSlot);
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
                        if !students.student_map.contains(student_id) {
                            return Err(GroupListError::InvalidStudentId(*student_id));
                        }
                    }
                }
            }
            group_lists::GroupListFilling::Automatic { excluded_students } => {
                for student_id in excluded_students {
                    if !students.student_map.contains(student_id) {
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
        // The associations table is sparse (one row per associated
        // `(period, subject)`), so there is no per-period denseness to check;
        // instead every row's period and subject must be valid.
        let period_ids = self.build_period_ids();
        for ((period_id, subject_id), group_list_id) in
            self.group_lists.subjects_associations.iter()
        {
            if !period_ids.contains(&period_id) {
                return Err(InvariantError::WrongPeriodCountInSubjectAssociationsForGroupLists);
            }
            if !self.group_lists.group_list_map.contains(group_list_id) {
                return Err(InvariantError::InvalidGroupListIdInSubjectAssociations);
            }
            let subject = self
                .subjects
                .find_subject(subject_id)
                .ok_or(InvariantError::InvalidSubjectIdInSubjectAssociations)?;

            if subject.parameters.interrogation_parameters.is_none() {
                return Err(InvariantError::SubjectAssociationForSubjectWithoutInterrogations);
            };
            if subject.excluded_periods.contains(&period_id) {
                return Err(InvariantError::SubjectAssociationForSubjectNotRunningOnPeriod);
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
            if !self.students.student_map.contains(&student_id) {
                return Err(SettingsError::InvalidStudentId(student_id));
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
            let Some(subject) = self.subjects.find_subject(subject_id) else {
                return Err(BalancingError::InvalidSubjectId(subject_id));
            };
            if subject.parameters.interrogation_parameters.is_none() {
                return Err(BalancingError::SubjectHasNoInterrogation(subject_id));
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
        if self.pairings.pairing_rule_map.contains(&temp_id) {
            return Some(temp_id);
        }

        None
    }

    /// Builds a map from SlotId to SubjectId for all slots
    fn build_slot_subject_map(&self) -> BTreeMap<SlotId, SubjectId> {
        self.slots
            .all_slots()
            .map(|(slot_id, slot)| (*slot_id, slot.subject_id))
            .collect()
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
            .iter()
            .map(|(id, _)| id)
            .collect();
        Self::validate_slot_pairing_rule_internal(rule, &slot_subject_map, &period_ids)
    }

    /// Promotes an u64 to a [SlotPairingRuleId] if it is valid
    pub fn validate_slot_pairing_rule_id(&self, id: u64) -> Option<SlotPairingRuleId> {
        let id = unsafe { SlotPairingRuleId::new(id) };
        if self.slot_pairings.slot_pairing_rule_map.contains(&id) {
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
        for (id, _) in self.periods.ordered_period_list.iter() {
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
        self.week_patterns.week_pattern_map.keys().collect()
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
            .iter()
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
