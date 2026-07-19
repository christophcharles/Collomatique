//! Slots submodule
//!
//! This module defines the relevant types to describes the interrogation slots

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::{Join, References};

use crate::Table;
use crate::ids::{NewId, PeriodId, SlotId, SlotPairingRuleId, SubjectId, TeacherId, WeekPatternId};
use crate::ops::AnnotatedSlotOp;

/// Description of the interrogation slots
///
/// The backend is a flat id-keyed [Table] of slots (each slot carries its
/// subject as a foreign key) plus an explicit per-subject ordering sidecar.
/// The two must stay consistent; the invariant is checked in
/// `check_slots_data_consistency`:
/// - `ordering` is sparse: a row is present exactly when the subject has at
///   least one slot (canonical form — no empty rows), and that subject exists
///   and has interrogations, and
/// - `ordering[s]` is a duplicate-free permutation of
///   `{ id | slot_map[id].subject_id == s }`.
///
/// All mutation goes through the compound `pub(crate)` helpers below so no
/// call site can desynchronize the two structures. The fields are private:
/// consumers read through the accessor surface further down.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Slots {
    /// Every slot, keyed by its id
    slot_map: Table<SlotId, Slot>,
    /// Per-subject ordered list of slot ids
    ///
    /// Sparse: one entry per subject that has at least one slot. A subject
    /// with interrogations but no slots yet has no entry (canonical absent).
    ordering: Table<SubjectId, Vec<SlotId>>,
}

/// Error returned when building [Slots] from rows with a duplicated slot id
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("duplicated slot id {0:?}")]
pub struct DuplicatedSlotIdError(pub SlotId);

/// Description of a single slot
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join)]
#[join(error = NewId)]
pub struct Slot {
    /// Subject this slot belongs to
    ///
    /// This is authoritative: the slot is grouped under this subject in the
    /// ordering sidecar, and [crate::SlotOp::Update] rejects changing it.
    #[fk(name = subject)]
    pub subject_id: SubjectId,
    /// Teacher for the interrogation
    #[fk(name = teacher)]
    pub teacher_id: TeacherId,
    /// Day and start time for the interrogation
    /// The duration is fixed by the subject
    pub start_time: collomatique_time::SlotStart,
    /// Extra info that can be exported (like the room number)
    pub extra_info: String,
    /// Week pattern for the interrogation
    ///
    /// If None, the interrogation happens everyweek
    #[fk]
    pub week_pattern: Option<WeekPatternId>,
    /// Cost for the interrogation
    ///
    /// An optional cost can be defined. By default, this will be 0.
    /// But a positive cost can be chosen to avoid a slot.
    /// A negative cost would rather favor a given slot
    pub cost: i32,
}

impl Slots {
    /// Builds a [Slots] from per-subject rows (used by storage decode).
    ///
    /// `entries` provides one row per subject that has slots: the subject id
    /// and its ordered slots (in the intended order). Each slot must already
    /// carry the matching `subject_id`. Empty rows are dropped so the sparse
    /// canonical form (no empty ordering entry) is preserved. Returns an error
    /// if a slot id appears more than once across all rows — otherwise the two
    /// backend structures would silently desynchronize.
    pub fn from_subject_rows(
        entries: impl IntoIterator<Item = (SubjectId, Vec<(SlotId, Slot)>)>,
    ) -> Result<Self, DuplicatedSlotIdError> {
        let mut slot_map = Table::new();
        let mut ordering = Table::new();
        for (subject_id, slots) in entries {
            if slots.is_empty() {
                // Canonical sparse form: a subject with no slots gets no row.
                continue;
            }
            let mut order = Vec::with_capacity(slots.len());
            for (slot_id, slot) in slots {
                if slot_map.insert(slot_id, slot).is_some() {
                    return Err(DuplicatedSlotIdError(slot_id));
                }
                order.push(slot_id);
            }
            ordering.insert(subject_id, order);
        }
        Ok(Slots { slot_map, ordering })
    }

    /// Test-only corruption: inserts an ordering row verbatim, bypassing the
    /// canonical-sparse discipline of [Self::from_subject_rows] (which drops
    /// empty rows) — a stored empty row is exactly the
    /// [crate::invariants::LogicError::EmptySlotsRow] the invariant checker must
    /// detect, and no production surface can produce it.
    #[cfg(test)]
    pub(crate) fn forge_ordering_row(&mut self, subject: SubjectId, order: Vec<SlotId>) {
        self.ordering.insert(subject, order);
    }

