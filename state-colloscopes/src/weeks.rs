//! Weeks submodule
//!
//! This module defines the relevant types to describe the weeks and their
//! per-period ordering — the twin of [crate::slots].

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::partial_order::option_lift_discrete;
use collomatique_state::{ContentOrd, Join, References};

use crate::Table;
use crate::ids::{NewId, PeriodId, WeekId};
use crate::ops::AnnotatedWeekOp;
use crate::periods::Periods;

/// Description of the weeks
///
/// The backend is a flat id-keyed [Table] of weeks (each week carries its
/// owning period as a foreign key) plus an explicit per-period ordering
/// sidecar. The two must stay consistent; the invariant is checked by the
/// week-ordering `LogicError`s in `InnerData::broken_invariants`:
/// - `ordering` is sparse: a row is present exactly when the period has at
///   least one week (canonical form — no empty rows), and that period exists,
///   and
/// - `ordering[p]` is a duplicate-free permutation of
///   `{ id | week_map[id].period_id == p }`.
///
/// All mutation goes through the compound `pub(crate)` helpers below so no
/// call site can desynchronize the two structures. The fields are private:
/// consumers read through the accessor surface further down. The cross-container
/// readers take the sibling [Periods] as an explicit parameter (the
/// `WeekPatterns::is_week_active(&Weeks, …)` precedent) since the period display
/// order lives there.
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
pub struct Weeks {
    /// Every week, keyed by its id
    ///
    /// Each week carries its owning period as a foreign key; the `ordering`
    /// sidecar groups those same week ids under that period.
    week_map: Table<WeekId, Week>,

    /// Per-period ordered week ids
    ///
    /// Sparse: a row exists exactly when the period has at least one week
    /// (canonical form — no empty rows), mirroring the slots ordering. A
    /// week-empty period has no row. The row key has double duty with each
    /// week's `period_id` foreign key, so it needs no separate reference edge.
    ordering: Table<PeriodId, Vec<WeekId>>,
}

/// Error returned when building [Weeks] from rows with a duplicated week id
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("duplicated week id {0:?}")]
pub struct DuplicatedWeekIdError(pub WeekId);

/// Description of a single week
///
/// This is the stored week entity: it carries its owning period as a foreign
/// key plus whether an interrogation happens on it and an optional annotation.
/// The period-less, id-less [WeekDesc] is the matching op-payload / DTO form.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join, ContentOrd)]
#[join(error = NewId)]
pub struct Week {
    /// Period this week belongs to
    ///
    /// This is authoritative: the week is grouped under this period in the
    /// ordering sidecar.
    #[fk(name = period)]
    pub period_id: PeriodId,
    /// Whether an interrogation happens on this week
    pub interrogations: bool,
    /// Optional annotation (e.g. "Rentrée", "Vacances")
    // `NonEmptyString` is foreign, so the helper supplies the `Option` rule:
    // clearing the annotation is removing content, and two different
    // annotations are incomparable.
    #[ord(with = option_lift_discrete)]
    pub annotation: Option<non_empty_string::NonEmptyString>,
}

impl Week {
    /// Builds a week entity from its owning period and a description.
    pub(crate) fn from_desc(period_id: PeriodId, desc: WeekDesc) -> Week {
        Week {
            period_id,
            interrogations: desc.interrogations,
            annotation: desc.annotation,
        }
    }

    /// The period-less, id-less description of this week (op-payload / DTO form).
    pub fn desc(&self) -> WeekDesc {
        WeekDesc {
            interrogations: self.interrogations,
            annotation: self.annotation.clone(),
        }
    }
}

/// Period-less, id-less description of a week
///
/// This is the op-payload / DTO counterpart of the stored [Week] entity: it
/// carries only the mutable payload (whether an interrogation happens and the
/// annotation), used by the week ops, gtk4 dialogs and python glue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekDesc {
    pub interrogations: bool,
    pub annotation: Option<non_empty_string::NonEmptyString>,
}

