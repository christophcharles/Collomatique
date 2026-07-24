//! Period submodule
//!
//! This module defines the relevant types to describes the periods

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::{Join, References};

use crate::ids::{
    NewId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId, SubjectId, WeekId,
    WeekPatternId,
};
use crate::ops::{AnnotatedPeriodOp, AnnotatedWeekOp};
use crate::{OrderedTable, Table};

/// Description of the periods
///
/// A period owns *existence and display order* only: `ordered_period_list` is
/// the public ordered set of period ids (mirroring `Subjects.ordered_subject_list`),
/// each mapping to `()` — a period carries no other data of its own. Weeks are
/// standalone [Week] entities in `week_map` (each naming its owning period as a
/// foreign key), and their per-period ordering lives in the sparse `ordering`
/// sidecar. Both `week_map` and `ordering` are private: consumers read through
/// the accessor surface below rather than touching the containers directly, so
/// the backend shape can change without rippling through every call site.
///
/// (The module split of `weeks.rs` — moving `week_map`/`ordering` and the week
/// surface into their own module, the twin of `slots.rs` — lands in a later
/// commit; this shape is already the final one.)
///
/// The three structures must stay consistent — every period named by an
/// `ordering` row exists in `ordered_period_list`, the row is non-empty
/// (canonical sparse form), every week id in it exists in `week_map` and names
/// that period, and no `week_map` entry is left un-ordered — an invariant
/// checked in `Parameters::check_periods_data_consistency`. All mutation stays
/// inside this module and routes the week structures through the compound
/// helpers below so no call site can desynchronize them.
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
    /// in the `ordering` sidecar and `week_map`. Public, mirroring
    /// `Subjects.ordered_subject_list`.
    pub ordered_period_list: OrderedTable<PeriodId, ()>,

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

/// Error returned when building [Periods] from rows with a duplicated id
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum PeriodRowsError {
    /// A period id appears more than once
    #[error("duplicated period id {0:?}")]
    DuplicatedPeriodId(PeriodId),
    /// A week id appears more than once (across all periods)
    #[error("duplicated week id {0:?}")]
    DuplicatedWeekId(WeekId),
}

/// Description of a single week
///
/// This is the stored week entity: it carries its owning period as a foreign
/// key plus whether an interrogation happens on it and an optional annotation.
/// The period-less, id-less [WeekDesc] is the matching op-payload / DTO form.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join)]
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

impl Periods {
    /// Builds a [Periods] from period rows (used by storage decode).
    ///
    /// `rows` provides the periods in display order, each with its ordered
    /// weeks (identity paired with description). Returns an error if a period
    /// id or a week id appears more than once — otherwise the two backend
    /// structures would silently desynchronize.
    pub fn from_period_rows(
        first_week: Option<collomatique_time::WeekStart>,
        rows: Vec<(PeriodId, Vec<(WeekId, WeekDesc)>)>,
    ) -> Result<Self, PeriodRowsError> {
        let mut week_map = Table::new();
        let mut ordering = Table::new();
        let mut period_rows = Vec::with_capacity(rows.len());
        for (period_id, weeks) in rows {
            let mut order = Vec::with_capacity(weeks.len());
            for (week_id, desc) in weeks {
                if week_map
                    .insert(week_id, Week::from_desc(period_id, desc))
                    .is_some()
                {
                    return Err(PeriodRowsError::DuplicatedWeekId(week_id));
                }
                order.push(week_id);
            }
            // Canonical sparse form: a week-empty period gets no ordering row.
            if !order.is_empty() {
                ordering.insert(period_id, order);
            }
            period_rows.push((period_id, ()));
        }
        let ordered_period_list = period_rows.try_into().map_err(
            |collomatique_state::tables::DuplicatedIdError(id)| {
                PeriodRowsError::DuplicatedPeriodId(id)
            },
        )?;
        Ok(Periods {
            first_week,
            ordered_period_list,
            week_map,
            ordering,
        })
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
    // These methods are the sanctioned way to read the periods. Consumers go
    // through them rather than the private `ordered_period_list` / `week_map`
    // fields.

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

    /// The canonical global week order: every week of every period, in
    /// period-then-position order, each with its identity. `walk().enumerate()`
    /// gives the global week index — this replaces every hand-rolled
    /// accumulate-`len()` loop.
    pub fn walk(&self) -> impl Iterator<Item = (PeriodId, WeekId, &Week)> + '_ {
        self.ordered_period_list.keys().flat_map(move |period_id| {
            self.week_order(period_id).iter().map(move |week_id| {
                let week = self
                    .week_map
                    .get(week_id)
                    .expect("ordering id should be present in week_map");
                (period_id, *week_id, week)
            })
        })
    }

