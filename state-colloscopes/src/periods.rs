//! Period submodule
//!
//! This module defines the relevant types to describes the periods

use thiserror::Error;

use crate::OrderedTable;
use crate::ids::{PairingRuleId, PeriodId, SlotPairingRuleId, StudentId, SubjectId};
use crate::ops::AnnotatedPeriodOp;

/// Description of the periods
///
/// A period owns *existence and display order* only: `ordered_period_list` is
/// the public ordered set of period ids (mirroring `Subjects.ordered_subject_list`),
/// each mapping to `()` — a period carries no other data of its own.
///
/// Weeks and their per-period ordering live in the sibling [crate::weeks::Weeks]
/// container, a sibling field on [crate::Parameters].
///
/// The cross-container consistency (every `ordering` row names a live period,
/// the row is non-empty, every week names its period, no week is left
/// un-ordered) is checked in `Parameters::check_weeks_data_consistency`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Periods {
    /// Start date for the colloscope
    ///
    /// The date might not be set but of course, this will hinder
    /// the eventual pretty output
    pub first_week: Option<collomatique_time::WeekStart>,

    /// Ordered set of period ids — existence and display order only
    ///
    /// A period owns nothing else; week data and per-period week ordering live
    /// in the sibling [crate::weeks::Weeks]. Public, mirroring
    /// `Subjects.ordered_subject_list`.
    pub ordered_period_list: OrderedTable<PeriodId, ()>,
}

/// Error returned when building [Periods] from ordered ids with a duplicate
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("duplicated period id {0:?}")]
pub struct DuplicatedPeriodIdError(pub PeriodId);

impl Periods {
    /// Builds a [Periods] from an ordered list of period ids (no weeks).
    ///
    /// The ids define the display order; the resulting periods have no weeks.
    /// Returns an error on a duplicate id. This is the constructor storage
    /// decode uses; the sibling [`crate::weeks::Weeks`] container is built
    /// separately with [`crate::weeks::Weeks::from_period_rows`].
    pub fn from_ordered_ids(
        first_week: Option<collomatique_time::WeekStart>,
        ids: Vec<PeriodId>,
    ) -> Result<Self, DuplicatedPeriodIdError> {
        let period_rows: Vec<(PeriodId, ())> = ids.into_iter().map(|id| (id, ())).collect();
        let ordered_period_list = period_rows.try_into().map_err(
            |collomatique_state::tables::DuplicatedIdError(id)| DuplicatedPeriodIdError(id),
        )?;
        Ok(Periods {
            first_week,
            ordered_period_list,
        })
    }

    // ---- Read surface ----
    //
    // These methods are the sanctioned way to read the periods. Consumers go
    // through them rather than the private `ordered_period_list` field. Week
    // data is read from the sibling [`crate::weeks::Weeks`] container.

    /// Period ids in display order.
    pub fn period_ids(&self) -> impl Iterator<Item = PeriodId> + '_ {
        self.ordered_period_list.keys()
    }

    /// Number of periods.
    pub fn period_count(&self) -> usize {
        self.ordered_period_list.len()
    }

    /// Whether there are no periods at all.
    pub fn is_empty(&self) -> bool {
        self.ordered_period_list.is_empty()
    }

    /// The period id at the given display position, if any.
    pub fn period_id_at(&self, pos: usize) -> Option<PeriodId> {
        self.ordered_period_list.get_at(pos).map(|(id, _)| id)
    }

    /// Finds the position of a period by id
    pub fn find_period_position(&self, id: PeriodId) -> Option<usize> {
        self.ordered_period_list.position_of(&id)
    }
}

/// Errors for periods operations
///
/// These errors can be returned when trying to modify [crate::Data] with a period op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PeriodError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// The period id already exists
    #[error("period id ({0:?}) already exists")]
    PeriodIdAlreadyExists(PeriodId),

    /// The period is referenced by a subject
    #[error("period id ({0:?}) is referenced by subject {1:?}")]
    PeriodIsReferencedBySubject(PeriodId, SubjectId),

    /// The period is referenced by a student
    #[error("period id ({0:?}) is referenced by student {1:?}")]
    PeriodIsReferencedByStudent(PeriodId, StudentId),

    /// Some non-default assignments are still present for the period
    #[error(
        "period id ({0:?}) has non-default assignments for subject id {1:?} and cannot be removed"
    )]
    PeriodStillHasNonTrivialAssignments(PeriodId, SubjectId),

    /// Some non-default group list association are still present for the period
    #[error("period id ({0:?}) has non-default group list associations and cannot be removed")]
    PeriodStillHasNonTrivialGroupListAssociation(PeriodId),

    /// Period is not empty in colloscope
    #[error("period id ({0:?}) is not empty in colloscope")]
    NotEmptyPeriodInColloscope(PeriodId),

    /// The period still has weeks and cannot be removed
    #[error("period id ({0:?}) still has weeks and cannot be removed")]
    PeriodStillHasWeeks(PeriodId),

    /// The period is referenced by a pairing rule
    #[error("period id ({0:?}) is referenced by pairing rule {1:?}")]
    PeriodIsReferencedByPairingRule(PeriodId, PairingRuleId),

    /// The period is referenced by a slot pairing rule
    #[error("period id ({0:?}) is referenced by slot pairing rule {1:?}")]
    PeriodIsReferencedBySlotPairingRule(PeriodId, SlotPairingRuleId),
}

