//! Student_list submodule
//!
//! This module defines the student list entry for the JSON description
//!
use super::*;

use std::collections::{BTreeMap, BTreeSet};

/// JSON desc of Students
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    /// map between ids and students
    ///
    /// each student is described by an id (which should not
    /// be duplicate) and a structure [Student]
    #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
    pub student_map: BTreeMap<u64, Student>,
}

/// JSON desc of a single student
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Student {
    pub desc: super::common::PersonWithContact,
    pub excluded_periods: BTreeSet<u64>,
}

impl From<&collomatique_state_colloscopes::students::Student> for Student {
    fn from(value: &collomatique_state_colloscopes::students::Student) -> Self {
        Student {
            desc: (&value.desc).into(),
            excluded_periods: value.excluded_periods.iter().map(|x| x.inner()).collect(),
        }
    }
}

impl From<Student> for collomatique_state_colloscopes::students::StudentExternalData {
    fn from(value: Student) -> Self {
        collomatique_state_colloscopes::students::StudentExternalData {
            desc: value.desc.into(),
            excluded_periods: value.excluded_periods,
        }
    }
}