impl Default for WeekDesc {
    fn default() -> Self {
        WeekDesc {
            interrogations: true,
            annotation: None,
        }
    }
}

impl WeekDesc {
    pub fn new(interrogations: bool) -> WeekDesc {
        WeekDesc {
            interrogations,
            annotation: None,
        }
    }
}

impl Weeks {
    /// Builds a [Weeks] from per-period rows (used by storage decode).
    ///
    /// `rows` provides one row per period that has weeks: the period id and its
    /// ordered weeks (identity paired with description, in the intended order).
    /// Empty rows are dropped so the sparse canonical form (no empty ordering
    /// entry) is preserved. Returns an error if a week id appears more than once
    /// across all rows — otherwise the two backend structures would silently
    /// desynchronize. The owning period fk is derived from the row key.
    pub fn from_period_rows(
        rows: impl IntoIterator<Item = (PeriodId, Vec<(WeekId, WeekDesc)>)>,
    ) -> Result<Self, DuplicatedWeekIdError> {
        let mut week_map = Table::new();
        let mut ordering = Table::new();
        for (period_id, weeks) in rows {
            if weeks.is_empty() {
                // Canonical sparse form: a week-empty period gets no row.
                continue;
            }
            let mut order = Vec::with_capacity(weeks.len());
            for (week_id, desc) in weeks {
                if week_map
                    .insert(week_id, Week::from_desc(period_id, desc))
                    .is_some()
                {
                    return Err(DuplicatedWeekIdError(week_id));
                }
                order.push(week_id);
            }
            ordering.insert(period_id, order);
        }
        Ok(Weeks { week_map, ordering })
    }

    /// Test-only corruption: inserts an ordering row verbatim, bypassing the
    /// canonical-sparse discipline of [Self::from_period_rows] (which drops
    /// empty rows) — a stored empty row is exactly the
    /// [crate::invariants::LogicError::EmptyWeeksRow] the invariant checker must
    /// detect, and no production surface can produce it. Twin of
    /// [`crate::slots::Slots::forge_ordering_row`].
    #[cfg(test)]
    pub(crate) fn forge_ordering_row(&mut self, period: PeriodId, order: Vec<WeekId>) {
        self.ordering.insert(period, order);
    }

    // ---- Read surface ----
    //
    // These methods are the sanctioned way to read the weeks. Consumers go
    // through them rather than the private `week_map` / `ordering` fields.

