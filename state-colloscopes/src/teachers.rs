//! Teachers submodule
//!
//! This module defines the relevant types to describes the teachers

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use thiserror::Error;

use collomatique_state::{Join, References};

use crate::Table;
use crate::ids::{NewId, SubjectId, TeacherId};
use crate::ops::AnnotatedTeacherOp;

/// Description of the teachers
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Teachers {
    /// List of teachers
    ///
    /// Each item associates an id to a teacher description
    pub teacher_map: Table<TeacherId, Teacher>,
}

/// Description of a single teacher
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, References, Join)]
#[join(error = NewId)]
pub struct Teacher {
    /// Description of the teacher in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of subjects the teacher can interrogate in
    #[fk]
    pub subjects: BTreeSet<SubjectId>,
}

/// Precondition errors of the forced teacher ops — the carve-out subset
/// (step-3 survey Table 2). See [StudentPrecheckError](crate::StudentPrecheckError)
/// for the shape rationale.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum TeacherPrecheckError {
    /// A teacher id is invalid
    #[error("invalid teacher id ({0:?})")]
    InvalidTeacherId(TeacherId),

    /// The teacher id already exists
    #[error("teacher id ({0:?}) already exists")]
    TeacherIdAlreadyExists(TeacherId),
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a teacher op: carve-out guards kept (returned as
    /// [TeacherPrecheckError]), invariant guards stripped (step-3 survey Table 1).
    /// May leave the state invalid; the caller owns checking and rollback.
    pub(crate) fn force_apply_teacher(
        &mut self,
        teacher_op: &AnnotatedTeacherOp,
    ) -> std::result::Result<AnnotatedTeacherOp, TeacherPrecheckError> {
        match teacher_op {
            AnnotatedTeacherOp::Add(new_id, teacher) => {
                if self.inner_data.params.teachers.teacher_map.contains(new_id) {
                    return Err(TeacherPrecheckError::TeacherIdAlreadyExists(*new_id));
                }
                // stripped: validate_teacher

                self.inner_data
                    .params
                    .teachers
                    .teacher_map
                    .insert(*new_id, teacher.clone());

                Ok(AnnotatedTeacherOp::Remove(*new_id))
            }
            AnnotatedTeacherOp::Remove(id) => {
                // stripped: slot-reference scan
                let Some(old_teacher) = self.inner_data.params.teachers.teacher_map.remove(id)
                else {
                    return Err(TeacherPrecheckError::InvalidTeacherId(*id));
                };

                Ok(AnnotatedTeacherOp::Add(*id, old_teacher))
            }
            AnnotatedTeacherOp::Update(id, new_teacher) => {
                // stripped: validate_teacher + dropped-subject slot scan
                let Some(current_teacher) = self.inner_data.params.teachers.teacher_map.get_mut(id)
                else {
                    return Err(TeacherPrecheckError::InvalidTeacherId(*id));
                };

                let old_teacher = std::mem::replace(current_teacher, new_teacher.clone());

                Ok(AnnotatedTeacherOp::Update(*id, old_teacher))
            }
        }
    }
}
