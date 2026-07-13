//! Assignments submodule
//!
//! This module defines the relevant types to describes the assignments

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use thiserror::Error;

use crate::ids::{PeriodId, StudentId, SubjectId};
use crate::ops::AnnotatedAssignmentOp;

/// Description of the assignments
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Assignments {
    /// Assignments for each period
    ///
    /// Each item associates a period id to an assignment description
    /// There should be an entry for each valid period
    pub period_map: BTreeMap<PeriodId, PeriodAssignments>,
}

/// Description of an assignment for a period
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodAssignments {
    /// Assignments for each student on the period
    ///
    /// Each item associates a subject id to an assignment description
    /// There should be an entry for each valid subject in the period
    /// The set is the list of students who do attend during the period
    pub subject_map: BTreeMap<SubjectId, BTreeSet<StudentId>>,
}

/// Errors for assignment operations
///
/// These errors can be returned when trying to modify [crate::Data] with a assignment op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AssignmentError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),

    /// Subject does not run on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    SubjectDoesNotRunOnPeriod(SubjectId, PeriodId),

    /// Student is not present on given period
    #[error("invalid subject id {0:?} for period {1:?}")]
    StudentIsNotPresentOnPeriod(StudentId, PeriodId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply assignment operations
    pub(crate) fn apply_assignment(
        &mut self,
        assignment_op: &AnnotatedAssignmentOp,
    ) -> std::result::Result<AnnotatedAssignmentOp, AssignmentError> {
        match assignment_op {
            AnnotatedAssignmentOp::Assign(period_id, student_id, subject_id, status) => {
                let Some(period_assignments) = self
                    .inner_data
                    .params
                    .assignments
                    .period_map
                    .get_mut(period_id)
                else {
                    return Err(AssignmentError::InvalidPeriodId(*period_id));
                };

                if self
                    .inner_data
                    .params
                    .subjects
                    .find_subject_position(*subject_id)
                    .is_none()
                {
                    return Err(AssignmentError::InvalidSubjectId(*subject_id));
                }

                let Some(assigned_students) = period_assignments.subject_map.get_mut(subject_id)
                else {
                    return Err(AssignmentError::SubjectDoesNotRunOnPeriod(
                        *subject_id,
                        *period_id,
                    ));
                };

                let Some(student_desc) =
                    self.inner_data.params.students.student_map.get(student_id)
                else {
                    return Err(AssignmentError::InvalidStudentId(*student_id));
                };

                if student_desc.excluded_periods.contains(period_id) {
                    return Err(AssignmentError::StudentIsNotPresentOnPeriod(
                        *student_id,
                        *period_id,
                    ));
                }

                let previous_status = assigned_students.contains(student_id);

                if *status {
                    assigned_students.insert(*student_id);
                } else {
                    assigned_students.remove(student_id);
                }

                Ok(AnnotatedAssignmentOp::Assign(
                    *period_id,
                    *student_id,
                    *subject_id,
                    previous_status,
                ))
            }
        }
    }
}
