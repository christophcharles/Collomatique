//! Slots submodule
//!
//! This module defines the relevant types to describes the interrogation slots

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Table;
use crate::colloscopes;
use crate::ids::{PeriodId, SlotId, SlotPairingRuleId, SubjectId, TeacherId, WeekPatternId};
use crate::ops::AnnotatedSlotOp;

/// Description of the interrogation slots
///
/// The backend is a flat id-keyed [Table] of slots (each slot carries its
/// subject as a foreign key) plus an explicit per-subject ordering sidecar.
/// The two must stay consistent; the invariant is checked in
/// `check_slots_data_consistency`:
/// - `ordering` has exactly one entry per subject with interrogations
///   (dense-key semantics), and
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
    /// One entry per subject with interrogations (empty vec when the subject
    /// has no slots yet).
    ordering: Table<SubjectId, Vec<SlotId>>,
}

/// Error returned when building [Slots] from rows with a duplicated slot id
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("duplicated slot id {0:?}")]
pub struct DuplicatedSlotIdError(pub SlotId);

/// Description of a single slot
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    /// Subject this slot belongs to
    ///
    /// This is authoritative: the slot is grouped under this subject in the
    /// ordering sidecar, and [crate::SlotOp::Update] rejects changing it.
    pub subject_id: SubjectId,
    /// Teacher for the interrogation
    pub teacher_id: TeacherId,
    /// Day and start time for the interrogation
    /// The duration is fixed by the subject
    pub start_time: collomatique_time::SlotStart,
    /// Extra info that can be exported (like the room number)
    pub extra_info: String,
    /// Week pattern for the interrogation
    ///
    /// If None, the interrogation happens everyweek
    pub week_pattern: Option<WeekPatternId>,
    /// Cost for the interrogation
    ///
    /// An optional cost can be defined. By default, this will be 0.
    /// But a positive cost can be chosen to avoid a slot.
    /// A negative cost would rather favor a given slot
    pub cost: i32,
}

impl Slot {
    pub(crate) fn build_pattern_for_new_period(
        &self,
        new_period_desc: &[super::periods::WeekDesc],
        first_week: usize,
        week_patterns: &super::week_patterns::WeekPatterns,
    ) -> Vec<bool> {
        let mut base_pattern: Vec<_> = new_period_desc.iter().map(|x| x.interrogations).collect();

        if let Some(week_pattern_id) = self.week_pattern {
            let pattern = week_patterns.get_pattern(week_pattern_id);
            for (i, base_status) in base_pattern.iter_mut().enumerate() {
                let week_pattern_status = match pattern.get(first_week + i) {
                    Some(val) => *val,
                    None => true,
                };
                if !week_pattern_status {
                    *base_status = false;
                }
            }
        }

        base_pattern
    }
}

impl Slots {
    /// Builds a [Slots] from dense per-subject rows (used by storage decode).
    ///
    /// `entries` provides one row per subject with interrogations: the subject
    /// id and its ordered slots (in the intended order). Each slot must already
    /// carry the matching `subject_id`. Returns an error if a slot id appears
    /// more than once across all rows — otherwise the two backend structures
    /// would silently desynchronize.
    pub fn from_subject_rows(
        entries: impl IntoIterator<Item = (SubjectId, Vec<(SlotId, Slot)>)>,
    ) -> Result<Self, DuplicatedSlotIdError> {
        let mut slot_map = Table::new();
        let mut ordering = Table::new();
        for (subject_id, slots) in entries {
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

    /// Iterator over the subjects that have interrogations (dense-key semantics), in id order.
    pub fn subjects_with_slots(&self) -> impl Iterator<Item = SubjectId> + '_ {
        self.ordering.keys()
    }

    /// Whether the subject is a valid subject with interrogations (has an ordering entry).
    pub fn has_interrogations(&self, subject_id: SubjectId) -> bool {
        self.ordering.contains(&subject_id)
    }

    /// Whether there is no subject with interrogations at all.
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

    /// Registers a subject as having interrogations (empty ordering entry).
    pub(crate) fn add_subject_entry(&mut self, subject_id: SubjectId) {
        self.ordering.insert(subject_id, Vec::new());
    }

    /// Deregisters a subject's interrogations.
    ///
    /// The subject must have no remaining slots.
    pub(crate) fn remove_subject_entry(&mut self, subject_id: SubjectId) {
        let previous = self.ordering.remove(&subject_id);
        debug_assert!(
            previous.map(|order| order.is_empty()).unwrap_or(true),
            "removing a subject entry that still has slots"
        );
    }

    /// Inserts a slot at `position` within its subject's ordering.
    ///
    /// The subject is taken from `slot.subject_id` and must already be registered.
    pub(crate) fn insert_slot_at(&mut self, slot_id: SlotId, slot: Slot, position: usize) {
        let subject_id = slot.subject_id;
        self.ordering
            .get_mut(&subject_id)
            .expect("subject should be registered before inserting a slot")
            .insert(position, slot_id);
        self.slot_map.insert(slot_id, slot);
    }

    /// Removes a slot, returning its former position (within its subject) and data.
    pub(crate) fn remove_slot(&mut self, slot_id: SlotId) -> (usize, Slot) {
        let slot = self.slot_map.remove(&slot_id).expect("slot should exist");
        let order = self
            .ordering
            .get_mut(&slot.subject_id)
            .expect("slot's subject should be registered");
        let pos = order
            .iter()
            .position(|id| *id == slot_id)
            .expect("slot should appear in its subject's ordering");
        order.remove(pos);
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

                if !self.inner_data.params.slots.has_interrogations(subject_id) {
                    return Err(SlotError::SubjectHasNoInterrogation(subject_id));
                }

                self.inner_data
                    .params
                    .slots
                    .insert_slot_at(*new_id, slot.clone(), position);

                let subject = self
                    .inner_data
                    .params
                    .subjects
                    .find_subject(subject_id)
                    .expect("Subject ID should be valid at this point");
                for (period_id, period) in &mut self.inner_data.colloscope.period_map {
                    if subject.excluded_periods.contains(period_id) {
                        continue;
                    }

                    period.slot_map.insert(
                        *new_id,
                        colloscopes::ColloscopeSlot::new_empty_from_params(
                            &self.inner_data.params,
                            *period_id,
                            *new_id,
                        ),
                    );
                }

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

                for (period_id, collo_period) in &self.inner_data.colloscope.period_map {
                    let Some(collo_slot) = collo_period.slot_map.get(id) else {
                        continue;
                    };

                    if !collo_slot.is_empty() {
                        return Err(SlotError::NotEmptySlotInColloscope(*id, *period_id));
                    }
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
                for collo_period in self.inner_data.colloscope.period_map.values_mut() {
                    // The slot might not be in period but this won't raise an error
                    collo_period.slot_map.remove(id);
                }

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
                let pattern = self
                    .inner_data
                    .params
                    .get_merged_pattern(new_slot.week_pattern);
                if !self.inner_data.colloscope.check_empty_on_removed_weeks(
                    *slot_id,
                    &self.inner_data.params.periods,
                    &pattern[..],
                ) {
                    return Err(SlotError::NotCompatibleSlotInColloscope(
                        *slot_id,
                        new_slot.week_pattern,
                    ));
                }

                let old_slot = self
                    .inner_data
                    .params
                    .slots
                    .replace_slot(*slot_id, new_slot.clone());
                self.inner_data.colloscope.update_slot_for_week_pattern(
                    *slot_id,
                    &self.inner_data.params.periods,
                    &pattern[..],
                );

                Ok(AnnotatedSlotOp::Update(*slot_id, old_slot))
            }
        }
    }
}