    // ---- Read surface ----
    //
    // These methods are the sanctioned way to read the slots. Consumers go
    // through them rather than the private `slot_map` / `ordering` fields.

    /// Returns the subject and position (within its subject) of a slot id, if valid.
    pub fn find_slot_subject_and_position(&self, slot_id: SlotId) -> Option<(SubjectId, usize)> {
        let subject_id = self.slot_map.get(&slot_id)?.subject_id;
        let pos = self
            .ordering
            .get(&subject_id)
            .expect("slot's subject should have an ordering entry")
            .iter()
            .position(|id| *id == slot_id)
            .expect("slot should appear in its subject's ordering");
        Some((subject_id, pos))
    }

    /// Returns the slot description for a slot id, if it is valid.
    pub fn find_slot(&self, slot_id: SlotId) -> Option<&Slot> {
        self.slot_map.get(&slot_id)
    }

    /// Returns the subject and the slot description for a slot id, if it is valid.
    pub fn find_slot_with_subject(&self, slot_id: SlotId) -> Option<(SubjectId, &Slot)> {
        let slot = self.slot_map.get(&slot_id)?;
        Some((slot.subject_id, slot))
    }

    /// Iterator over the subjects that have at least one slot, in id order.
    ///
    /// Under the sparse ordering this is *not* the same as "subjects with
    /// interrogations": a subject with interrogation parameters but no slots
    /// yet is absent. Consult `Subject::interrogation_parameters` when the
    /// interrogation flag itself is what matters.
    pub fn subjects_with_slots(&self) -> impl Iterator<Item = SubjectId> + '_ {
        self.ordering.keys()
    }

    /// Whether no subject has any slots.
    pub fn is_empty(&self) -> bool {
        self.ordering.is_empty()
    }

    /// Ordered slots for a subject, or `None` if the subject has no interrogations.
    pub fn slots_for_subject(
        &self,
        subject_id: SubjectId,
    ) -> Option<impl Iterator<Item = (&SlotId, &Slot)>> {
        let order = self.ordering.get(&subject_id)?;
        Some(order.iter().map(move |slot_id| {
            let slot = self
                .slot_map
                .get(slot_id)
                .expect("ordering id should be present in slot_map");
            (slot_id, slot)
        }))
    }

    /// Owned copy of the ordered slots for a subject, or `None` if it has no interrogations.
    pub fn slots_vec_for_subject(&self, subject_id: SubjectId) -> Option<Vec<(SlotId, Slot)>> {
        let order = self.ordering.get(&subject_id)?;
        Some(
            order
                .iter()
                .map(|slot_id| {
                    (
                        *slot_id,
                        self.slot_map
                            .get(slot_id)
                            .expect("ordering id should be present in slot_map")
                            .clone(),
                    )
                })
                .collect(),
        )
    }

    /// Number of slots for a subject, or `None` if the subject has no interrogations.
    pub fn slot_count_for_subject(&self, subject_id: SubjectId) -> Option<usize> {
        self.ordering.get(&subject_id).map(|order| order.len())
    }

    /// First slot id for a subject (in order), or `None` if the subject is absent or empty.
    pub fn first_slot_id_for_subject(&self, subject_id: SubjectId) -> Option<SlotId> {
        self.ordering.get(&subject_id)?.first().copied()
    }

    /// Last slot id for a subject (in order), or `None` if the subject is absent or empty.
    pub fn last_slot_id_for_subject(&self, subject_id: SubjectId) -> Option<SlotId> {
        self.ordering.get(&subject_id)?.last().copied()
    }

    /// Iterator over every slot across all subjects (subject grouping flattened), in subject-then-position order.
    pub fn all_slots(&self) -> impl Iterator<Item = (&SlotId, &Slot)> {
        self.ordering.values().flat_map(move |order| {
            order.iter().map(move |slot_id| {
                let slot = self
                    .slot_map
                    .get(slot_id)
                    .expect("ordering id should be present in slot_map");
                (slot_id, slot)
            })
        })
    }

    /// USED INTERNALLY
    ///
    /// Iterator over every slot id, in id order, straight from the slot table
    /// (independent of the ordering sidecar, so it is safe on potentially
    /// inconsistent data during invariant checking).
    pub(crate) fn slot_ids(&self) -> impl Iterator<Item = SlotId> + '_ {
        self.slot_map.keys()
    }

    /// USED INTERNALLY
    ///
    /// Iterator over every `(slot id, slot)` entry, in id order, straight from
    /// the slot table (independent of the ordering sidecar). Used by the
    /// reference registry, which walks slots in id order.
    pub(crate) fn slot_entries(&self) -> impl Iterator<Item = (SlotId, &Slot)> {
        self.slot_map.iter()
    }

    /// USED INTERNALLY
    ///
    /// Raw view of the ordering sidecar (subject → ordered slot ids), for the
    /// consistency invariant check.
    pub(crate) fn ordering_entries(&self) -> impl Iterator<Item = (SubjectId, &[SlotId])> {
        self.ordering
            .iter()
            .map(|(id, order)| (id, order.as_slice()))
    }

    // ---- Compound mutators ----
    //
    // Every mutation goes through one of these so `slot_map` and `ordering`
    // can never desynchronize.

    /// Inserts a slot at `position` within its subject's ordering.
    ///
    /// The subject is taken from `slot.subject_id`. Under the sparse ordering
    /// the row is created on demand for the subject's first slot (which lands
    /// at position 0).
    pub(crate) fn insert_slot_at(&mut self, slot_id: SlotId, slot: Slot, position: usize) {
        let subject_id = slot.subject_id;
        if let Some(order) = self.ordering.get_mut(&subject_id) {
            order.insert(position, slot_id);
        } else {
            debug_assert_eq!(
                position, 0,
                "first slot of a subject must land at position 0"
            );
            self.ordering.insert(subject_id, vec![slot_id]);
        }
        self.slot_map.insert(slot_id, slot);
    }

    /// Removes a slot, returning its former position (within its subject) and data.
    ///
    /// Dropping a subject's last slot removes its ordering row, keeping the
    /// sparse canonical form.
    pub(crate) fn remove_slot(&mut self, slot_id: SlotId) -> (usize, Slot) {
        let slot = self.slot_map.remove(&slot_id).expect("slot should exist");
        let order = self
            .ordering
            .get_mut(&slot.subject_id)
            .expect("slot's subject should have an ordering row");
        let pos = order
            .iter()
            .position(|id| *id == slot_id)
            .expect("slot should appear in its subject's ordering");
        order.remove(pos);
        if order.is_empty() {
            self.ordering.remove(&slot.subject_id);
        }
        (pos, slot)
    }

    /// Moves a slot to `new_pos` within its subject's ordering, returning the old position.
    pub(crate) fn move_slot(&mut self, slot_id: SlotId, new_pos: usize) -> usize {
        let subject_id = self
            .slot_map
            .get(&slot_id)
            .expect("slot should exist")
            .subject_id;
        let order = self
            .ordering
            .get_mut(&subject_id)
            .expect("slot's subject should be registered");
        let old_pos = order
            .iter()
            .position(|id| *id == slot_id)
            .expect("slot should appear in its subject's ordering");
        let id = order.remove(old_pos);
        order.insert(new_pos, id);
        old_pos
    }

    /// Replaces a slot's data (subject unchanged), returning the old data.
    pub(crate) fn replace_slot(&mut self, slot_id: SlotId, new_slot: Slot) -> Slot {
        std::mem::replace(
            self.slot_map.get_mut(&slot_id).expect("slot should exist"),
            new_slot,
        )
    }
}

