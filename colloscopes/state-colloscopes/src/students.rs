//! Students submodule
//!
//! This module defines the relevant types to describes the students

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use thiserror::Error;

use collomatique_state::{ContentOrd, Join, References};

use crate::Table;
use crate::ids::{NewId, PeriodId, StudentId};
use crate::ops::AnnotatedStudentOp;

/// Description of the students
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
pub struct Students {
    /// List of students
    ///
    /// Each item associates an id to a student description
    pub student_map: Table<StudentId, Student>,
}

/// Description of a single student
#[derive(
    Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, References, Join, ContentOrd,
)]
#[join(error = NewId)]
pub struct Student {
    /// Description of the student in term of name and contact
    pub desc: crate::PersonWithContact,
    /// List of periods the student will not take part in
    #[fk]
    pub excluded_periods: BTreeSet<PeriodId>,
}

// The container's half of the dense renumbering walk (see [crate::compact]).
// The two methods must visit exactly the same id occurrences.
impl Students {
    pub(crate) fn collect_ids(&self, ids: &mut BTreeSet<u64>) {
        use crate::ids::Id as _;
        for (student_id, student) in self.student_map.iter() {
            ids.insert(student_id.inner());
            for period_id in &student.excluded_periods {
                ids.insert(period_id.inner());
            }
        }
    }

    pub(crate) fn remap_ids(self, map: &crate::compact::IdMap) -> Self {
        use crate::compact::remap;
        Students {
            student_map: self
                .student_map
                .into_iter()
                .map(|(student_id, student)| {
                    let Student {
                        desc,
                        excluded_periods,
                    } = student;
                    (
                        remap(map, student_id),
                        Student {
                            desc,
                            excluded_periods: excluded_periods
                                .into_iter()
                                .map(|period_id| remap(map, period_id))
                                .collect(),
                        },
                    )
                })
                .collect(),
        }
    }
}

/// Precondition errors of the forced student ops — the carve-out subset.
///
/// [crate::Data::force_apply] keeps only the transition/input guards
/// (no-clobber, op-target existence) and strips every invariant guard.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum StudentPrecheckError {
    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// The student id already exists
    #[error("student id ({0:?}) already exists")]
    StudentIdAlreadyExists(StudentId),
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a student op: carve-out guards kept (returned as
    /// [StudentPrecheckError]), invariant guards stripped. May leave the state
    /// invalid; the caller owns checking and rollback.
    pub(crate) fn force_apply_student(
        &mut self,
        student_op: &AnnotatedStudentOp,
    ) -> std::result::Result<AnnotatedStudentOp, StudentPrecheckError> {
        match student_op {
            AnnotatedStudentOp::Add(new_id, student) => {
                if self.inner_data.params.students.student_map.contains(new_id) {
                    return Err(StudentPrecheckError::StudentIdAlreadyExists(*new_id));
                }
                // stripped: validate_student

                self.inner_data
                    .params
                    .students
                    .student_map
                    .insert(*new_id, student.clone());

                Ok(AnnotatedStudentOp::Remove(*new_id))
            }
            AnnotatedStudentOp::Remove(id) => {
                // stripped: colloscope-placement / group-list / assignments / settings scans
                let Some(old_student) = self.inner_data.params.students.student_map.remove(id)
                else {
                    return Err(StudentPrecheckError::InvalidStudentId(*id));
                };

                Ok(AnnotatedStudentOp::Add(*id, old_student))
            }
            AnnotatedStudentOp::Update(id, new_student) => {
                // stripped: validate_student + newly-excluded-period assignment scan
                let Some(current_student) = self.inner_data.params.students.student_map.get_mut(id)
                else {
                    return Err(StudentPrecheckError::InvalidStudentId(*id));
                };

                let old_student = std::mem::replace(current_student, new_student.clone());

                Ok(AnnotatedStudentOp::Update(*id, old_student))
            }
        }
    }
}
