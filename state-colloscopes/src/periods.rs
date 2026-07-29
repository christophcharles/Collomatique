//! Period submodule
//!
//! This module defines the relevant types to describes the periods

use thiserror::Error;

use collomatique_state::ContentOrd;
use collomatique_state::partial_order::option_lift_discrete;

use crate::OrderedTable;
use crate::ids::PeriodId;
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
/// un-ordered) is checked by the week-ordering `LogicError`s in
/// `InnerData::broken_invariants`.
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
pub struct Periods {
    /// Start date for the colloscope
    ///
    /// The date might not be set but of course, this will hinder
    /// the eventual pretty output
    // `WeekStart` is foreign, so it carries no `ContentOrd` impl of its own:
    // the helper lifts the plain `Option` rule over it (unset is below set,
    // two different dates are incomparable).
    #[ord(with = option_lift_discrete)]
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

/// Precondition errors of the forced period ops — the carve-out subset
/// (step-3 survey Table 2). Kept: no-clobber and op-target existence (Remove
/// target + `AddAfter` anchor both surface as [Self::InvalidPeriodId]). All
/// reference scans are stripped, including the empty-first `PeriodStillHasWeeks`
/// guard: force-removing a period with weeks leaves dangling `Week::period_id`
/// FKs for the cascade, exactly like every other stripped reference scan.
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
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a period op: carve-out guards kept (returned as
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

                // Periods are created week-less; weeks are spliced in by the
                // week ops afterwards.
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
                // stripped: the association-row cleanup the retired checked
                // apply_period carried. There it was dead code (the also-retired
                // PeriodStillHasNonTrivialGroupListAssociation guard rejected
                // the removal while any row existed); alive here it would
                // silently repair the would-be-dangling rows, landing a VALID
                // state on an op the gate must reject — and irreversibly, since
                // the reverse only re-adds the period. force_apply never fixes
                // anything: the dangling rows are the checker's to report.

                Ok(match previous_id {
                    None => AnnotatedPeriodOp::AddFront(*period_id),
                    Some(prev) => AnnotatedPeriodOp::AddAfter(*period_id, prev),
                })
            }
        }
    }
}