    /// The ordered week ids of a period, defaulting to an empty slice when the
    /// period has no ordering row (a week-empty or invalid period).
    fn week_order(&self, id: PeriodId) -> &[WeekId] {
        self.ordering.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The canonical global week order: every week of every period, in
    /// period-then-position order, each with its identity. `walk(periods)
    /// .enumerate()` gives the global week index — this replaces every
    /// hand-rolled accumulate-`len()` loop. The period display order comes from
    /// the sibling [Periods].
    ///
    /// `periods` must be the sibling container of the same state. In a valid
    /// state the invariants guarantee every week is visited exactly once, so
    /// the walk total equals [`Self::count_weeks`]; a dangling ordering row
    /// (broken state — the checker's concern) is simply not walked.
    pub fn walk<'a>(
        &'a self,
        periods: &'a Periods,
    ) -> impl Iterator<Item = (PeriodId, WeekId, &'a Week)> + 'a {
        periods
            .ordered_period_list
            .keys()
            .flat_map(move |period_id| {
                self.week_order(period_id).iter().map(move |week_id| {
                    let week = self
                        .week_map
                        .get(week_id)
                        .expect("ordering id should be present in week_map");
                    (period_id, *week_id, week)
                })
            })
    }

    /// All week ids, in global week order (period display order from [Periods]).
    pub fn week_ids<'a>(&'a self, periods: &'a Periods) -> impl Iterator<Item = WeekId> + 'a {
        periods
            .ordered_period_list
            .keys()
            .flat_map(move |period_id| self.week_order(period_id).iter().copied())
    }

    /// Weeks of one period, in order (identity paired with entity); `None` if
    /// the period has no weeks (no ordering row). Twin of
    /// [`crate::slots::Slots::slots_for_subject`].
    pub fn weeks_for_period(
        &self,
        period_id: PeriodId,
    ) -> Option<impl Iterator<Item = (&WeekId, &Week)>> {
        let order = self.ordering.get(&period_id)?;
        Some(order.iter().map(move |week_id| {
            let week = self
                .week_map
                .get(week_id)
                .expect("ordering id should be present in week_map");
            (week_id, week)
        }))
    }

    /// Owned copy of a period's weeks — descriptions only, ids and owning period
    /// stripped (op-payload building in `ops/` and gtk4); `None` if the period
    /// has no weeks (no ordering row).
    pub fn weeks_desc_vec_for_period(&self, period_id: PeriodId) -> Option<Vec<WeekDesc>> {
        let order = self.ordering.get(&period_id)?;
        Some(
            order
                .iter()
                .map(|week_id| {
                    self.week_map
                        .get(week_id)
                        .expect("ordering id should be present in week_map")
                        .desc()
                })
                .collect(),
        )
    }

    /// Number of weeks of one period; `None` if the period has no weeks (no
    /// ordering row). Twin of [`crate::slots::Slots::slot_count_for_subject`].
    pub fn week_count_for_period(&self, period_id: PeriodId) -> Option<usize> {
        self.ordering.get(&period_id).map(|order| order.len())
    }

    /// Whether no period has any weeks.
    ///
    /// Reads the week table: the compound mutators keep it in lockstep with
    /// `ordering`, so the two containers cover the same week ids in every
    /// ops-reachable state (force ops included); only test forgery can split
    /// them.
    pub fn is_empty(&self) -> bool {
        self.week_map.is_empty()
    }

    /// Total number of weeks across all periods.
    ///
    /// Reads the week table; by the same lockstep argument as
    /// [`Self::is_empty`] this equals summing the `ordering` rows, and — in a
    /// valid state — the [`Self::walk`] total.
    pub fn count_weeks(&self) -> usize {
        self.week_map.len()
    }

    /// Finds a week by id, returning the stored [Week] entity (owning period
    /// available through [`Week::period_id`]). `None` if the week id is invalid.
    pub fn find_week(&self, id: WeekId) -> Option<&Week> {
        self.week_map.get(&id)
    }

    /// Locates a week by id: its owning period and its position within that
    /// period. `None` if the week id is invalid.
    pub fn week_position(&self, id: WeekId) -> Option<(PeriodId, usize)> {
        let period_id = self.week_map.get(&id)?.period_id;
        let pos = self
            .ordering
            .get(&period_id)
            .expect("week's period should have an ordering row")
            .iter()
            .position(|week_id| *week_id == id)
            .expect("week should appear in its period's ordering");
        Some((period_id, pos))
    }

    /// The id of the week at position `pos` within `period`; `None` if the
    /// period has no such week (no row, or position out of range).
    pub fn week_id_at(&self, period: PeriodId, pos: usize) -> Option<WeekId> {
        self.week_order(period).get(pos).copied()
    }

    /// The global week position of a week (its index in `walk()` order);
    /// `None` if the week id is invalid.
    pub fn global_week_position(&self, periods: &Periods, id: WeekId) -> Option<usize> {
        self.walk(periods).position(|(_, week_id, _)| week_id == id)
    }

    /// Finds the position of a period by id and gives the number of the first
    /// week (the period display order comes from the sibling [Periods]).
    pub fn find_period_position_and_first_week(
        &self,
        periods: &Periods,
        id: PeriodId,
    ) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for (pos, period_id) in periods.ordered_period_list.keys().enumerate() {
            if period_id == id {
                return Some((pos, first_week));
            }
            first_week += self.week_order(period_id).len();
        }

        None
    }