    /// The ordered week ids of a period that is known to exist, defaulting to an
    /// empty slice when the period has no ordering row (a week-empty period).
    fn week_order(&self, id: PeriodId) -> &[WeekId] {
        self.ordering.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// All week ids, in global week order.
    pub fn week_ids(&self) -> impl Iterator<Item = WeekId> + '_ {
        self.ordered_period_list
            .keys()
            .flat_map(move |period_id| self.week_order(period_id).iter().copied())
    }

    /// Weeks of one period, in order; `None` if the period id is invalid.
    pub fn weeks_of(&self, id: PeriodId) -> Option<impl Iterator<Item = &Week> + '_> {
        if !self.ordered_period_list.contains(&id) {
            return None;
        }
        Some(self.week_order(id).iter().map(move |week_id| {
            self.week_map
                .get(week_id)
                .expect("ordering id should be present in week_map")
        }))
    }

    /// Ordered week ids of one period; `None` if the period id is invalid.
    pub fn week_ids_of(&self, id: PeriodId) -> Option<impl Iterator<Item = WeekId> + '_> {
        if !self.ordered_period_list.contains(&id) {
            return None;
        }
        Some(self.week_order(id).iter().copied())
    }

    /// Owned copy of a period's weeks — descriptions only, ids and owning period
    /// stripped (op-payload building in `ops/` and gtk4); `None` if the period
    /// id is invalid.
    pub fn weeks_vec_of(&self, id: PeriodId) -> Option<Vec<WeekDesc>> {
        if !self.ordered_period_list.contains(&id) {
            return None;
        }
        Some(
            self.week_order(id)
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

    /// Number of weeks of one period; `None` if the period id is invalid.
    pub fn week_count_of(&self, id: PeriodId) -> Option<usize> {
        if !self.ordered_period_list.contains(&id) {
            return None;
        }
        Some(self.week_order(id).len())
    }

    pub fn count_weeks(&self) -> usize {
        self.ordering.values().map(|order| order.len()).sum()
    }

    /// Finds the position of a period by id
    pub fn find_period_position(&self, id: PeriodId) -> Option<usize> {
        self.ordered_period_list.position_of(&id)
    }

    /// Finds the position of a period by id and gives the number of the first week
    pub fn find_period_position_and_first_week(&self, id: PeriodId) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for (pos, period_id) in self.ordered_period_list.keys().enumerate() {
            if period_id == id {
                return Some((pos, first_week));
            }
            first_week += self.week_order(period_id).len();
        }

        None
    }

    /// Finds the position of a period by id and gives the total number of weeks up to and including the
    /// given period
    pub fn find_period_position_and_total_number_of_weeks(
        &self,
        id: PeriodId,
    ) -> Option<(usize, usize)> {
        let mut total_weeks = 0usize;

        for (pos, period_id) in self.ordered_period_list.keys().enumerate() {
            total_weeks += self.week_order(period_id).len();
            if period_id == id {
                return Some((pos, total_weeks));
            }
        }

        None
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
    /// period id is invalid or the position is out of range.
    pub fn week_id_at(&self, period: PeriodId, pos: usize) -> Option<WeekId> {
        if !self.ordered_period_list.contains(&period) {
            return None;
        }
        self.week_order(period).get(pos).copied()
    }

    /// The global week position of a week (its index in `walk()` order);
    /// `None` if the week id is invalid.
    pub fn global_week_position(&self, id: WeekId) -> Option<usize> {
        self.walk().position(|(_, week_id, _)| week_id == id)
    }

    /// Finds the first week number and the length of a period
    pub fn get_first_week_and_length_for_period(&self, id: PeriodId) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for period_id in self.ordered_period_list.keys() {
            let len = self.week_order(period_id).len();
            if period_id == id {
                return Some((first_week, len));
            }
            first_week += len;
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
    // Every week mutation goes through one of these so `ordered_period_list`
    // and `week_map` can never desynchronize.

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

/// Errors for week operations
///
/// These errors can be returned when trying to modify [crate::Data] with a week op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum WeekError {
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
    #[error("invalid position ({1}) in period ({0:?})")]
    InvalidPosition(PeriodId, usize),

    /// A week pattern is not trivial on the week to be removed
    #[error("week pattern {1:?} is not trivial on week {0:?}")]
    NonTrivialWeekPattern(WeekId, WeekPatternId),

    /// A slot in the colloscope blocks the operation on the week
    #[error("slot {1:?} in colloscope blocks the operation on week {0:?}")]
    NotCompatibleSlotInColloscope(WeekId, SlotId),
}

/// Precondition errors of the forced week ops — the carve-out subset
/// (step-3 survey Table 2). Kept: no-clobber, op-target existence
/// ([Self::InvalidWeekId]), destination-period existence for add/move
/// ([Self::InvalidPeriodId]), and position bounds. The Remove reference scans,
/// the Update silencing guard, and both `WeekMove` semantic guards (the F2
/// inline re-implementations) are stripped. Variants copied verbatim from
/// [WeekError].
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
    #[error("invalid position ({1}) in period ({0:?})")]
    InvalidPosition(PeriodId, usize),
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
                    .periods
                    .week_count_of(*period_id)
                    .expect("period id comes from find_period_position");
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
                                .periods
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
                    if rule.excluded_periods.contains(period_id) {
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

impl crate::Data {
    /// Used internally
    ///
    /// Apply week operations.
    ///
    /// Weeks are standalone [Week] entities. Week patterns key their exclusions
    /// by week id and the colloscope keys its rows by `(slot, week)`, so neither
    /// needs positional maintenance when a week is spliced in or out — the only
    /// bookkeeping left is the guards that reject an op which would strand a
    /// colloscope row on an inactive week.
    pub(crate) fn apply_week(
        &mut self,
        week_op: &AnnotatedWeekOp,
    ) -> std::result::Result<AnnotatedWeekOp, WeekError> {
        match week_op {
            AnnotatedWeekOp::AddFront(week_id, period_id, desc) => {
                self.add_week(*week_id, *period_id, 0, desc)?;
                Ok(AnnotatedWeekOp::Remove(*week_id))
            }
            AnnotatedWeekOp::AddAfter(week_id, after_id, desc) => {
                let Some((period_id, after_pos)) =
                    self.inner_data.params.periods.week_position(*after_id)
                else {
                    return Err(WeekError::InvalidWeekId(*after_id));
                };
                self.add_week(*week_id, period_id, after_pos + 1, desc)?;
                Ok(AnnotatedWeekOp::Remove(*week_id))
            }
            AnnotatedWeekOp::Remove(week_id) => self.remove_week(*week_id),
            AnnotatedWeekOp::Update(week_id, desc) => self.update_week(*week_id, desc),
            AnnotatedWeekOp::Move(week_id, dest_period, dest_pos) => {
                self.move_week(*week_id, *dest_period, *dest_pos)
            }
        }
    }

    /// Splices a week into `period_id` at per-period position `per_pos`.
    ///
    /// The new week's id belongs to no pattern's exclusion set, so patterns
    /// need no maintenance; and the colloscope keys rows by `(slot, week)`, so a
    /// fresh week simply has no rows yet (an absent row is an empty cell).
    fn add_week(
        &mut self,
        week_id: WeekId,
        period_id: PeriodId,
        per_pos: usize,
        desc: &WeekDesc,
    ) -> Result<(), WeekError> {
        if self.inner_data.params.periods.find_week(week_id).is_some() {
            return Err(WeekError::WeekIdAlreadyExists(week_id));
        }

        let period_len = match self.inner_data.params.periods.week_count_of(period_id) {
            Some(len) => len,
            None => return Err(WeekError::InvalidPeriodId(period_id)),
        };
        if per_pos > period_len {
            return Err(WeekError::InvalidPosition(period_id, per_pos));
        }

        self.inner_data
            .params
            .periods
            .insert_week_at(week_id, period_id, per_pos, desc.clone());

        Ok(())
    }

    /// Removes an existing week.
    ///
    /// Requires no week pattern to exclude the week (so undo restores its
    /// membership exactly) and every colloscope cell to be empty. The reverse
    /// re-adds the week at the same spot with the same id.
    fn remove_week(&mut self, week_id: WeekId) -> Result<AnnotatedWeekOp, WeekError> {
        let Some((period_id, per_pos)) = self.inner_data.params.periods.week_position(week_id)
        else {
            return Err(WeekError::InvalidWeekId(week_id));
        };

        for (week_pattern_id, week_pattern) in
            self.inner_data.params.week_patterns.week_pattern_map.iter()
        {
            if week_pattern.excluded_weeks.contains(&week_id) {
                return Err(WeekError::NonTrivialWeekPattern(week_id, week_pattern_id));
            }
        }

        // Canonical-absent surface: any interrogation row on this week (for any
        // slot) blocks removal. Report the first such slot in surface order.
        if let Some(slot_id) = self
            .inner_data
            .colloscope
            .iter()
            .find(|((_slot_id, week), _groups)| *week == week_id)
            .map(|((slot_id, _week), _groups)| slot_id)
        {
            return Err(WeekError::NotCompatibleSlotInColloscope(week_id, slot_id));
        }

        // Compute the reverse op before mutating.
        let prev_week_id = if per_pos > 0 {
            self.inner_data
                .params
                .periods
                .week_id_at(period_id, per_pos - 1)
        } else {
            None
        };

        let (_removed_period, _removed_pos, removed_desc) =
            self.inner_data.params.periods.remove_week_entry(week_id);

        Ok(match prev_week_id {
            None => AnnotatedWeekOp::AddFront(week_id, period_id, removed_desc),
            Some(prev) => AnnotatedWeekOp::AddAfter(week_id, prev, removed_desc),
        })
    }

    /// Updates a week's description (status / annotation) in place.
    ///
    /// The week keeps its id and position, so colloscope rows (keyed by
    /// `(slot, week)`) stay put. Only a `true → false` interrogation flip can
    /// invalidate a row — it would strand an interrogation on a now-inactive
    /// week — which is rejected before mutating.
    fn update_week(
        &mut self,
        week_id: WeekId,
        new_desc: &WeekDesc,
    ) -> Result<AnnotatedWeekOp, WeekError> {
        if self
            .inner_data
            .params
            .periods
            .week_position(week_id)
            .is_none()
        {
            return Err(WeekError::InvalidWeekId(week_id));
        }

        // Silencing the week (interrogations flipping off) would leave any
        // colloscope row on it stranded on an inactive week. Reject before
        // mutating. A `false → true` flip only activates weeks, so it can never
        // invalidate an existing row.
        if !new_desc.interrogations
            && let Some(slot_id) = self
                .inner_data
                .colloscope
                .iter()
                .find(|((_slot, week), _groups)| *week == week_id)
                .map(|((slot, _week), _groups)| slot)
        {
            return Err(WeekError::NotCompatibleSlotInColloscope(week_id, slot_id));
        }

        let old_desc = self
            .inner_data
            .params
            .periods
            .replace_week_desc(week_id, new_desc.clone());

        Ok(AnnotatedWeekOp::Update(week_id, old_desc))
    }

    /// Moves a week to `dest_pos` in `dest_period`, carrying its content.
    ///
    /// The week keeps its id, so its pattern exclusions and its colloscope rows
    /// (keyed by `(slot, week)`) travel with it automatically — nothing is
    /// re-spliced. The one thing that can change is the destination period: a
    /// non-empty row may only land in a period that runs the slot's subject, and
    /// only if its groups fit the destination association bound. Both are
    /// checked before mutating so an invalid move is rejected cleanly.
    fn move_week(
        &mut self,
        week_id: WeekId,
        dest_period: PeriodId,
        dest_pos: usize,
    ) -> Result<AnnotatedWeekOp, WeekError> {
        let Some((src_period, src_pos)) = self.inner_data.params.periods.week_position(week_id)
        else {
            return Err(WeekError::InvalidWeekId(week_id));
        };
        if self
            .inner_data
            .params
            .periods
            .week_count_of(dest_period)
            .is_none()
        {
            return Err(WeekError::InvalidPeriodId(dest_period));
        }

        // Destination length once the week is detached from its current spot.
        let dest_len_post = self
            .inner_data
            .params
            .periods
            .week_count_of(dest_period)
            .expect("dest period validated above")
            - if dest_period == src_period { 1 } else { 0 };
        if dest_pos > dest_len_post {
            return Err(WeekError::InvalidPosition(dest_period, dest_pos));
        }

        // Guard: every colloscope row on the moving week must be able to live in
        // the destination period. The week's activity for a slot is unchanged
        // (pattern exclusion and the week's interrogation flag both key on the
        // preserved id), so only the destination period matters — the slot's
        // subject must run there and the assigned groups must fit the
        // destination association bound.
        let params = &self.inner_data.params;
        for ((slot_id, week), groups) in self.inner_data.colloscope.iter() {
            if week != week_id {
                continue;
            }
            let (subject_id, _pos) = params
                .slots
                .find_slot_subject_and_position(slot_id)
                .expect("slot id from colloscope is valid");
            let subject = params
                .subjects
                .find_subject(subject_id)
                .expect("subject id from a live slot is valid");
            if subject.parameters.interrogation_parameters.is_none()
                || subject.excluded_periods.contains(&dest_period)
            {
                return Err(WeekError::NotCompatibleSlotInColloscope(week_id, slot_id));
            }
            let bound = params
                .group_lists
                .subjects_associations
                .get(&(dest_period, subject_id))
                .map(|group_list_id| {
                    params
                        .group_lists
                        .group_list_map
                        .get(group_list_id)
                        .expect("association references a live group list")
                        .params
                        .group_names
                        .len() as u32
                })
                .unwrap_or(0);
            if groups.iter().any(|g| *g >= bound) {
                return Err(WeekError::NotCompatibleSlotInColloscope(week_id, slot_id));
            }
        }

        // Move the week entry (ordering slot + owning period). Patterns and the
        // colloscope need no maintenance: both key on the week id, which the
        // move preserves, so every exclusion and every row travels with it.
        self.inner_data
            .params
            .periods
            .move_week_entry(week_id, dest_period, dest_pos);

        Ok(AnnotatedWeekOp::Move(week_id, src_period, src_pos))
    }

    /// Used internally by [crate::Data::force_apply]
    ///
    /// Thin copy of [Self::apply_week]: carve-out guards kept (returned as
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
                    self.inner_data.params.periods.week_position(*after_id)
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

    /// Thin copy of [Self::add_week]: no invariant guard exists here, so every
    /// carve-out guard (no-clobber, destination-period existence, position
    /// bounds) is kept.
    fn force_add_week(
        &mut self,
        week_id: WeekId,
        period_id: PeriodId,
        per_pos: usize,
        desc: &WeekDesc,
    ) -> Result<(), WeekPrecheckError> {
        if self.inner_data.params.periods.find_week(week_id).is_some() {
            return Err(WeekPrecheckError::WeekIdAlreadyExists(week_id));
        }

        let period_len = match self.inner_data.params.periods.week_count_of(period_id) {
            Some(len) => len,
            None => return Err(WeekPrecheckError::InvalidPeriodId(period_id)),
        };
        if per_pos > period_len {
            return Err(WeekPrecheckError::InvalidPosition(period_id, per_pos));
        }

        self.inner_data
            .params
            .periods
            .insert_week_at(week_id, period_id, per_pos, desc.clone());

        Ok(())
    }

    /// Thin copy of [Self::remove_week]: target existence kept; the
    /// pattern-exclusion and colloscope-row scans (invariant guards) stripped.
    fn force_remove_week(&mut self, week_id: WeekId) -> Result<AnnotatedWeekOp, WeekPrecheckError> {
        let Some((period_id, per_pos)) = self.inner_data.params.periods.week_position(week_id)
        else {
            return Err(WeekPrecheckError::InvalidWeekId(week_id));
        };

        // stripped: NonTrivialWeekPattern scan + colloscope-row scan

        // Compute the reverse op before mutating.
        let prev_week_id = if per_pos > 0 {
            self.inner_data
                .params
                .periods
                .week_id_at(period_id, per_pos - 1)
        } else {
            None
        };

        let (_removed_period, _removed_pos, removed_desc) =
            self.inner_data.params.periods.remove_week_entry(week_id);

        Ok(match prev_week_id {
            None => AnnotatedWeekOp::AddFront(week_id, period_id, removed_desc),
            Some(prev) => AnnotatedWeekOp::AddAfter(week_id, prev, removed_desc),
        })
    }

    /// Thin copy of [Self::update_week]: target existence kept; the silencing
    /// colloscope guard (invariant guard) stripped.
    fn force_update_week(
        &mut self,
        week_id: WeekId,
        new_desc: &WeekDesc,
    ) -> Result<AnnotatedWeekOp, WeekPrecheckError> {
        if self
            .inner_data
            .params
            .periods
            .week_position(week_id)
            .is_none()
        {
            return Err(WeekPrecheckError::InvalidWeekId(week_id));
        }

        // stripped: the interrogations→off silencing colloscope guard

        let old_desc = self
            .inner_data
            .params
            .periods
            .replace_week_desc(week_id, new_desc.clone());

        Ok(AnnotatedWeekOp::Update(week_id, old_desc))
    }

    /// Thin copy of [Self::move_week]: target existence, destination-period
    /// existence and position bounds kept; both `WeekMove` semantic guards (the
    /// F2 inline re-implementations) stripped.
    fn force_move_week(
        &mut self,
        week_id: WeekId,
        dest_period: PeriodId,
        dest_pos: usize,
    ) -> Result<AnnotatedWeekOp, WeekPrecheckError> {
        let Some((src_period, src_pos)) = self.inner_data.params.periods.week_position(week_id)
        else {
            return Err(WeekPrecheckError::InvalidWeekId(week_id));
        };
        if self
            .inner_data
            .params
            .periods
            .week_count_of(dest_period)
            .is_none()
        {
            return Err(WeekPrecheckError::InvalidPeriodId(dest_period));
        }

        // Destination length once the week is detached from its current spot.
        let dest_len_post = self
            .inner_data
            .params
            .periods
            .week_count_of(dest_period)
            .expect("dest period validated above")
            - if dest_period == src_period { 1 } else { 0 };
        if dest_pos > dest_len_post {
            return Err(WeekPrecheckError::InvalidPosition(dest_period, dest_pos));
        }

        // stripped: the per-row colloscope compatibility guard (subject-runs +
        // group-bound, the F2 inline re-implementations)

        self.inner_data
            .params
            .periods
            .move_week_entry(week_id, dest_period, dest_pos);

        Ok(AnnotatedWeekOp::Move(week_id, src_period, src_pos))
    }
}
