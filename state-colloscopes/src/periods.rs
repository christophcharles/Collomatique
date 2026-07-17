//! Period submodule
//!
//! This module defines the relevant types to describes the periods

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::OrderedTable;
use crate::colloscopes::{ColloscopeInterrogation, ColloscopePeriod};
use crate::ids::{
    PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId, SubjectId, WeekId, WeekPatternId,
};
use crate::ops::{AnnotatedPeriodOp, AnnotatedWeekOp};

/// Description of the periods
///
/// The period order and each period's weeks live in `ordered_period_list`
/// (private): consumers read through the accessor surface below rather than
/// touching the container directly, so its payload shape can change without
/// rippling through every call site. All mutation stays inside this module
/// (`apply_period`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Periods {
    /// Start date for the colloscope
    ///
    /// The date might not be set but of course, this will hinder
    /// the eventual pretty output
    pub first_week: Option<collomatique_time::WeekStart>,

    /// Ordered list of periods
    ///
    /// This field gives the relative order of the different
    /// periods identified by their ids
    ///
    /// For each period, we get also a list of weeks. Each entry is a
    /// `(WeekId, WeekDesc)` pair: the week's identity (carried inline in this
    /// transitional shape) plus whether an interrogation happens on it and an
    /// optional annotation.
    ordered_period_list: OrderedTable<PeriodId, Vec<(WeekId, WeekDesc)>>,
}