    // ---- Internal accessors (checker / reference registry) ----

    /// USED INTERNALLY
    ///
    /// Iterator over every `(week id, week)` entry, in id order, straight from
    /// the week table (independent of the ordering sidecar, so it is safe on
    /// potentially inconsistent data during invariant checking and drives the
    /// reference registry, which walks weeks in id order).
    pub(crate) fn week_entries(&self) -> impl Iterator<Item = (WeekId, &Week)> {
        self.week_map.iter()
    }

    /// USED INTERNALLY
    ///
    /// Raw view of the ordering sidecar (period → ordered week ids), for the
    /// consistency invariant check.
    pub(crate) fn ordering_entries(&self) -> impl Iterator<Item = (PeriodId, &[WeekId])> {
        self.ordering
            .iter()
            .map(|(id, order)| (id, order.as_slice()))
    }

    // ---- Compound week mutators ----
    //
    // Every week mutation goes through one of these so `ordering` and
    // `week_map` can never desynchronize.

    /// Inserts a week into `period_id`'s ordering at `pos` and into `week_map`.
    ///
    /// Under the sparse ordering the row is created on demand for the period's
    /// first week (which lands at position 0), mirroring
    /// [`crate::slots::Slots::insert_slot_at`].
    pub(crate) fn insert_week_at(
        &mut self,
        week_id: WeekId,
        period_id: PeriodId,
        pos: usize,
        desc: WeekDesc,
    ) {
        if let Some(order) = self.ordering.get_mut(&period_id) {
            order.insert(pos, week_id);
        } else {
            debug_assert_eq!(pos, 0, "first week of a period must land at position 0");
            self.ordering.insert(period_id, vec![week_id]);
        }
        self.week_map
            .insert(week_id, Week::from_desc(period_id, desc));
    }

    /// Removes a week, returning its former period, position and description.
    ///
    /// Dropping a period's last week removes its ordering row, keeping the
    /// sparse canonical form.
    pub(crate) fn remove_week_entry(&mut self, week_id: WeekId) -> (PeriodId, usize, WeekDesc) {
        let week = self.week_map.remove(&week_id).expect("week should exist");
        let order = self
            .ordering
            .get_mut(&week.period_id)
            .expect("week's period should have an ordering row");
        let pos = order
            .iter()
            .position(|id| *id == week_id)
            .expect("week should appear in its period's ordering");
        order.remove(pos);
        if order.is_empty() {
            self.ordering.remove(&week.period_id);
        }
        (week.period_id, pos, week.desc())
    }

    /// Moves a week to `dest_pos` in `dest_period`, returning its former
    /// `(period, position)`. The week keeps its id and description; only its
    /// owning period and its slot in the ordering change.
    ///
    /// Detaching from the source period drops that period's ordering row if it
    /// becomes empty; attaching to the destination creates its row on demand —
    /// keeping the sparse canonical form on both sides.
    pub(crate) fn move_week_entry(
        &mut self,
        week_id: WeekId,
        dest_period: PeriodId,
        dest_pos: usize,
    ) -> (PeriodId, usize) {
        let src_period = self
            .week_map
            .get(&week_id)
            .expect("week should exist")
            .period_id;
        let src_pos = {
            let order = self
                .ordering
                .get_mut(&src_period)
                .expect("week's period should have an ordering row");
            let pos = order
                .iter()
                .position(|id| *id == week_id)
                .expect("week should appear in its period's ordering");
            order.remove(pos);
            if order.is_empty() && src_period != dest_period {
                self.ordering.remove(&src_period);
            }
            pos
        };
        if let Some(order) = self.ordering.get_mut(&dest_period) {
            order.insert(dest_pos, week_id);
        } else {
            debug_assert_eq!(
                dest_pos, 0,
                "first week of a period must land at position 0"
            );
            self.ordering.insert(dest_period, vec![week_id]);
        }
        self.week_map
            .get_mut(&week_id)
            .expect("week should exist")
            .period_id = dest_period;
        (src_period, src_pos)
    }

