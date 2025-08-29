//! Students submodule
//!
//! This module defines the relevant types to describes the students

use std::collections::{BTreeMap, BTreeSet};

use crate::ids::{PeriodId, StudentId};

/// Description of the students
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Students {
    /// List of students
    ///
    /// Each item associates an id to a student description
    pub student_map: BTreeMap<StudentId, Student>,
}

/// Description of a single student
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Student {
    /// Description of the student in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of periods the student will not take part in
    pub excluded_periods: BTreeSet<PeriodId>,
}

impl Student {
    /// Builds a student from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [StudentExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: StudentExternalData) -> Student {
        Student {
            desc: external_data.desc,
            excluded_periods: external_data
                .excluded_periods
                .into_iter()
                .map(|x| unsafe { PeriodId::new(x) })
                .collect(),
        }
    }
}

impl Students {
    /// Builds students from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [StudentsExternalData::validate_all] and [StudentExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: StudentsExternalData) -> Students {
        Students {
            student_map: external_data
                .student_map
                .into_iter()
                .map(|(id, data)| {
                    (unsafe { StudentId::new(id) }, unsafe {
                        Student::from_external_data(data)
                    })
                })
                .collect(),
        }
    }
}

/// Description of the students but unchecked
///
/// This structure is an unchecked equivalent of [Students].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StudentsExternalData {
    /// List of students
    ///
    /// Each item associates an id to a student description
    pub student_map: BTreeMap<u64, StudentExternalData>,
}

impl StudentsExternalData {
    /// Checks the validity of all [StudentExternalData] in the map.
    ///
    /// In practice, this means checking that the ids for periods are valid,
    ///
    /// **Beware**, this does not check the validity of the ids for the students!
    pub fn validate_all(&self, period_ids: &BTreeSet<u64>) -> bool {
        self.student_map
            .iter()
            .all(|(_id, data)| data.validate(period_ids))
    }
}

/// Description of a single student but unchecked
///
/// This structure is an unchecked equivalent of [Student].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StudentExternalData {
    /// Description of the student in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of periods ids the student does not take part in
    pub excluded_periods: BTreeSet<u64>,
}

impl StudentExternalData {
    /// Checks the validity of a [StudentExternalData].
    ///
    /// In practice, this means checking that the ids for periods are valid
    pub fn validate(&self, period_ids: &BTreeSet<u64>) -> bool {
        if !self.excluded_periods.iter().all(|x| period_ids.contains(x)) {
            return false;
        }
        true
    }
}

impl From<Student> for StudentExternalData {
    fn from(value: Student) -> Self {
        StudentExternalData {
            desc: value.desc,
            excluded_periods: value.excluded_periods.iter().map(|x| x.inner()).collect(),
        }
    }
}
