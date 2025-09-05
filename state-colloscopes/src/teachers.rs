//! Teachers submodule
//!
//! This module defines the relevant types to describes the teachers

use std::collections::{BTreeMap, BTreeSet};

use crate::ids::Id;

/// Description of the teachers
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Teachers<TeacherId: Id, SubjectId: Id> {
    /// List of teachers
    ///
    /// Each item associates an id to a teacher description
    pub teacher_map: BTreeMap<TeacherId, Teacher<SubjectId>>,
}

impl<TeacherId: Id, SubjectId: Id> Default for Teachers<TeacherId, SubjectId> {
    fn default() -> Self {
        Teachers {
            teacher_map: BTreeMap::new(),
        }
    }
}

/// Description of a single teacher
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Teacher<SubjectId: Id> {
    /// Description of the teacher in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of subjects the teacher can interrogate in
    pub subjects: BTreeSet<SubjectId>,
}

impl<SubjectId: Id> Default for Teacher<SubjectId> {
    fn default() -> Self {
        Teacher {
            desc: crate::PersonWithContact::default(),
            subjects: BTreeSet::new(),
        }
    }
}

impl<SubjectId: Id> Teacher<SubjectId> {
    /// Builds a teacher from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [TeacherExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: TeacherExternalData) -> Self {
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

impl<TeacherId: Id, SubjectId: Id> Teachers<TeacherId, SubjectId> {
    /// Builds teachers from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [TeachersExternalData::validate_all] and [TeacherExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: TeachersExternalData) -> Self {
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
    pub fn validate_all(&self, subjects: &super::subjects::SubjectsExternalData) -> bool {
        self.teacher_map
            .iter()
            .all(|(_id, data)| data.validate(subjects))
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
    pub fn validate(&self, subjects: &super::subjects::SubjectsExternalData) -> bool {
        if !self.subjects.iter().all(|x| {
            let Some(subject) = subjects.find_subject(*x) else {
                return false;
            };
            subject.parameters.interrogation_parameters.is_some()
        }) {
            return false;
        }
        true
    }
}

impl<SubjectId: Id> From<Teacher<SubjectId>> for TeacherExternalData {
    fn from(value: Teacher<SubjectId>) -> Self {
        TeacherExternalData {
            desc: value.desc,
            subjects: value.subjects.iter().map(|x| x.inner()).collect(),
        }
    }
}
