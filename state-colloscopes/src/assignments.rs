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

// The container's half of the dense renumbering walk (see [crate::compact]).
// The two methods must visit exactly the same id occurrences — here both
// components of the composite key and every assigned student.
impl Assignments {
    pub(crate) fn collect_ids(&self, ids: &mut BTreeSet<u64>) {
        use crate::ids::Id as _;
        for ((period_id, subject_id), students) in self.map.iter() {
            ids.insert(period_id.inner());
            ids.insert(subject_id.inner());
            for student_id in students {
                ids.insert(student_id.inner());
            }
        }
    }

    pub(crate) fn remap_ids(self, map: &crate::compact::IdMap) -> Self {
        use crate::compact::remap;
        Assignments {
            map: self
                .map
                .into_iter()
                .map(|((period_id, subject_id), students)| {
                    (
                        (remap(map, period_id), remap(map, subject_id)),
                        students
                            .into_iter()
                            .map(|student_id| remap(map, student_id))
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

/// Precondition errors of the forced assignment op — the carve-out subset
/// (step-3 survey Table 2, as revised by the pre-step-7 review).
///
/// The two *address* checks (the row's `(period, subject)` key) are kept: with
/// an empty payload `SetRow` clears the row, so nothing lands in the document
/// and the dangling-FK net has no material to see — a dead key is the one case
/// it is structurally blind to.
///
/// The payload-student sweep and the two semantic guards (subject-runs /
/// student-present) are stripped: they are op *content*, owned by the checker
/// ([crate::FixableInvariant::DanglingFk] at
/// [crate::StudentRefSite::AssignmentsStudent],
/// [crate::Convergence::AssignmentForSubjectNotRunningOnPeriod],
/// [crate::Convergence::AssignedStudentNotPresentForPeriod]).
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum AssignmentPrecheckError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// A subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies an assignment op: the two *address* checks (the row's
    /// `(period, subject)` key) are kept, returned as
    /// [AssignmentPrecheckError]. The payload-student sweep and the two
    /// semantic guards (subject-runs / student-present) are stripped — they are
    /// op content, owned by the checker (see [AssignmentPrecheckError]).
    /// Write-time canonicalization is copied verbatim. May leave the state
    /// invalid; the caller owns checking and rollback.
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

                // stripped: payload-student existence sweep — the students are
                // op *content*, owned by the FK net (`DanglingFk @
                // StudentRefSite::AssignmentsStudent`); only the address
                // (period, subject) is prechecked.

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
