//! Teachers submodule
//!
//! This module defines the relevant types to describes the teachers

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::ids::{ColloscopeSubjectId, ColloscopeTeacherId, Id};

/// Description of the teachers
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    pub(crate) fn duplicate_with_id_maps(
        &self,
        subjects_map: &BTreeMap<SubjectId, ColloscopeSubjectId>,
    ) -> Option<Teacher<ColloscopeSubjectId>> {
        let mut subjects = BTreeSet::new();

        for subject_id in &self.subjects {
            let new_id = subjects_map.get(subject_id)?;
            subjects.insert(*new_id);
        }

        Some(Teacher {
            desc: self.desc.clone(),
            subjects,
        })
    }
}

impl<TeacherId: Id, SubjectId: Id> Teachers<TeacherId, SubjectId> {
    pub(crate) fn duplicate_with_id_maps(
        &self,
        teachers_map: &BTreeMap<TeacherId, ColloscopeTeacherId>,
        subjects_map: &BTreeMap<SubjectId, ColloscopeSubjectId>,
    ) -> Option<Teachers<ColloscopeTeacherId, ColloscopeSubjectId>> {
        let mut teacher_map = BTreeMap::new();

        for (teacher_id, teacher) in &self.teacher_map {
            let new_id = teachers_map.get(teacher_id)?;
            let new_teacher = teacher.duplicate_with_id_maps(subjects_map)?;
            teacher_map.insert(*new_id, new_teacher);
        }

        Some(Teachers { teacher_map })
    }
}
