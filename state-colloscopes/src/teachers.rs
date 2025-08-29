//! Teachers submodule
//!
//! This module defines the relevant types to describes the teachers

use std::collections::{BTreeMap, BTreeSet};

use crate::ids::{SubjectId, TeacherId};

/// Description of the teachers
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Teachers {
    /// List of teachers
    ///
    /// Each item associates an id to a teacher description
    pub teacher_map: BTreeMap<TeacherId, Teacher>,
}

/// Description of a single teacher
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Teacher {
    /// Description of the teacher in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of subjects the teacher can interrogate in
    pub subjects: BTreeSet<SubjectId>,
}

impl Teacher {
    /// Builds a teacher from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [TeacherExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: TeacherExternalData) -> Teacher {
        Teacher {
            desc: external_data.desc,
            subjects: external_data
                .subjects
                .into_iter()
                .map(|x| unsafe { SubjectId::new(x) })
                .collect(),
        }
    }
}

impl Teachers {
    /// Builds teachers from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [TeachersExternalData::validate_all] and [TeacherExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: TeachersExternalData) -> Teachers {
        Teachers {
            teacher_map: external_data
                .teacher_map
                .into_iter()
                .map(|(id, data)| {
                    (unsafe { TeacherId::new(id) }, unsafe {
                        Teacher::from_external_data(data)
                    })
                })
                .collect(),
        }
    }
}

/// Description of the teachers but unchecked
///
/// This structure is an unchecked equivalent of [Teachers].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TeachersExternalData {
    /// List of teachers
    ///
    /// Each item associates an id to a teacher description
    pub teacher_map: BTreeMap<u64, TeacherExternalData>,
}

impl TeachersExternalData {
    /// Checks the validity of all [TeacherExternalData] in the map.
    ///
    /// In practice, this means checking that the ids for subjects are valid,
    ///
    /// **Beware**, this does not check the validity of the ids for the teachers!
    pub fn validate_all(&self, subject_ids: &BTreeSet<u64>) -> bool {
        self.teacher_map
            .iter()
            .all(|(_id, data)| data.validate(subject_ids))
    }
}

/// Description of a single teacher but unchecked
///
/// This structure is an unchecked equivalent of [Teacher].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeacherExternalData {
    /// Description of the teacher in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of subjects ids the teacher can interrogate in
    pub subjects: BTreeSet<u64>,
}

impl TeacherExternalData {
    /// Checks the validity of a [TeacherExternalData].
    ///
    /// In practice, this means checking that the ids for subjects are valid
    pub fn validate(&self, subject_ids: &BTreeSet<u64>) -> bool {
        if !self.subjects.iter().all(|x| subject_ids.contains(x)) {
            return false;
        }
        true
    }
}

impl From<Teacher> for TeacherExternalData {
    fn from(value: Teacher) -> Self {
        TeacherExternalData {
            desc: value.desc,
            subjects: value.subjects.iter().map(|x| x.inner()).collect(),
        }
    }
}