    /// Replaces a week's description (owning period unchanged), returning the
    /// previous description.
    pub(crate) fn replace_week_desc(&mut self, week_id: WeekId, desc: WeekDesc) -> WeekDesc {
        let week = self.week_map.get_mut(&week_id).expect("week should exist");
        let old = week.desc();
        week.interrogations = desc.interrogations;
        week.annotation = desc.annotation;
        old
    }
}

/// Precondition errors of the forced week ops — the carve-out subset
/// (step-3 survey Table 2). Kept: no-clobber, op-target existence
/// ([Self::InvalidWeekId]), destination-period existence for add/move
/// ([Self::InvalidPeriodId]), and position bounds. The Remove reference scans,
/// the Update silencing guard, and both `WeekMove` semantic guards (the F2
/// inline re-implementations) are stripped.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WeekPrecheckError {
    /// A period id is invalid
    #[error("invalid period id ({0:?})")]
    InvalidPeriodId(PeriodId),

    /// A week id is invalid
    #[error("invalid week id ({0:?})")]
    InvalidWeekId(WeekId),

    /// The week id already exists
    #[error("week id ({0:?}) already exists")]
    WeekIdAlreadyExists(WeekId),

    /// The target position is out of range for the destination period
    #[error("position {position} is out of range for period {period:?} (size = {size})")]
    PositionOutOfBounds {
        period: PeriodId,
        position: usize,
        size: usize,
    },
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a week op: carve-out guards kept (returned as
    /// [WeekPrecheckError] — no-clobber, target existence, destination-period
    /// existence, position bounds), invariant guards stripped (step-3 survey
    /// Table 1). May leave the state invalid; the caller owns checking and
    /// rollback.
    pub(crate) fn force_apply_week(
        &mut self,
        week_op: &AnnotatedWeekOp,
    ) -> std::result::Result<AnnotatedWeekOp, WeekPrecheckError> {
        match week_op {
            AnnotatedWeekOp::AddFront(week_id, period_id, desc) => {
                self.force_add_week(*week_id, *period_id, 0, desc)?;
                Ok(AnnotatedWeekOp::Remove(*week_id))
            }
            AnnotatedWeekOp::AddAfter(week_id, after_id, desc) => {
                let Some((period_id, after_pos)) =
                    self.inner_data.params.weeks.week_position(*after_id)
                else {
                    return Err(WeekPrecheckError::InvalidWeekId(*after_id));
                };
                self.force_add_week(*week_id, period_id, after_pos + 1, desc)?;
                Ok(AnnotatedWeekOp::Remove(*week_id))
            }
            AnnotatedWeekOp::Remove(week_id) => self.force_remove_week(*week_id),
            AnnotatedWeekOp::Update(week_id, desc) => self.force_update_week(*week_id, desc),
            AnnotatedWeekOp::Move(week_id, dest_period, dest_pos) => {
                self.force_move_week(*week_id, *dest_period, *dest_pos)
            }
        }
    }

    /// Force-applies a week add: no invariant guard exists here, so every
    /// carve-out guard (no-clobber, destination-period existence, position
    /// bounds) is kept.
    fn force_add_week(
        &mut self,
        week_id: WeekId,
        period_id: PeriodId,
        per_pos: usize,
        desc: &WeekDesc,
    ) -> Result<(), WeekPrecheckError> {
        if self.inner_data.params.weeks.find_week(week_id).is_some() {
            return Err(WeekPrecheckError::WeekIdAlreadyExists(week_id));
        }

        // Existence and bounds are separate (see [Self::add_week]).
        if self
            .inner_data
            .params
            .periods
            .find_period_position(period_id)
            .is_none()
        {
            return Err(WeekPrecheckError::InvalidPeriodId(period_id));
        }
        let period_len = self
            .inner_data
            .params
            .weeks
            .week_count_for_period(period_id)
            .unwrap_or(0);
        if per_pos > period_len {
            return Err(WeekPrecheckError::PositionOutOfBounds {
                period: period_id,
                position: per_pos,
                size: period_len,
            });
        }

        self.inner_data
            .params
            .weeks
            .insert_week_at(week_id, period_id, per_pos, desc.clone());

        Ok(())
    }

    /// Force-applies a week removal: target existence kept; the
    /// pattern-exclusion and colloscope-row scans (invariant guards) stripped.
    fn force_remove_week(&mut self, week_id: WeekId) -> Result<AnnotatedWeekOp, WeekPrecheckError> {
        let Some((period_id, per_pos)) = self.inner_data.params.weeks.week_position(week_id) else {
            return Err(WeekPrecheckError::InvalidWeekId(week_id));
        };

        // stripped: NonTrivialWeekPattern scan + colloscope-row scan

        // Compute the reverse op before mutating.
        let prev_week_id = if per_pos > 0 {
            self.inner_data
                .params
                .weeks
                .week_id_at(period_id, per_pos - 1)
        } else {
            None
        };

        let (_removed_period, _removed_pos, removed_desc) =
            self.inner_data.params.weeks.remove_week_entry(week_id);

        Ok(match prev_week_id {
            None => AnnotatedWeekOp::AddFront(week_id, period_id, removed_desc),
            Some(prev) => AnnotatedWeekOp::AddAfter(week_id, prev, removed_desc),
        })
    }

    /// Force-applies a week update: target existence kept; the silencing
    /// colloscope guard (invariant guard) stripped.
    fn force_update_week(
        &mut self,
        week_id: WeekId,
        new_desc: &WeekDesc,
    ) -> Result<AnnotatedWeekOp, WeekPrecheckError> {
        if self
            .inner_data
            .params
            .weeks
            .week_position(week_id)
            .is_none()
        {
            return Err(WeekPrecheckError::InvalidWeekId(week_id));
        }

        // stripped: the interrogations→off silencing colloscope guard

        let old_desc = self
            .inner_data
            .params
            .weeks
            .replace_week_desc(week_id, new_desc.clone());

        Ok(AnnotatedWeekOp::Update(week_id, old_desc))
    }

    /// Force-applies a week move: target existence, destination-period
    /// existence and position bounds kept; both `WeekMove` semantic guards (the
    /// F2 inline re-implementations) stripped.
    fn force_move_week(
        &mut self,
        week_id: WeekId,
        dest_period: PeriodId,
        dest_pos: usize,
    ) -> Result<AnnotatedWeekOp, WeekPrecheckError> {
        let Some((src_period, src_pos)) = self.inner_data.params.weeks.week_position(week_id)
        else {
            return Err(WeekPrecheckError::InvalidWeekId(week_id));
        };
        if self
            .inner_data
            .params
            .periods
            .find_period_position(dest_period)
            .is_none()
        {
            return Err(WeekPrecheckError::InvalidPeriodId(dest_period));
        }

        // Destination length once the week is detached from its current spot.
        let dest_len_post = self
            .inner_data
            .params
            .weeks
            .week_count_for_period(dest_period)
            .unwrap_or(0)
            - if dest_period == src_period { 1 } else { 0 };
        if dest_pos > dest_len_post {
            // `size` is the post-detachment length — the size of the list the
            // position actually indexes into, so the reported bound matches the
            // one that was checked (on a same-period move that is one below the
            // period's current week count).
            return Err(WeekPrecheckError::PositionOutOfBounds {
                period: dest_period,
                position: dest_pos,
                size: dest_len_post,
            });
        }

        // stripped: the per-row colloscope compatibility guard (subject-runs +
        // group-bound, the F2 inline re-implementations)

        self.inner_data
            .params
            .weeks
            .move_week_entry(week_id, dest_period, dest_pos);

        Ok(AnnotatedWeekOp::Move(week_id, src_period, src_pos))
    }
}
