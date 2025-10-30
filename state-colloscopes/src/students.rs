//! Students submodule
//!
//! This module defines the relevant types to describes the students

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{PeriodId, StudentId};

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
