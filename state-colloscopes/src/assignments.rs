//! Assignments submodule
//!
//! This module defines the relevant types to describes the assignments

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{PeriodId, StudentId, SubjectId};

/// Description of the assignments
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignments {
    /// Assignments for each period
    ///
    /// Each item associates a period id to an assignment description
    /// There should be an entry for each valid period
    pub period_map: BTreeMap<PeriodId, PeriodAssignments>,
}

/// Description of an assignment for a period
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodAssignments {
    /// Assignments for each student on the period
    ///
    /// Each item associates a subject id to an assignment description
    /// There should be an entry for each valid subject in the period
    /// The set is the list of students who do attend during the period
    pub subject_map: BTreeMap<SubjectId, BTreeSet<StudentId>>,
}