/// Errors for interrogation slot operations
///
/// These errors can be returned when trying to modify [crate::Data] with a slot op.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SlotError {
    /// A slot id is invalid
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),

    /// The slot id already exists
    #[error("slot id ({0:?}) already exists")]
    SlotIdAlreadyExists(SlotId),

    /// A position is outside of bounds
    #[error("Position {0} is outside the list (size = {1})")]
    PositionOutOfBounds(usize, usize),

    /// The previous slot given is not for the same subject
    #[error("Slot {0:?} to be previous slot is not for subject {1:?}")]
    PreviousSlotIsNotInRightSubject(SlotId, SubjectId),

    /// subject id is invalid
    #[error("invalid subject id ({0:?})")]
    InvalidSubjectId(SubjectId),

    /// subject has no interrogations
    #[error("subject ({0:?}) does not have interrogations")]
    SubjectHasNoInterrogation(SubjectId),

    /// An update tried to move the slot to a different subject
    #[error("slot ({0:?}) cannot change subject (from {1:?} to {2:?})")]
    CannotChangeSubject(SlotId, SubjectId, SubjectId),

    /// teacher id is invalid
    #[error("invalid teacher id ({0:?})")]
    InvalidTeacherId(TeacherId),

    /// week pattern id is invalid
    #[error("invalid week pattern id ({0:?})")]
    InvalidWeekPatternId(WeekPatternId),

    /// Provided teacher does not teach in the corresponding subject
    #[error("Provided teacher ({0:?}) does not teach in subject ({1:?})")]
    TeacherDoesNotTeachInSubject(TeacherId, SubjectId),

    /// Slot overlaps with next day
    #[error("The slot start time is too late and the slot overlaps with the next day")]
    SlotOverlapsWithNextDay,

    /// The slot is not empty in colloscope
    #[error("slot {0:?} in colloscope is not empty for period {1:?}")]
    NotEmptySlotInColloscope(SlotId, PeriodId),

    /// The slot in colloscope is incomaptible with the new week pattern
    #[error("slot {0:?} in colloscope is not compatible with the new week pattern {1:?}")]
    NotCompatibleSlotInColloscope(SlotId, Option<WeekPatternId>),

    /// The slot is referenced by a slot pairing rule
    #[error("slot id ({0:?}) is referenced by a slot pairing rule ({1:?})")]
    SlotIsReferencedBySlotPairingRule(SlotId, SlotPairingRuleId),
}

