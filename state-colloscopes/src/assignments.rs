//! Assignments submodule
//!
//! This module defines the relevant types to describes the assignments

use std::collections::BTreeSet;

use thiserror::Error;

use crate::Table;
use crate::ids::{PeriodId, StudentId, SubjectId};
use crate::ops::AnnotatedAssignmentOp;

/// Description of the assignments
///
/// Assignments are stored as a sparse junction table keyed by
/// `(period, subject)`: a row is present exactly when at least one student is
/// assigned to that subject on that period. An absent row means nobody is
/// assigned (the canonical form — ops never leave an empty row behind).
/// Whether a subject *runs* on a period is not encoded here; consult
/// [`crate::subjects::Subject::excluded_periods`] instead. Canonical absence
/// is checked in `check_assignments_data_consistency`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Assignments {
    /// Attending students for each `(period, subject)` pair with ≥1 student
    pub map: Table<(PeriodId, SubjectId), BTreeSet<StudentId>>,
}

impl Assignments {
    /// Attending students for a `(period, subject)` pair, if the pair exists.
    pub fn students(&self, period: PeriodId, subject: SubjectId) -> Option<&BTreeSet<StudentId>> {
        self.map.get(&(period, subject))
    }

    /// Iterates over the `(period, subject, students)` entries, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (PeriodId, SubjectId, &BTreeSet<StudentId>)> {
        self.map
            .iter()
            .map(|((period, subject), students)| (period, subject, students))
    }

    /// Iterates over the `(subject, students)` entries for a period, in subject-id order.
    pub fn subjects_for_period(
        &self,
        period: PeriodId,
    ) -> impl Iterator<Item = (SubjectId, &BTreeSet<StudentId>)> {
        self.map
            .iter()
            .filter_map(move |((p, s), students)| (p == period).then_some((s, students)))
    }
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
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    return Err(AssignmentError::InvalidPeriodId(*period_id));
                }

                let Some(subject) = self.inner_data.params.subjects.find_subject(*subject_id)
                else {
                    return Err(AssignmentError::InvalidSubjectId(*subject_id));
                };

                // "Subject runs on period" is a property of the subject's
                // excluded-period set, not of the assignments key set: consult
                // it directly rather than probing for a `(period, subject)` row.
                if subject.excluded_periods.contains(period_id) {
                    return Err(AssignmentError::SubjectDoesNotRunOnPeriod(
                        *subject_id,
                        *period_id,
                    ));
                }

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

                // Sparse canonical form: a `(period, subject)` row exists iff
                // its student set is non-empty. Assigning creates the row on
                // demand; de-assigning drops the row once it empties.
                let map = &mut self.inner_data.params.assignments.map;
                let key = (*period_id, *subject_id);
                let previous_status = map.get(&key).is_some_and(|s| s.contains(student_id));

                if *status {
                    if let Some(assigned_students) = map.get_mut(&key) {
                        assigned_students.insert(*student_id);
                    } else {
                        map.insert(key, BTreeSet::from([*student_id]));
                    }
                } else if let Some(assigned_students) = map.get_mut(&key) {
                    assigned_students.remove(student_id);
                    if assigned_students.is_empty() {
                        map.remove(&key);
                    }
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
