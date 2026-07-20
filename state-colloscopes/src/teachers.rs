//! Teachers submodule
//!
//! This module defines the relevant types to describes the teachers

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use thiserror::Error;

use collomatique_state::{Join, References};

use crate::Table;
use crate::ids::{NewId, SlotId, SubjectId, TeacherId};
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

/// Errors for teacher operations
///
/// These errors can be returned when trying to modify [crate::Data] with a teacher op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum TeacherError {
    /// A teacher id is invalid
    #[error("invalid teacher id ({0:?})")]
    InvalidTeacherId(TeacherId),

    /// The teacher id already exists
    #[error("teacher id ({0:?}) already exists")]
    TeacherIdAlreadyExists(TeacherId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// The selected subject does not have interrogations
    #[error("Subject id ({0:?}) corresponds to a subject without interrogations")]
    SubjectHasNoInterrogation(SubjectId),

    /// The teacher is referenced by a slot
    #[error("teacher id ({0:?}) is referenced by a slot ({1:?})")]
    TeacherStillHasAssociatedSlots(TeacherId, SlotId),

    /// The teacher is referenced by slots for a bad subject
    #[error("teacher id ({0:?}) gives interrogation in a now forbidden subject ({1:?})")]
    TeacherStillHasAssociatedSlotsInSubject(TeacherId, SubjectId),
}

/// Precondition errors of the forced teacher ops — the carve-out subset
/// (step-3 survey Table 2). See [StudentPrecheckError](crate::StudentPrecheckError)
/// for the shape rationale; variants copied verbatim from [TeacherError].
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
    /// Used internally
    ///
    /// Apply teacher operations
    pub(crate) fn apply_teacher(
        &mut self,
        teacher_op: &AnnotatedTeacherOp,
    ) -> std::result::Result<AnnotatedTeacherOp, TeacherError> {
        match teacher_op {
            AnnotatedTeacherOp::Add(new_id, teacher) => {
                if self.inner_data.params.teachers.teacher_map.contains(new_id) {
                    return Err(TeacherError::TeacherIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_teacher(teacher)?;

                self.inner_data
                    .params
                    .teachers
                    .teacher_map
                    .insert(*new_id, teacher.clone());

                Ok(AnnotatedTeacherOp::Remove(*new_id))
            }
            AnnotatedTeacherOp::Remove(id) => {
                if !self.inner_data.params.teachers.teacher_map.contains(id) {
                    return Err(TeacherError::InvalidTeacherId(*id));
                }

                for (slot_id, slot) in self.inner_data.params.slots.all_slots() {
                    if *id == slot.teacher_id {
                        return Err(TeacherError::TeacherStillHasAssociatedSlots(*id, *slot_id));
                    }
                }

                let old_teacher = self
                    .inner_data
                    .params
                    .teachers
                    .teacher_map
                    .remove(id)
                    .expect("Teacher ID was checked above");

                Ok(AnnotatedTeacherOp::Add(*id, old_teacher))
            }
            AnnotatedTeacherOp::Update(id, new_teacher) => {
                self.inner_data.params.validate_teacher(new_teacher)?;
                let Some(current_teacher) = self.inner_data.params.teachers.teacher_map.get_mut(id)
                else {
                    return Err(TeacherError::InvalidTeacherId(*id));
                };

                for subject_id in self.inner_data.params.slots.subjects_with_slots() {
                    if new_teacher.subjects.contains(&subject_id) {
                        continue;
                    }
                    for (_slot_id, slot) in self
                        .inner_data
                        .params
                        .slots
                        .slots_for_subject(subject_id)
                        .into_iter()
                        .flatten()
                    {
                        if *id == slot.teacher_id {
                            return Err(TeacherError::TeacherStillHasAssociatedSlotsInSubject(
                                *id, subject_id,
                            ));
                        }
                    }
                }

                let old_teacher = std::mem::replace(current_teacher, new_teacher.clone());

                Ok(AnnotatedTeacherOp::Update(*id, old_teacher))
            }
        }
    }

    /// Used internally by [crate::Data::force_apply]
    ///
    /// Thin copy of [Self::apply_teacher]: carve-out guards kept (returned as
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
