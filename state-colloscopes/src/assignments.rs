//! Assignments submodule
//!
//! This module defines the relevant types to describes the assignments

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{ColloscopePeriodId, ColloscopeStudentId, ColloscopeSubjectId, Id};

/// Description of the assignments
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    pub(crate) fn duplicate_with_id_maps(
        &self,
        subjects_map: &BTreeMap<SubjectId, ColloscopeSubjectId>,
        students_map: &BTreeMap<StudentId, ColloscopeStudentId>,
    ) -> Option<PeriodAssignments<ColloscopeSubjectId, ColloscopeStudentId>> {
        let mut subject_map = BTreeMap::new();

        for (subject_id, student_set) in &self.subject_map {
            let new_subject_id = subjects_map.get(subject_id)?;
            let mut new_student_set = BTreeSet::new();

            for student_id in student_set {
                let new_student_id = students_map.get(student_id)?;
                new_student_set.insert(*new_student_id);
            }

            subject_map.insert(*new_subject_id, new_student_set);
        }

        Some(PeriodAssignments { subject_map })
    }
}

impl<PeriodId: Id, SubjectId: Id, StudentId: Id> Assignments<PeriodId, SubjectId, StudentId> {
    pub(crate) fn duplicate_with_id_maps(
        &self,
        periods_map: &BTreeMap<PeriodId, ColloscopePeriodId>,
        subjects_map: &BTreeMap<SubjectId, ColloscopeSubjectId>,
        students_map: &BTreeMap<StudentId, ColloscopeStudentId>,
    ) -> Option<Assignments<ColloscopePeriodId, ColloscopeSubjectId, ColloscopeStudentId>> {
        let mut period_map = BTreeMap::new();

        for (period_id, period_assignments) in &self.period_map {
            let new_id = periods_map.get(period_id)?;
            let new_period_assignments =
                period_assignments.duplicate_with_id_maps(subjects_map, students_map)?;

            period_map.insert(*new_id, new_period_assignments);
        }

        Some(Assignments { period_map })
    }
}