/// Error returned when building [Periods] from rows with a duplicated period id
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("duplicated period id {0:?}")]
pub struct DuplicatedPeriodIdError(pub PeriodId);

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
    /// id appears more than once.
    pub fn from_period_rows(
        first_week: Option<collomatique_time::WeekStart>,
        rows: Vec<(PeriodId, Vec<(WeekId, WeekDesc)>)>,
    ) -> Result<Self, DuplicatedPeriodIdError> {
        let ordered_period_list =
            rows.try_into()
                .map_err(|collomatique_state::tables::DuplicatedIdError(id)| {
                    DuplicatedPeriodIdError(id)
                })?;
        Ok(Periods {
            first_week,
            ordered_period_list,
        })
    }

    // ---- Read surface ----
    //
    // These methods are the sanctioned way to read the periods. Consumers go
    // through them rather than the private `ordered_period_list` field.

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
    pub fn walk(&self) -> impl Iterator<Item = (PeriodId, WeekId, &WeekDesc)> + '_ {
        self.ordered_period_list
            .iter()
            .flat_map(|(period_id, weeks)| {
                weeks
                    .iter()
                    .map(move |(week_id, desc)| (period_id, *week_id, desc))
            })
    }

    /// All week ids, in global week order.
    pub fn week_ids(&self) -> impl Iterator<Item = WeekId> + '_ {
        self.ordered_period_list
            .iter()
            .flat_map(|(_period_id, weeks)| weeks.iter().map(|(week_id, _desc)| *week_id))
    }

    /// Weeks of one period, in order; `None` if the period id is invalid.
    pub fn weeks_of(&self, id: PeriodId) -> Option<impl Iterator<Item = &WeekDesc> + '_> {
        Some(
            self.ordered_period_list
                .get(&id)?
                .iter()
                .map(|(_week_id, desc)| desc),
        )
    }

    /// Owned copy of a period's weeks — descriptions only, ids stripped
    /// (op-payload building in `ops/` and gtk4); `None` if the period id is
    /// invalid.
    pub fn weeks_vec_of(&self, id: PeriodId) -> Option<Vec<WeekDesc>> {
        Some(
            self.ordered_period_list
                .get(&id)?
                .iter()
                .map(|(_week_id, desc)| desc.clone())
                .collect(),
        )
    }

    /// Number of weeks of one period; `None` if the period id is invalid.
    pub fn week_count_of(&self, id: PeriodId) -> Option<usize> {
        self.ordered_period_list.get(&id).map(|weeks| weeks.len())
    }

    pub fn count_weeks(&self) -> usize {
        self.ordered_period_list.iter().map(|x| x.1.len()).sum()
    }

    /// Finds the position of a period by id
    pub fn find_period_position(&self, id: PeriodId) -> Option<usize> {
        self.ordered_period_list.position_of(&id)
    }

    /// Finds the position of a period by id and gives the number of the first week
    pub fn find_period_position_and_first_week(&self, id: PeriodId) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for (pos, (period_id, desc)) in self.ordered_period_list.iter().enumerate() {
            if period_id == id {
                return Some((pos, first_week));
            }
            first_week += desc.len();
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

        for (pos, (period_id, desc)) in self.ordered_period_list.iter().enumerate() {
            total_weeks += desc.len();
            if period_id == id {
                return Some((pos, total_weeks));
            }
        }

        None
    }

    /// Finds a period by id, returning its weeks (identity paired with
    /// description).
    pub fn find_period(&self, id: PeriodId) -> Option<&Vec<(WeekId, WeekDesc)>> {
        self.ordered_period_list.get(&id)
    }

    /// Finds a week by id, returning its period and description.
    ///
    /// Transitional linear scan (week ids live inline in the period rows).
    pub fn find_week(&self, id: WeekId) -> Option<(PeriodId, &WeekDesc)> {
        self.ordered_period_list
            .iter()
            .find_map(|(period_id, weeks)| {
                weeks
                    .iter()
                    .find(|(week_id, _)| *week_id == id)
                    .map(|(_, desc)| (period_id, desc))
            })
    }

    /// Locates a week by id: its owning period and its position within that
    /// period. `None` if the week id is invalid.
    pub fn week_position(&self, id: WeekId) -> Option<(PeriodId, usize)> {
        self.ordered_period_list
            .iter()
            .find_map(|(period_id, weeks)| {
                weeks
                    .iter()
                    .position(|(week_id, _)| *week_id == id)
                    .map(|pos| (period_id, pos))
            })
    }

    /// The id of the week at position `pos` within `period`; `None` if the
    /// period id is invalid or the position is out of range.
    pub fn week_id_at(&self, period: PeriodId, pos: usize) -> Option<WeekId> {
        self.ordered_period_list
            .get(&period)?
            .get(pos)
            .map(|(week_id, _)| *week_id)
    }

    /// The global week position of a week (its index in `walk()` order);
    /// `None` if the week id is invalid.
    pub fn global_week_position(&self, id: WeekId) -> Option<usize> {
        self.walk().position(|(_, week_id, _)| week_id == id)
    }

    /// Finds the first week number and the length of a period
    pub fn get_first_week_and_length_for_period(&self, id: PeriodId) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for (period_id, desc) in self.ordered_period_list.iter() {
            if period_id == id {
                return Some((first_week, desc.len()));
            }
            first_week += desc.len();
        }

        None
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
                // table gets a row), and an empty colloscope period. Weeks are
                // added afterwards through the WeekOp family.
                self.inner_data
                    .params
                    .periods
                    .ordered_period_list
                    .insert_at(0, *period_id, Vec::new())
                    .expect("period id absence checked above");
                self.inner_data.colloscope.period_map.insert(
                    *period_id,
                    ColloscopePeriod::new_empty_from_params(&self.inner_data.params, *period_id),
                );
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
                    .insert_at(position + 1, *period_id, Vec::new())
                    .expect("period id absence checked above");
                self.inner_data.colloscope.period_map.insert(
                    *period_id,
                    ColloscopePeriod::new_empty_from_params(&self.inner_data.params, *period_id),
                );
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
                    .ordered_period_list
                    .get_at(position)
                    .expect("position comes from find_period_position")
                    .1
                    .len();
                if week_count != 0 {
                    return Err(PeriodError::PeriodStillHasWeeks(*period_id));
                }

                let colloscope_period = self
                    .inner_data
                    .colloscope
                    .period_map
                    .get(period_id)
                    .expect("Period ID should be valid at this point");

                if !colloscope_period.is_empty() {
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
                self.inner_data.colloscope.period_map.remove(period_id);

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
    /// Weeks live inline in the period rows in this transitional stage, so
    /// this is where the pattern-bit and colloscope-cell bookkeeping that used
    /// to ride on the whole-period `PeriodOp::Update` now happens for a single
    /// week. Both maintenance layers are temporary: the pattern splicing dies
    /// with the week-pattern reshape (B2) and the colloscope cell splicing
    /// with the colloscope reshape (1d).
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
    /// A `true` bit is inserted at the new global week in every week pattern
    /// and one colloscope cell per slot of the period is created (active iff
    /// the week carries interrogations, since every pattern is trivial here).
    fn add_week(
        &mut self,
        week_id: WeekId,
        period_id: PeriodId,
        per_pos: usize,
        desc: &WeekDesc,
    ) -> Result<(), WeekError> {
        if self
            .inner_data
            .params
            .periods
            .week_position(week_id)
            .is_some()
        {
            return Err(WeekError::WeekIdAlreadyExists(week_id));
        }

        let Some((_pos, first_week)) = self
            .inner_data
            .params
            .periods
            .find_period_position_and_first_week(period_id)
        else {
            return Err(WeekError::InvalidPeriodId(period_id));
        };
        let period_len = self
            .inner_data
            .params
            .periods
            .week_count_of(period_id)
            .expect("period id validated above");
        if per_pos > period_len {
            return Err(WeekError::InvalidPosition(period_id, per_pos));
        }

        let global_pos = first_week + per_pos;

        self.inner_data
            .params
            .periods
            .ordered_period_list
            .get_mut(&period_id)
            .expect("period id validated above")
            .insert(per_pos, (week_id, desc.clone()));

        for week_pattern in self
            .inner_data
            .params
            .week_patterns
            .week_pattern_map
            .values_mut()
        {
            week_pattern.add_weeks(global_pos, 1);
        }

        // Every pattern bit at `global_pos` is `true` by construction, so the
        // merged activity of the new week reduces to `desc.interrogations`.
        let cell = if desc.interrogations {
            Some(ColloscopeInterrogation::default())
        } else {
            None
        };
        for collo_slot in self
            .inner_data
            .colloscope
            .period_map
            .get_mut(&period_id)
            .expect("period id validated above")
            .slot_map
            .values_mut()
        {
            collo_slot.interrogations.insert(per_pos, cell.clone());
        }

        Ok(())
    }

    /// Removes an existing week.
    ///
    /// Requires every week pattern to be trivial (`true`) at the week (so undo
    /// restores it exactly) and every colloscope cell to be empty. The reverse
    /// re-adds the week at the same spot with the same id.
    fn remove_week(&mut self, week_id: WeekId) -> Result<AnnotatedWeekOp, WeekError> {
        let Some((period_id, per_pos)) = self.inner_data.params.periods.week_position(week_id)
        else {
            return Err(WeekError::InvalidWeekId(week_id));
        };
        let global_pos = self
            .inner_data
            .params
            .periods
            .global_week_position(week_id)
            .expect("week id validated above");

        for (week_pattern_id, week_pattern) in
            self.inner_data.params.week_patterns.week_pattern_map.iter()
        {
            if !week_pattern.can_remove_weeks(global_pos, 1) {
                return Err(WeekError::NonTrivialWeekPattern(week_id, week_pattern_id));
            }
        }

        for (slot_id, collo_slot) in self
            .inner_data
            .colloscope
            .period_map
            .get(&period_id)
            .expect("period id from week_position is valid")
            .slot_map
            .iter()
        {
            let cell_empty = collo_slot
                .interrogations
                .get(per_pos)
                .expect("cell exists for every week of the period")
                .as_ref()
                .is_none_or(|interrogation| interrogation.is_empty());
            if !cell_empty {
                return Err(WeekError::NotCompatibleSlotInColloscope(week_id, *slot_id));
            }
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

        let (_removed_id, removed_desc) = self
            .inner_data
            .params
            .periods
            .ordered_period_list
            .get_mut(&period_id)
            .expect("period id from week_position is valid")
            .remove(per_pos);

        for week_pattern in self
            .inner_data
            .params
            .week_patterns
            .week_pattern_map
            .values_mut()
        {
            week_pattern.remove_weeks(global_pos, 1);
        }

        for collo_slot in self
            .inner_data
            .colloscope
            .period_map
            .get_mut(&period_id)
            .expect("period id from week_position is valid")
            .slot_map
            .values_mut()
        {
            collo_slot.interrogations.remove(per_pos);
        }

        Ok(match prev_week_id {
            None => AnnotatedWeekOp::AddFront(week_id, period_id, removed_desc),
            Some(prev) => AnnotatedWeekOp::AddAfter(week_id, prev, removed_desc),
        })
    }

    /// Updates a week's description (status / annotation) in place.
    ///
    /// The week count is unchanged, so no pattern bits move; only a
    /// `true → false` interrogation flip can silence a colloscope cell, which
    /// is rejected when that cell is non-empty (same guard the whole-period
    /// update uses).
    fn update_week(
        &mut self,
        week_id: WeekId,
        new_desc: &WeekDesc,
    ) -> Result<AnnotatedWeekOp, WeekError> {
        let Some((period_id, per_pos)) = self.inner_data.params.periods.week_position(week_id)
        else {
            return Err(WeekError::InvalidWeekId(week_id));
        };
        let (_pos, first_week) = self
            .inner_data
            .params
            .periods
            .find_period_position_and_first_week(period_id)
            .expect("period id from week_position is valid");

        // The period's would-be week descriptions with this week replaced.
        let new_descs: Vec<WeekDesc> = self
            .inner_data
            .params
            .periods
            .weeks_of(period_id)
            .expect("period id from week_position is valid")
            .enumerate()
            .map(|(i, desc)| {
                if i == per_pos {
                    new_desc.clone()
                } else {
                    desc.clone()
                }
            })
            .collect();

        for (slot_id, collo_slot) in self
            .inner_data
            .colloscope
            .period_map
            .get(&period_id)
            .expect("period id from week_position is valid")
            .slot_map
            .iter()
        {
            let slot = self
                .inner_data
                .params
                .slots
                .find_slot(*slot_id)
                .expect("slot id from colloscope is valid");
            let new_pattern = slot.build_pattern_for_new_period(
                &new_descs,
                first_week,
                &self.inner_data.params.week_patterns,
            );
            if !collo_slot.check_empty_on_removed_weeks(&new_pattern) {
                return Err(WeekError::NotCompatibleSlotInColloscope(week_id, *slot_id));
            }
        }

        let old_desc = std::mem::replace(
            &mut self
                .inner_data
                .params
                .periods
                .ordered_period_list
                .get_mut(&period_id)
                .expect("period id from week_position is valid")[per_pos]
                .1,
            new_desc.clone(),
        );

        let slot_ids: Vec<_> = self
            .inner_data
            .params
            .slots
            .all_slots()
            .map(|(slot_id, _slot)| *slot_id)
            .collect();
        for slot_id in slot_ids {
            self.inner_data
                .colloscope
                .update_slot_to_match_week_pattern(slot_id, &self.inner_data.params);
        }

        Ok(AnnotatedWeekOp::Update(week_id, old_desc))
    }

    /// Moves a week to `dest_pos` in `dest_period`, carrying its content.
    ///
    /// The week's pattern bits travel positionally (no triviality guard — no
    /// information is lost) and its colloscope cells travel where the slot
    /// exists on both sides. A non-empty cell may only travel to a slot the
    /// destination period runs, and only if its groups fit the destination
    /// association bounds.
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
        let (_src_ppos, src_first) = self
            .inner_data
            .params
            .periods
            .find_period_position_and_first_week(src_period)
            .expect("src period from week_position is valid");
        let Some((_dest_ppos, dest_first)) = self
            .inner_data
            .params
            .periods
            .find_period_position_and_first_week(dest_period)
        else {
            return Err(WeekError::InvalidPeriodId(dest_period));
        };
        let src_global = src_first + src_pos;

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

        // Detached global numbering: removing `src_global` shifts everything
        // after it down by one.
        let dest_first_post = if src_global < dest_first {
            dest_first - 1
        } else {
            dest_first
        };
        let dest_global = dest_first_post + dest_pos;

        let desc = self
            .inner_data
            .params
            .periods
            .find_week(week_id)
            .expect("week id validated above")
            .1
            .clone();

        // Source cells (captured before mutating), keyed by slot.
        let src_cells: BTreeMap<SlotId, Option<ColloscopeInterrogation>> = self
            .inner_data
            .colloscope
            .period_map
            .get(&src_period)
            .expect("src period is valid")
            .slot_map
            .iter()
            .map(|(slot_id, collo_slot)| {
                (
                    *slot_id,
                    collo_slot
                        .interrogations
                        .get(src_pos)
                        .expect("cell exists for every week of the period")
                        .clone(),
                )
            })
            .collect();

        // Guard: any non-empty source cell must be able to travel.
        let dest_collo_slots: BTreeSet<SlotId> = self
            .inner_data
            .colloscope
            .period_map
            .get(&dest_period)
            .expect("dest period is valid")
            .slot_map
            .keys()
            .copied()
            .collect();
        for (slot_id, cell) in &src_cells {
            let Some(interrogation) = cell else { continue };
            if interrogation.is_empty() {
                continue;
            }
            if !dest_collo_slots.contains(slot_id) {
                return Err(WeekError::NotCompatibleSlotInColloscope(week_id, *slot_id));
            }
            let (subject_id, _pos) = self
                .inner_data
                .params
                .slots
                .find_slot_subject_and_position(*slot_id)
                .expect("slot id from colloscope is valid");
            let bound = self
                .inner_data
                .params
                .group_lists
                .subjects_associations
                .get(&(dest_period, subject_id))
                .map(|group_list_id| {
                    self.inner_data
                        .params
                        .group_lists
                        .group_list_map
                        .get(group_list_id)
                        .expect("association references a live group list")
                        .params
                        .group_names
                        .len() as u32
                })
                .unwrap_or(0);
            if interrogation.assigned_groups.iter().any(|g| *g >= bound) {
                return Err(WeekError::NotCompatibleSlotInColloscope(week_id, *slot_id));
            }
        }

        // --- Mutations ---

        // 1. Detach the week entry from the source period.
        self.inner_data
            .params
            .periods
            .ordered_period_list
            .get_mut(&src_period)
            .expect("src period is valid")
            .remove(src_pos);

        // 2. Move each pattern bit to the destination position.
        for week_pattern in self
            .inner_data
            .params
            .week_patterns
            .week_pattern_map
            .values_mut()
        {
            week_pattern.move_week(src_global, dest_global);
        }

        // 3. Detach the source cells.
        for collo_slot in self
            .inner_data
            .colloscope
            .period_map
            .get_mut(&src_period)
            .expect("src period is valid")
            .slot_map
            .values_mut()
        {
            collo_slot.interrogations.remove(src_pos);
        }

        // 4. Splice the week entry into the destination period.
        self.inner_data
            .params
            .periods
            .ordered_period_list
            .get_mut(&dest_period)
            .expect("dest period is valid")
            .insert(dest_pos, (week_id, desc));

        // 5. Insert one colloscope cell per destination slot. A traveling cell
        //    keeps its content; a slot only present at the destination gets a
        //    fresh cell reflecting the merged activity at the new week.
        let dest_slot_ids: Vec<SlotId> = self
            .inner_data
            .colloscope
            .period_map
            .get(&dest_period)
            .expect("dest period is valid")
            .slot_map
            .keys()
            .copied()
            .collect();
        for slot_id in dest_slot_ids {
            let week_pattern_id = self
                .inner_data
                .params
                .slots
                .find_slot(slot_id)
                .expect("slot id from colloscope is valid")
                .week_pattern;
            let merged = self.inner_data.params.get_merged_pattern(week_pattern_id);
            let cell = if merged[dest_global] {
                src_cells
                    .get(&slot_id)
                    .cloned()
                    .flatten()
                    .or_else(|| Some(ColloscopeInterrogation::default()))
            } else {
                None
            };
            self.inner_data
                .colloscope
                .period_map
                .get_mut(&dest_period)
                .expect("dest period is valid")
                .slot_map
                .get_mut(&slot_id)
                .expect("slot id from dest_slot_ids")
                .interrogations
                .insert(dest_pos, cell);
        }

        Ok(AnnotatedWeekOp::Move(week_id, src_period, src_pos))
    }
}