/// Precondition errors of the forced slot ops — the carve-out subset
/// (step-3 survey Table 2). Kept: no-clobber, op-target existence
/// ([Self::InvalidSlotId]), the `AddAfter` same-subject anchor
/// ([Self::PreviousSlotIsNotInRightSubject]), position bounds, and the
/// subject-immutability guard ([Self::CannotChangeSubject]). `validate_slot`,
/// the Remove colloscope/pairing scans and the Update pattern guard are
/// stripped. Variants copied verbatim from [SlotError].
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SlotPrecheckError {
    /// A slot id is invalid
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),

    /// The slot id already exists
    #[error("slot id ({0:?}) already exists")]
    SlotIdAlreadyExists(SlotId),

    /// A position is outside of bounds
    #[error("Position {0} is outside the list (size = {1})")]
    PositionOutOfBounds(usize, usize),

    /// The previous slot given is not for the same subject
    #[error("Slot {0:?} to be previous slot is not for subject {1:?}")]
    PreviousSlotIsNotInRightSubject(SlotId, SubjectId),

    /// An update tried to move the slot to a different subject
    #[error("slot ({0:?}) cannot change subject (from {1:?} to {2:?})")]
    CannotChangeSubject(SlotId, SubjectId, SubjectId),
}

impl crate::Data {
    /// Used internally
    ///
    /// Apply slot operations
    pub(crate) fn apply_slot(
        &mut self,
        slot_op: &AnnotatedSlotOp,
    ) -> std::result::Result<AnnotatedSlotOp, SlotError> {
        match slot_op {
            AnnotatedSlotOp::AddAfter(new_id, after_id, slot) => {
                // The subject is authoritative from the slot itself.
                let subject_id = slot.subject_id;

                if self.inner_data.params.slots.find_slot(*new_id).is_some() {
                    return Err(SlotError::SlotIdAlreadyExists(*new_id));
                }
                self.inner_data.params.validate_slot(slot)?;

                let position = match after_id {
                    Some(id) => {
                        let (sub_id, after_pos) = self
                            .inner_data
                            .params
                            .slots
                            .find_slot_subject_and_position(*id)
                            .ok_or(SlotError::InvalidSlotId(*id))?;
                        if sub_id != subject_id {
                            return Err(SlotError::PreviousSlotIsNotInRightSubject(
                                *id, subject_id,
                            ));
                        }

                        after_pos + 1
                    }
                    None => 0,
                };

                // `validate_slot` above already rejected a subject without
                // interrogation parameters (`SubjectHasNoInterrogation`), so no
                // separate ordering-presence guard is needed: the sparse row is
                // created on demand by `insert_slot_at`.
                self.inner_data
                    .params
                    .slots
                    .insert_slot_at(*new_id, slot.clone(), position);

                // A fresh slot has no colloscope rows (keyed by `(slot, week)`,
                // an absent row is an empty cell), so nothing is seeded.

                Ok(AnnotatedSlotOp::Remove(*new_id))
            }
            AnnotatedSlotOp::ChangePosition(id, new_pos) => {
                let Some((subject_id, old_pos)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*id)
                else {
                    return Err(SlotError::InvalidSlotId(*id));
                };

                let count = self
                    .inner_data
                    .params
                    .slots
                    .slot_count_for_subject(subject_id)
                    .expect("Subject id should be valid at this point");
                if *new_pos >= count {
                    return Err(SlotError::PositionOutOfBounds(*new_pos, count));
                }

                self.inner_data.params.slots.move_slot(*id, *new_pos);

                Ok(AnnotatedSlotOp::ChangePosition(*id, old_pos))
            }
            AnnotatedSlotOp::Remove(id) => {
                let Some((subject_id, old_pos)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*id)
                else {
                    return Err(SlotError::InvalidSlotId(*id));
                };

                // Canonical-absent surface: a slot blocks removal iff it holds
                // any interrogation row. Report the period of the first such row.
                if let Some(period_id) = self
                    .inner_data
                    .colloscope
                    .interrogations_for_slot(*id)
                    .next()
                    .and_then(|(week, _groups)| {
                        self.inner_data
                            .params
                            .periods
                            .week_position(week)
                            .map(|(period_id, _pos)| period_id)
                    })
                {
                    return Err(SlotError::NotEmptySlotInColloscope(*id, period_id));
                }

                for (rule_id, rule) in self
                    .inner_data
                    .params
                    .slot_pairings
                    .slot_pairing_rule_map
                    .iter()
                {
                    if rule.antecedent.slot_id == *id || rule.consequent.slot_id == *id {
                        return Err(SlotError::SlotIsReferencedBySlotPairingRule(*id, rule_id));
                    }
                }

                // Capture the previous slot in the subject ordering before removing.
                let previous_id = if old_pos > 0 {
                    self.inner_data
                        .params
                        .slots
                        .slots_for_subject(subject_id)
                        .expect("Subject id should be valid at this point")
                        .nth(old_pos - 1)
                        .map(|(slot_id, _)| *slot_id)
                } else {
                    None
                };
                let (_old_pos, old_slot) = self.inner_data.params.slots.remove_slot(*id);
                // The removal guard above already rejected the op if any
                // colloscope row referenced this slot, so no rows remain to drop.

                Ok(AnnotatedSlotOp::AddAfter(*id, previous_id, old_slot))
            }
            AnnotatedSlotOp::Update(slot_id, new_slot) => {
                let Some((subject_id, _position)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                else {
                    return Err(SlotError::InvalidSlotId(*slot_id));
                };

                // A slot cannot be moved to a different subject.
                if new_slot.subject_id != subject_id {
                    return Err(SlotError::CannotChangeSubject(
                        *slot_id,
                        subject_id,
                        new_slot.subject_id,
                    ));
                }

                self.inner_data.params.validate_slot(new_slot)?;

                // If the new week pattern would silence a week that currently
                // holds a colloscope row for this slot, the row would strand an
                // interrogation on an inactive week. Reject before mutating.
                // Rows key on the week id, so nothing else needs to move.
                for (week, _groups) in self.inner_data.colloscope.interrogations_for_slot(*slot_id)
                {
                    if !self
                        .inner_data
                        .params
                        .is_week_active(week, new_slot.week_pattern)
                    {
                        return Err(SlotError::NotCompatibleSlotInColloscope(
                            *slot_id,
                            new_slot.week_pattern,
                        ));
                    }
                }

                let old_slot = self
                    .inner_data
                    .params
                    .slots
                    .replace_slot(*slot_id, new_slot.clone());

                Ok(AnnotatedSlotOp::Update(*slot_id, old_slot))
            }
        }
    }
}
