//! Assignments submodule
//!
//! This module defines the relevant types to describes the assignments

use std::collections::{BTreeMap, BTreeSet};

use crate::ids::Id;

/// Description of the assignments
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignments<PeriodId: Id, SubjectId: Id, StudentId: Id> {
    /// Assignments for each period
    ///
    /// Each item associates a period id to an assignment description
    /// There should be an entry for each valid period
    pub period_map: BTreeMap<PeriodId, PeriodAssignments<SubjectId, StudentId>>,
}

impl<PeriodId: Id, SubjectId: Id, StudentId: Id> Default
    for Assignments<PeriodId, SubjectId, StudentId>
{
    fn default() -> Self {
        Assignments {
            period_map: BTreeMap::new(),
        }
    }
}

/// Description of an assignment for a period
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodAssignments<SubjectId: Id, StudentId: Id> {
    /// Assignments for each student on the period
    ///
    /// Each item associates a subject id to an assignment description
    /// There should be an entry for each valid subject in the period
    /// The set is the list of students who do attend during the period
    pub subject_map: BTreeMap<SubjectId, BTreeSet<StudentId>>,
}

impl<SubjectId: Id, StudentId: Id> Default for PeriodAssignments<SubjectId, StudentId> {
    fn default() -> Self {
        PeriodAssignments {
            subject_map: BTreeMap::new(),
        }
    }
}

impl<SubjectId: Id, StudentId: Id> PeriodAssignments<SubjectId, StudentId> {
    /// Builds a period assignment from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [PeriodAssignmentExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: PeriodAssignmentsExternalData) -> Self {
        PeriodAssignments {
            subject_map: external_data
                .subject_map
                .into_iter()
                .map(|(subject_id, student_set)| {
                    (
                        unsafe { SubjectId::new(subject_id) },
                        student_set
                            .into_iter()
                            .map(|x| unsafe { StudentId::new(x) })
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

impl<PeriodId: Id, SubjectId: Id, StudentId: Id> Assignments<PeriodId, SubjectId, StudentId> {
    /// Builds an assignment from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [AssignmentsExternalData::validate_all] and [PeriodAssignmentsExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: AssignmentsExternalData) -> Self {
        Assignments {
            period_map: external_data
                .period_map
                .into_iter()
                .map(|(period_id, period_assignment)| {
                    (unsafe { PeriodId::new(period_id) }, unsafe {
                        PeriodAssignments::from_external_data(period_assignment)
                    })
                })
                .collect(),
        }
    }
}

/// Description of the assignments but unchecked
///
/// This structure is an unchecked equivalent of [Assignments].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AssignmentsExternalData {
    /// Assignments for each period
    ///
    /// Each item associates a period id to an assignment description
    /// There should be an entry for each valid period
    pub period_map: BTreeMap<u64, PeriodAssignmentsExternalData>,
}

impl AssignmentsExternalData {
    /// Checks the validity of all [PeriodAssignmentsExternalData] in the map.
    ///
    /// In practice, this means checking that the ids for periods are valid,
    /// and that the ids for students and subjects are valid
    pub fn validate_all(
        &self,
        period_ids: &BTreeSet<u64>,
        students: &super::students::StudentsExternalData,
        subjects: &super::subjects::SubjectsExternalData,
    ) -> bool {
        if self.period_map.len() != period_ids.len() {
            return false;
        }
        self.period_map.iter().all(|(period_id, data)| {
            if !period_ids.contains(period_id) {
                return false;
            }
            data.validate(*period_id, students, subjects)
        })
    }
}

/// Description of assignments for a period but unchecked
///
/// This structure is an unchecked equivalent of [PeriodAssignments].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PeriodAssignmentsExternalData {
    /// Assignments for each student on the period
    ///
    /// Each item associates a subject id to an assignment description
    /// There should be an entry for each valid subject in the period
    /// The set is the list of students who do attend during the period
    pub subject_map: BTreeMap<u64, BTreeSet<u64>>,
}

impl PeriodAssignmentsExternalData {
    /// Checks the validity of a [PeriodAssignmentsExternalData].
    pub fn validate(
        &self,
        current_period_id: u64,
        students: &super::students::StudentsExternalData,
        subjects: &super::subjects::SubjectsExternalData,
    ) -> bool {
        let mut subject_count_for_period = 0usize;
        for (subject_id, subject) in &subjects.ordered_subject_list {
            if subject.excluded_periods.contains(&current_period_id) {
                continue;
            }

            subject_count_for_period += 1;

            let Some(subject_assignments) = self.subject_map.get(subject_id) else {
                return false;
            };

            for student_id in subject_assignments {
                let Some(student) = students.student_map.get(student_id) else {
                    return false;
                };

                if student.excluded_periods.contains(&current_period_id) {
                    return false;
                }
            }
        }
        if subject_count_for_period != self.subject_map.len() {
            return false;
        }
        true
    }
}

impl<SubjectId: Id, StudentId: Id> From<PeriodAssignments<SubjectId, StudentId>>
    for PeriodAssignmentsExternalData
{
    fn from(value: PeriodAssignments<SubjectId, StudentId>) -> Self {
        PeriodAssignmentsExternalData {
            subject_map: value
                .subject_map
                .into_iter()
                .map(|(subject_id, student_map)| {
                    (
                        subject_id.inner(),
                        student_map.into_iter().map(|x| x.inner()).collect(),
                    )
                })
                .collect(),
        }
    }
}