/// Precondition errors of the forced period ops — the carve-out subset
/// (step-3 survey Table 2). Kept: no-clobber and op-target existence (Remove
/// target + `AddAfter` anchor both surface as [Self::InvalidPeriodId]). All
/// reference scans are stripped, including the empty-first `PeriodStillHasWeeks`
/// guard: force-removing a period with weeks leaves dangling `Week::period_id`
/// FKs for the cascade, exactly like every other stripped reference scan.
/// Variants copied verbatim from [PeriodError].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PeriodPrecheckError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// The period id already exists
    #[error("period id ({0:?}) already exists")]
    PeriodIdAlreadyExists(PeriodId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply period operations
    pub(crate) fn apply_period(
        &mut self,
        period_op: &AnnotatedPeriodOp,
    ) -> std::result::Result<AnnotatedPeriodOp, PeriodError> {
        match period_op {
            AnnotatedPeriodOp::ChangeStartDate(new_date) => {
                let old_date = std::mem::replace(
                    &mut self.inner_data.params.periods.first_week,
                    new_date.clone(),
                );
                Ok(AnnotatedPeriodOp::ChangeStartDate(old_date))
            }
            AnnotatedPeriodOp::AddFront(period_id) => {
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(*period_id));
                }

                // Periods are created week-less: no pattern bits to splice, no
                // assignments and no associations (so neither sparse junction
                // table gets a row), and no colloscope rows. Weeks are added
                // afterwards through the WeekOp family.
                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .insert_at(0, *period_id, ())
                    .expect("period id absence checked above");
                Ok(AnnotatedPeriodOp::Remove(*period_id))
            }
            AnnotatedPeriodOp::AddAfter(period_id, after_id) => {
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodError::PeriodIdAlreadyExists(*period_id));
                }

                let Some(position) = self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*after_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*after_id));
                };

                // Created week-less (see `AddFront` above).
                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .insert_at(position + 1, *period_id, ())
                    .expect("period id absence checked above");
                Ok(AnnotatedPeriodOp::Remove(*period_id))
            }
            AnnotatedPeriodOp::Remove(period_id) => {
                let Some(position) = self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return Err(PeriodError::InvalidPeriodId(*period_id));
                };

                // A period must be emptied (via WeekOp::Remove) before it can be
                // removed: `apply_week` is the only writer of week data, so
                // period removal never has to unwind pattern bits or colloscope
                // cells — a week-empty period has none.
                let week_count = self
                    .inner_data
                    .params
                    .weeks
                    .week_count_for_period(*period_id)
                    .unwrap_or(0);
                if week_count != 0 {
                    return Err(PeriodError::PeriodStillHasWeeks(*period_id));
                }

                // Canonical-absent surface: any interrogation row on a week of
                // this period blocks removal. Vacuous once `PeriodStillHasWeeks`
                // passes (a week-empty period has no rows), but kept as
                // belt-and-suspenders.
                let has_colloscope_row =
                    self.inner_data
                        .colloscope
                        .iter()
                        .any(|((_slot_id, week), _groups)| {
                            self.inner_data
                                .params
                                .weeks
                                .week_position(week)
                                .map(|(p, _pos)| p)
                                == Some(*period_id)
                        });
                if has_colloscope_row {
                    return Err(PeriodError::NotEmptyPeriodInColloscope(*period_id));
                }

                for (subject_id, subject) in
                    self.inner_data.params.subjects.ordered_subject_list.iter()
                {
                    if subject.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedBySubject(
                            *period_id, subject_id,
                        ));
                    }
                }

                for (student_id, student) in self.inner_data.params.students.student_map.iter() {
                    if student.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedByStudent(
                            *period_id, student_id,
                        ));
                    }
                }

                for (rule_id, rule) in self.inner_data.params.pairings.pairing_rule_map.iter() {
                    if rule.excluded_periods().contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedByPairingRule(
                            *period_id, rule_id,
                        ));
                    }
                }

                for (rule_id, rule) in self
                    .inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .iter()
                {
                    if rule.excluded_periods.contains(period_id) {
                        return Err(PeriodError::PeriodIsReferencedBySlotPairingRule(
                            *period_id, rule_id,
                        ));
                    }
                }

                // Under canonical-absent, a row exists iff it is non-trivial,
                // so any surviving row for this period blocks the removal.
                if let Some((subject_id, _)) = self
                    .inner_data
                    .params
                    .assignments
                    .subjects_for_period(*period_id)
                    .next()
                {
                    return Err(PeriodError::PeriodStillHasNonTrivialAssignments(
                        *period_id, subject_id,
                    ));
                }

                if self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .keys()
                    .any(|(p, _)| p == *period_id)
                {
                    return Err(PeriodError::PeriodStillHasNonTrivialGroupListAssociation(
                        *period_id,
                    ));
                }

                let previous_id = (position > 0).then(|| {
                    self.inner_data
                        .params
                        .periods
                        .ordered_period_list
                        .get_at(position - 1)
                        .expect("position > 0 checked")
                        .0
                });

                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .remove_at(position);
                // Drop this period's rows from the associations table (none
                // remain once the emptiness check passes, but stay consistent
                // regardless). Assignment rows are already gone: the guard
                // above rejects the removal while any survive. No week-pattern
                // bits to unwind either: a week-empty period contributes none.
                let association_keys: Vec<_> = self
                    .inner_data
                    .params
                    .group_lists
                    .subjects_associations
                    .keys()
                    .filter(|(p, _)| *p == *period_id)
                    .collect();
                for key in association_keys {
                    self.inner_data
                        .params
                        .group_lists
                        .subjects_associations
                        .remove(&key);
                }

                Ok(match previous_id {
                    None => AnnotatedPeriodOp::AddFront(*period_id),
                    Some(prev) => AnnotatedPeriodOp::AddAfter(*period_id, prev),
                })
            }
        }
    }

    /// Used internally by [crate::Data::force_apply]
    ///
    /// Thin copy of [Self::apply_period]: carve-out guards kept (returned as
    /// [PeriodPrecheckError] — no-clobber, target existence, `AddAfter` anchor),
    /// invariant guards stripped (step-3 survey Table 1), including the
    /// empty-first `PeriodStillHasWeeks` guard — force-removing a period with
    /// weeks leaves dangling `Week::period_id` FKs. May leave the state invalid;
    /// the caller owns checking and rollback.
    pub(crate) fn force_apply_period(
        &mut self,
        period_op: &AnnotatedPeriodOp,
    ) -> std::result::Result<AnnotatedPeriodOp, PeriodPrecheckError> {
        match period_op {
            AnnotatedPeriodOp::ChangeStartDate(new_date) => {
                let old_date = std::mem::replace(
                    &mut self.inner_data.params.periods.first_week,
                    new_date.clone(),
                );
                Ok(AnnotatedPeriodOp::ChangeStartDate(old_date))
            }
            AnnotatedPeriodOp::AddFront(period_id) => {
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodPrecheckError::PeriodIdAlreadyExists(*period_id));
                }

                // Periods are created week-less (see [Self::apply_period]).
                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .insert_at(0, *period_id, ())
                    .expect("period id absence checked above");
                Ok(AnnotatedPeriodOp::Remove(*period_id))
            }
            AnnotatedPeriodOp::AddAfter(period_id, after_id) => {
                if self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                    .is_some()
                {
                    return Err(PeriodPrecheckError::PeriodIdAlreadyExists(*period_id));
                }

                let Some(position) = self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*after_id)
                else {
                    return Err(PeriodPrecheckError::InvalidPeriodId(*after_id));
                };

                // Created week-less (see `AddFront` above).
                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .insert_at(position + 1, *period_id, ())
                    .expect("period id absence checked above");
                Ok(AnnotatedPeriodOp::Remove(*period_id))
            }
            AnnotatedPeriodOp::Remove(period_id) => {
                let Some(position) = self
                    .inner_data
                    .params
                    .periods
                    .find_period_position(*period_id)
                else {
                    return Err(PeriodPrecheckError::InvalidPeriodId(*period_id));
                };

                // stripped: the empty-first `PeriodStillHasWeeks` guard (a
                // period with weeks now removes, leaving dangling
                // `Week::period_id` FKs — the ordering sidecar row and
                // `week_map` entries are untouched, since force_apply fixes
                // nothing) and the colloscope / subject / student / pairing /
                // slot-pairing / assignment / group-list-association reference
                // scans

                let previous_id = (position > 0).then(|| {
                    self.inner_data
                        .params
                        .periods
                        .ordered_period_list
                        .get_at(position - 1)
                        .expect("position > 0 checked")
                        .0
                });

                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .remove_at(position);
                // stripped: the association-row cleanup of [Self::apply_period].
                // There it is dead code (the stripped
                // PeriodStillHasNonTrivialGroupListAssociation guard rejects the
                // removal while any row exists); alive here it would silently
                // repair the would-be-dangling rows, landing a VALID state on an
                // op the checked apply rejects — and irreversibly, since the
                // reverse only re-adds the period. force_apply never fixes
                // anything: the dangling rows are the checker's to report.

                Ok(match previous_id {
                    None => AnnotatedPeriodOp::AddFront(*period_id),
                    Some(prev) => AnnotatedPeriodOp::AddAfter(*period_id, prev),
                })
            }
        }
    }
}
