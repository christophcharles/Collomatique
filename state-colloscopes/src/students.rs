//! Students submodule
//!
//! This module defines the relevant types to describes the students

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use thiserror::Error;

use crate::ids::{GroupListId, PeriodId, StudentId, SubjectId};

/// Description of the students
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Students {
    /// List of students
    ///
    /// Each item associates an id to a student description
    pub student_map: BTreeMap<StudentId, Student>,
}

/// Description of a single student
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Student {
    /// Description of the student in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of periods the student will not take part in
    pub excluded_periods: BTreeSet<PeriodId>,
}

/// Errors for students operations
///
/// These errors can be returned when trying to modify [crate::Data] with a student op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StudentError {
    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// The student id already exists
    #[error("student id ({0:?}) already exists")]
    StudentIdAlreadyExists(StudentId),

    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// Some non-default assignments are still present for the student
    #[error(
        "student id {0:?} has non-default assignments for subject id {1:?} in period id ({0:?}) and cannot be removed or updated"
    )]
    StudentStillHasNonTrivialAssignments(StudentId, SubjectId, PeriodId),

    /// Student is still excluded by a group list
    #[error("student id {0:?} is still excluded by a group list {1:?}")]
    StudentIsStillExcludedByGroupList(StudentId, GroupListId),

    /// Student is still referenced by a pre-filled group list
    #[error("student id {0:?} is still referenced by a pre-filled group list {1:?}")]
    StudentIsStillReferencedByPrefilledGroupList(StudentId, GroupListId),

    /// Student is referenced in a colloscope group list
    #[error("student id {0:?} is referenced in a colloscope group list ({1:?})")]
    StudentIsReferencedInColloscopeGroupList(StudentId, GroupListId),

    /// Student still has per-student settings
    #[error("student id {0:?} still has per-student settings")]
    StudentStillHasSettings(StudentId),
}
