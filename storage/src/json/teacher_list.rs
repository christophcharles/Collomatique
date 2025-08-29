//! teachers submodule
//!
//! This module defines the teachers entry for the JSON description
//!
use super::*;

use std::collections::{BTreeMap, BTreeSet};

/// JSON desc of teachers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    /// map between ids and teachers
    ///
    /// each teacher is described by an id (which should not
    /// be duplicate) and a structure [Teacher]
    #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
    pub teacher_map: BTreeMap<u64, Teacher>,
}

/// JSON desc of a single teacher
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Teacher {
    pub desc: super::common::PersonWithContact,
    pub subjects: BTreeSet<u64>,
}

impl From<&collomatique_state_colloscopes::teachers::Teacher> for Teacher {
    fn from(value: &collomatique_state_colloscopes::teachers::Teacher) -> Self {
        Teacher {
            desc: (&value.desc).into(),
            subjects: value.subjects.iter().map(|x| x.inner()).collect(),
        }
    }
}

impl From<Teacher> for collomatique_state_colloscopes::teachers::TeacherExternalData {
    fn from(value: Teacher) -> Self {
        collomatique_state_colloscopes::teachers::TeacherExternalData {
            desc: value.desc.into(),
            subjects: value.subjects,
        }
    }
}
