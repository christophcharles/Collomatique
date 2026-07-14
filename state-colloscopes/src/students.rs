//! Students submodule
//!
//! This module defines the relevant types to describes the students

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use thiserror::Error;

use crate::Table;
use crate::ids::{GroupListId, PeriodId, StudentId, SubjectId};
use crate::ops::AnnotatedStudentOp;

/// Description of the students
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Students {
    /// List of students
    ///
    /// Each item associates an id to a student description
    pub student_map: Table<StudentId, Student>,
}

/// Description of a single student
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Student {
    /// Description of the student in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of periods the student will not take part in
    pub excluded_periods: BTreeSet<PeriodId>,
}

/// Errors for students operations
///
/// These errors can be returned when trying to modify [crate::Data] with a student op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StudentError {
    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// The student id already exists
    #[error("student id ({0:?}) already exists")]
    StudentIdAlreadyExists(StudentId),

    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// Some non-default assignments are still present for the student
    #[error(
        "student id {0:?} has non-default assignments for subject id {1:?} in period id ({0:?}) and cannot be removed or updated"
    )]
    StudentStillHasNonTrivialAssignments(StudentId, SubjectId, PeriodId),

    /// Student is still excluded by a group list
    #[error("student id {0:?} is still excluded by a group list {1:?}")]
    StudentIsStillExcludedByGroupList(StudentId, GroupListId),

    /// Student is still referenced by a pre-filled group list
    #[error("student id {0:?} is still referenced by a pre-filled group list {1:?}")]
    StudentIsStillReferencedByPrefilledGroupList(StudentId, GroupListId),

    /// Student is referenced in a colloscope group list
    #[error("student id {0:?} is referenced in a colloscope group list ({1:?})")]
    StudentIsReferencedInColloscopeGroupList(StudentId, GroupListId),

    /// Student still has per-student settings
    #[error("student id {0:?} still has per-student settings")]
    StudentStillHasSettings(StudentId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply student operations
    pub(crate) fn apply_student(
        &mut self,
        student_op: &AnnotatedStudentOp,
    ) -> std::result::Result<AnnotatedStudentOp, StudentError> {
        match student_op {
            AnnotatedStudentOp::Add(new_id, student) => {
                if self
                    .inner_data
                    .params
                    .students
                    .student_map
                    .contains_key(new_id)
                {
                    return Err(StudentError::StudentIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_student(student)?;

                self.inner_data
                    .params
                    .students
                    .student_map
                    .insert(*new_id, student.clone());

                Ok(AnnotatedStudentOp::Remove(*new_id))
            }
            AnnotatedStudentOp::Remove(id) => {
                let Some(current_student) = self.inner_data.params.students.student_map.get(id)
                else {
                    return Err(StudentError::InvalidStudentId(*id));
                };

                for (group_list_id, group_list) in &self.inner_data.colloscope.group_lists {
                    if group_list.groups_for_students.contains_key(id) {
                        return Err(StudentError::StudentIsReferencedInColloscopeGroupList(
                            *id,
                            *group_list_id,
                        ));
                    }
                }

                for (group_list_id, group_list) in
                    self.inner_data.params.group_lists.group_list_map.iter()
                {
                    if group_list.filling.excluded_students().contains(id) {
                        return Err(StudentError::StudentIsStillExcludedByGroupList(
                            *id,
                            group_list_id,
                        ));
                    }
                    if group_list.filling.contains_student(*id) {
                        return Err(StudentError::StudentIsStillReferencedByPrefilledGroupList(
                            *id,
                            group_list_id,
                        ));
                    }
                }

                for (period_id, period_assignments) in
                    &self.inner_data.params.assignments.period_map
                {
                    if current_student.excluded_periods.contains(period_id) {
                        continue;
                    }
                    for (subject_id, assigned_students) in &period_assignments.subject_map {
                        if assigned_students.contains(id) {
                            return Err(StudentError::StudentStillHasNonTrivialAssignments(
                                *id,
                                *subject_id,
                                *period_id,
                            ));
                        }
                    }
                }

                if self.inner_data.params.settings.students.contains_key(id) {
                    return Err(StudentError::StudentStillHasSettings(*id));
                }

                let old_student = self
                    .inner_data
                    .params
                    .students
                    .student_map
                    .remove(id)
                    .expect("Student ID was checked above");

                Ok(AnnotatedStudentOp::Add(*id, old_student))
            }
            AnnotatedStudentOp::Update(id, new_student) => {
                self.inner_data.params.validate_student(new_student)?;
                let Some(current_student) = self.inner_data.params.students.student_map.get_mut(id)
                else {
                    return Err(StudentError::InvalidStudentId(*id));
                };

                for (period_id, period_assignments) in
                    &self.inner_data.params.assignments.period_map
                {
                    if current_student.excluded_periods.contains(period_id)
                        || !new_student.excluded_periods.contains(period_id)
                    {
                        continue;
                    }
                    for (subject_id, assigned_students) in &period_assignments.subject_map {
                        if assigned_students.contains(id) {
                            return Err(StudentError::StudentStillHasNonTrivialAssignments(
                                *id,
                                *subject_id,
                                *period_id,
                            ));
                        }
                    }
                }

                let old_student = std::mem::replace(current_student, new_student.clone());

                Ok(AnnotatedStudentOp::Update(*id, old_student))
            }
        }
    }
}
