//! Students submodule
//!
//! This module defines the relevant types to describes the students

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{ColloscopePeriodId, ColloscopeStudentId, Id};

/// Description of the students
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Students<StudentId: Id, PeriodId: Id> {
    /// List of students
    ///
    /// Each item associates an id to a student description
    pub student_map: BTreeMap<StudentId, Student<PeriodId>>,
}

impl<StudentId: Id, PeriodId: Id> Default for Students<StudentId, PeriodId> {
    fn default() -> Self {
        Students {
            student_map: BTreeMap::new(),
        }
    }
}

/// Description of a single student
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Student<PeriodId: Id> {
    /// Description of the student in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of periods the student will not take part in
    pub excluded_periods: BTreeSet<PeriodId>,
}

impl<PeriodId: Id> Default for Student<PeriodId> {
    fn default() -> Self {
        Student {
            desc: crate::PersonWithContact::default(),
            excluded_periods: BTreeSet::new(),
        }
    }
}

impl<PeriodId: Id> Student<PeriodId> {
    pub(crate) fn duplicate_with_id_maps(
        &self,
        periods_map: &BTreeMap<PeriodId, ColloscopePeriodId>,
    ) -> Option<Student<ColloscopePeriodId>> {
        let mut excluded_periods = BTreeSet::new();

        for period_id in &self.excluded_periods {
            let new_id = periods_map.get(period_id)?;
            excluded_periods.insert(*new_id);
        }

        Some(Student {
            desc: self.desc.clone(),
            excluded_periods,
        })
    }
}

impl<StudentId: Id, PeriodId: Id> Students<StudentId, PeriodId> {
    pub(crate) fn duplicate_with_id_maps(
        &self,
        students_map: &BTreeMap<StudentId, ColloscopeStudentId>,
        periods_map: &BTreeMap<PeriodId, ColloscopePeriodId>,
    ) -> Option<Students<ColloscopeStudentId, ColloscopePeriodId>> {
        let mut student_map = BTreeMap::new();

        for (student_id, student) in &self.student_map {
            let new_id = students_map.get(student_id)?;
            let new_student = student.duplicate_with_id_maps(periods_map)?;
            student_map.insert(*new_id, new_student);
        }

        Some(Students { student_map })
    }
}
