//! Period submodule
//!
//! This module defines the relevant types to describes the periods

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::{Join, References};

use crate::colloscopes::{ColloscopeInterrogation, ColloscopePeriod};
use crate::ids::{
    NewId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId, SubjectId, WeekId,
    WeekPatternId,
};
use crate::ops::{AnnotatedPeriodOp, AnnotatedWeekOp};
use crate::{OrderedTable, Table};

/// Description of the periods
///
/// The period order lives in `ordered_period_list` and each week is a standalone
/// [Week] entity in `week_map` (both private): consumers read through the
/// accessor surface below rather than touching the containers directly, so the
/// backend shape can change without rippling through every call site.
///
/// The two structures must stay consistent — every week id in the ordering
/// exists in `week_map` and names its owning period, and no `week_map` entry is
/// left un-ordered — an invariant checked in
/// `Parameters::check_periods_data_consistency`. All mutation stays inside this
/// module and routes the week structures through the compound helpers below so
/// no call site can desynchronize them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Periods {
    /// Start date for the colloscope
    ///
    /// The date might not be set but of course, this will hinder
    /// the eventual pretty output
    pub first_week: Option<collomatique_time::WeekStart>,

    /// Ordered list of periods, each with its ordered week ids
    ///
    /// This field gives the relative order of the different periods identified
    /// by their ids, and for each period the order of its weeks (by id). The
    /// week data itself lives in `week_map`.
    ordered_period_list: OrderedTable<PeriodId, Vec<WeekId>>,

    /// Every week, keyed by its id
    ///
    /// Each week carries its owning period as a foreign key; the ordering above
    /// groups those same week ids under that period.
    week_map: Table<WeekId, Week>,
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
        let mut ordering_rows = Vec::with_capacity(rows.len());
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
            ordering_rows.push((period_id, order));
        }
        let ordered_period_list = ordering_rows.try_into().map_err(
            |collomatique_state::tables::DuplicatedIdError(id)| {
                PeriodRowsError::DuplicatedPeriodId(id)
            },
        )?;
        Ok(Periods {
            first_week,
            ordered_period_list,
            week_map,
        })
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
        self.ordered_period_list
            .iter()
            .flat_map(move |(period_id, order)| {
                order.iter().map(move |week_id| {
                    let week = self
                        .week_map
                        .get(week_id)
                        .expect("ordering id should be present in week_map");
                    (period_id, *week_id, week)
                })
            })
    }

    /// All week ids, in global week order.
    pub fn week_ids(&self) -> impl Iterator<Item = WeekId> + '_ {
        self.ordered_period_list
            .iter()
            .flat_map(|(_period_id, order)| order.iter().copied())
    }

    /// Weeks of one period, in order; `None` if the period id is invalid.
    pub fn weeks_of(&self, id: PeriodId) -> Option<impl Iterator<Item = &Week> + '_> {
        let order = self.ordered_period_list.get(&id)?;
        Some(order.iter().map(move |week_id| {
            self.week_map
                .get(week_id)
                .expect("ordering id should be present in week_map")
        }))
    }

    /// Owned copy of a period's weeks — descriptions only, ids and owning period
    /// stripped (op-payload building in `ops/` and gtk4); `None` if the period
    /// id is invalid.
    pub fn weeks_vec_of(&self, id: PeriodId) -> Option<Vec<WeekDesc>> {
        let order = self.ordered_period_list.get(&id)?;
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

    /// Number of weeks of one period; `None` if the period id is invalid.
    pub fn week_count_of(&self, id: PeriodId) -> Option<usize> {
        self.ordered_period_list.get(&id).map(|order| order.len())
    }

    pub fn count_weeks(&self) -> usize {
        self.ordered_period_list
            .iter()
            .map(|(_period_id, order)| order.len())
            .sum()
    }

    /// Finds the position of a period by id
    pub fn find_period_position(&self, id: PeriodId) -> Option<usize> {
        self.ordered_period_list.position_of(&id)
    }

    /// Finds the position of a period by id and gives the number of the first week
    pub fn find_period_position_and_first_week(&self, id: PeriodId) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for (pos, (period_id, order)) in self.ordered_period_list.iter().enumerate() {
            if period_id == id {
                return Some((pos, first_week));
            }
            first_week += order.len();
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

        for (pos, (period_id, order)) in self.ordered_period_list.iter().enumerate() {
            total_weeks += order.len();
            if period_id == id {
                return Some((pos, total_weeks));
            }
        }

        None
    }

    /// Finds a period by id, returning its ordered week ids.
    pub fn find_period(&self, id: PeriodId) -> Option<&Vec<WeekId>> {
        self.ordered_period_list.get(&id)
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
            .ordered_period_list
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
        self.ordered_period_list.get(&period)?.get(pos).copied()
    }

    /// The global week position of a week (its index in `walk()` order);
    /// `None` if the week id is invalid.
    pub fn global_week_position(&self, id: WeekId) -> Option<usize> {
        self.walk().position(|(_, week_id, _)| week_id == id)
    }

    /// Finds the first week number and the length of a period
    pub fn get_first_week_and_length_for_period(&self, id: PeriodId) -> Option<(usize, usize)> {
        let mut first_week = 0usize;

        for (period_id, order) in self.ordered_period_list.iter() {
            if period_id == id {
                return Some((first_week, order.len()));
            }
            first_week += order.len();
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
        self.ordered_period_list
            .iter()
            .map(|(id, order)| (id, order.as_slice()))
    }

    // ---- Compound week mutators ----
    //
    // Every week mutation goes through one of these so `ordered_period_list`
    // and `week_map` can never desynchronize.

    /// Inserts a week into `period_id`'s ordering at `pos` and into `week_map`.
    pub(crate) fn insert_week_at(
        &mut self,
        week_id: WeekId,
        period_id: PeriodId,
        pos: usize,
        desc: WeekDesc,
    ) {
        self.ordered_period_list
            .get_mut(&period_id)
            .expect("period id should be valid")
            .insert(pos, week_id);
        self.week_map
            .insert(week_id, Week::from_desc(period_id, desc));
    }

    /// Removes a week, returning its former period, position and description.
    pub(crate) fn remove_week_entry(&mut self, week_id: WeekId) -> (PeriodId, usize, WeekDesc) {
        let week = self.week_map.remove(&week_id).expect("week should exist");
        let order = self
            .ordered_period_list
            .get_mut(&week.period_id)
            .expect("week's period should have an ordering row");
        let pos = order
            .iter()
            .position(|id| *id == week_id)
            .expect("week should appear in its period's ordering");
        order.remove(pos);
        (week.period_id, pos, week.desc())
    }

    /// Moves a week to `dest_pos` in `dest_period`, returning its former
    /// `(period, position)`. The week keeps its id and description; only its
    /// owning period and its slot in the ordering change.
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
                .ordered_period_list
                .get_mut(&src_period)
                .expect("week's period should have an ordering row");
            let pos = order
                .iter()
                .position(|id| *id == week_id)
                .expect("week should appear in its period's ordering");
            order.remove(pos);
            pos
        };
        self.ordered_period_list
            .get_mut(&dest_period)
            .expect("destination period should be valid")
            .insert(dest_pos, week_id);
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
                    .week_count_of(*period_id)
                    .expect("period id comes from find_period_position");
                if week_count != 0 {
                    return Err(PeriodError::PeriodStillHasWeeks(*period_id));
                }

                // Canonical-absent surface: any interrogation row on a week of
                // this period blocks removal. Vacuous once `PeriodStillHasWeeks`
                // passes (a week-empty period has no rows), but kept as
                // belt-and-suspenders.
                let has_colloscope_row = self
                    .inner_data
                    .colloscope
                    .iter(&self.inner_data.params.periods)
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
    /// Weeks are standalone [Week] entities, but their bits still ride on two
    /// transitional maintenance layers: the week-pattern bit vectors and the
    /// colloscope cell vectors, both indexed positionally by week. This is
    /// where that bookkeeping happens for a single week. Both layers are
    /// temporary: the pattern splicing dies with the week-pattern reshape (B2)
    /// and the colloscope cell splicing with the colloscope reshape (1d).
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
    /// need no maintenance. One colloscope cell per slot of the period is
    /// created (active iff the week carries interrogations, since a fresh week
    /// is excluded by nobody).
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

        // The new week is excluded by no pattern, so its merged activity for
        // any slot reduces to `desc.interrogations`.
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
            .iter(&self.inner_data.params.periods)
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
            .map(|(i, week)| {
                if i == per_pos {
                    new_desc.clone()
                } else {
                    week.desc()
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
            let active_bits = self
                .inner_data
                .params
                .week_pattern_active_bits(slot.week_pattern);
            let new_pattern =
                slot.build_pattern_for_new_period(&new_descs, first_week, &active_bits);
            if !collo_slot.check_empty_on_removed_weeks(&new_pattern) {
                return Err(WeekError::NotCompatibleSlotInColloscope(week_id, *slot_id));
            }
        }

        let old_desc = self
            .inner_data
            .params
            .periods
            .replace_week_desc(week_id, new_desc.clone());

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

        // 1. Move the week entry (ordering slot + owning period).
        self.inner_data
            .params
            .periods
            .move_week_entry(week_id, dest_period, dest_pos);

        // 2. Patterns need no maintenance: exclusion is keyed by the week id,
        //    which is unchanged by the move — membership travels with the id.

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

        // 4. Insert one colloscope cell per destination slot. A traveling cell
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
