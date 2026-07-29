//! Assignments submodule
//!
//! This module defines the relevant types to describes the assignments

use std::collections::BTreeSet;

use thiserror::Error;

use collomatique_state::ContentOrd;

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
/// is checked by `LogicError::EmptyAssignmentsRow` in
/// `InnerData::broken_invariants`.
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
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

/// Precondition errors of the forced assignment op — the carve-out subset
/// (step-3 survey Table 2). The three coordinate-existence checks are
/// dual-listed (also invariant twins) and kept per Appendix D.3; the two
/// semantic guards (subject-runs / student-present) are stripped.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AssignmentPrecheckError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// A student id is invalid
    #[error("invalid student id ({0:?})")]
    InvalidStudentId(StudentId),
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies an assignment op: the three coordinate-existence
    /// checks are kept (dual-listed carve-outs, returned as
    /// [AssignmentPrecheckError]); the two semantic guards (subject-runs /
    /// student-present) are stripped (step-3 survey Table 1). Write-time
    /// canonicalization is copied verbatim. May leave the state invalid; the
    /// caller owns checking and rollback.
    pub(crate) fn force_apply_assignment(
        &mut self,
        assignment_op: &AnnotatedAssignmentOp,
    ) -> std::result::Result<AnnotatedAssignmentOp, AssignmentPrecheckError> {
        match assignment_op {
            AnnotatedAssignmentOp::SetRow(period_id, subject_id, students) => {
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_none()
                {
                    return Err(AssignmentPrecheckError::InvalidPeriodId(*period_id));
                }

                if self
                    .inner_data
                    .params
                    .subjects
                    .find_subject(*subject_id)
                    .is_none()
                {
                    return Err(AssignmentPrecheckError::InvalidSubjectId(*subject_id));
                }

                // stripped: SubjectDoesNotRunOnPeriod semantic guard

                // Every id in the incoming row must exist (coordinate carve-out).
                for student_id in students {
                    if !self
                        .inner_data
                        .params
                        .students
                        .student_map
                        .contains(student_id)
                    {
                        return Err(AssignmentPrecheckError::InvalidStudentId(*student_id));
                    }
                }

                // stripped: StudentIsNotPresentOnPeriod semantic guard

                // Sparse canonical form: a `(period, subject)` row exists iff
                // its student set is non-empty. An empty incoming set removes
                // the row; a non-empty one replaces it wholesale.
                let map = &mut self.inner_data.params.assignments.map;
                let key = (*period_id, *subject_id);
                let previous_row = map.get(&key).cloned().unwrap_or_default();

                if students.is_empty() {
                    map.remove(&key);
                } else {
                    map.insert(key, students.clone());
                }

                Ok(AnnotatedAssignmentOp::SetRow(
                    *period_id,
                    *subject_id,
                    previous_row,
                ))
            }
        }
    }
}
