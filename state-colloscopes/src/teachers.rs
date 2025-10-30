//! Teachers submodule
//!
//! This module defines the relevant types to describes the teachers

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{SubjectId, TeacherId};

/// Description of the teachers
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Teachers {
    /// List of teachers
    ///
    /// Each item associates an id to a teacher description
    pub teacher_map: BTreeMap<TeacherId, Teacher>,
}

/// Description of a single teacher
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Teacher {
    /// Description of the teacher in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of subjects the teacher can interrogate in
    pub subjects: BTreeSet<SubjectId>,
}
