//! Slots submodule
//!
//! This module defines the relevant types to describes the interrogation slots

use serde::{Deserialize, Serialize};
use thiserror::Error;

use collomatique_state::{ContentOrd, Join, References};

use crate::Table;
use crate::ids::{NewId, SlotId, SubjectId, TeacherId, WeekPatternId};
use crate::ops::AnnotatedSlotOp;

/// Description of the interrogation slots
///
/// The backend is a flat id-keyed [Table] of slots (each slot carries its
/// subject as a foreign key) plus an explicit per-subject ordering sidecar.
/// The two must stay consistent; the invariant is checked by the slot-ordering
/// `LogicError`s in `InnerData::broken_invariants`:
/// - `ordering` is sparse: a row is present exactly when the subject has at
///   least one slot (canonical form — no empty rows), and
/// - `ordering[s]` is a duplicate-free permutation of
///   `{ id | slot_map[id].subject_id == s }`.
///
/// Row-key *liveness* (the subject exists) is deliberately not part of these
/// `LogicError`s: a row keyed by a removed subject is the op-reachable dangling
/// state, reported as `DanglingFk` through the per-slot `SlotSubject` sites and
/// repaired by the cascade. The interrogation flag is not part of them either:
/// a slot on a subject without interrogations is
/// `Convergence::SlotForSubjectWithoutInterrogations`, also in the fixable tier.
///
/// All mutation goes through the compound `pub(crate)` helpers below so no
/// call site can desynchronize the two structures. The fields are private:
/// consumers read through the accessor surface further down.
#[derive(Clone, Debug, Default, PartialEq, Eq, ContentOrd)]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, References, Join, ContentOrd)]
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
    // A scalar leaf whose type is foreign (`SlotStart` cannot carry a
    // `ContentOrd` impl of ours), so the rule is inlined: same time or
    // incomparable. Moving a slot is not removing content.
    #[ord(atom)]
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
    ///
    /// Reads the ordering sidecar: the compound mutators keep it in lockstep
    /// with `slot_map`, so the two containers cover the same slots in every
    /// ops-reachable state (force ops included); only test forgery can split
    /// them. (The weeks twin reads its entity table instead — either side of
    /// the lockstep is equally authoritative.)
    pub fn is_empty(&self) -> bool {
        self.ordering.is_empty()
    }

    /// Ordered slots for a subject; `None` if the subject has no slots (no
    /// ordering row).
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

    /// Owned copy of the ordered slots for a subject; `None` if the subject has
    /// no slots (no ordering row).
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

    /// Number of slots for a subject; `None` if the subject has no slots (no
    /// ordering row).
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

// The container's half of the dense renumbering walk (see [crate::compact]).
// The two methods must visit exactly the same id occurrences — here both the
// slot table and the per-subject ordering mirror, keys and values alike.
impl Slots {
    pub(crate) fn collect_ids(&self, ids: &mut std::collections::BTreeSet<u64>) {
        use crate::ids::Id as _;
        for (slot_id, slot) in self.slot_map.iter() {
            ids.insert(slot_id.inner());
            ids.insert(slot.subject_id.inner());
            ids.insert(slot.teacher_id.inner());
            if let Some(week_pattern_id) = slot.week_pattern {
                ids.insert(week_pattern_id.inner());
            }
        }
        for (subject_id, slot_list) in self.ordering.iter() {
            ids.insert(subject_id.inner());
            for slot_id in slot_list {
                ids.insert(slot_id.inner());
            }
        }
    }

    pub(crate) fn remap_ids(self, map: &crate::compact::IdMap) -> Self {
        use crate::compact::remap;
        Slots {
            slot_map: self
                .slot_map
                .into_iter()
                .map(|(slot_id, slot)| {
                    let Slot {
                        subject_id,
                        teacher_id,
                        start_time,
                        extra_info,
                        week_pattern,
                        cost,
                    } = slot;
                    (
                        remap(map, slot_id),
                        Slot {
                            subject_id: remap(map, subject_id),
                            teacher_id: remap(map, teacher_id),
                            start_time,
                            extra_info,
                            week_pattern: week_pattern
                                .map(|week_pattern_id| remap(map, week_pattern_id)),
                            cost,
                        },
                    )
                })
                .collect(),
            ordering: self
                .ordering
                .into_iter()
                .map(|(subject_id, slot_list)| {
                    (
                        remap(map, subject_id),
                        slot_list
                            .into_iter()
                            .map(|slot_id| remap(map, slot_id))
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

/// Precondition errors of the forced slot ops — the carve-out subset. Kept:
/// no-clobber, op-target existence
/// ([Self::InvalidSlotId]), the `AddAfter` same-subject anchor
/// ([Self::PreviousSlotIsNotInRightSubject]), position bounds, and the
/// subject-immutability guard ([Self::CannotChangeSubject]). `validate_slot`,
/// the Remove colloscope/pairing scans and the Update pattern guard are
/// stripped.
#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum SlotPrecheckError {
    /// A slot id is invalid
    #[error("invalid slot id ({0:?})")]
    InvalidSlotId(SlotId),

    /// The slot id already exists
    #[error("slot id ({0:?}) already exists")]
    SlotIdAlreadyExists(SlotId),

    /// A position is outside the subject's slot list
    #[error("position {position} is outside the slot list of subject {subject:?} (size = {size})")]
    PositionOutOfBounds {
        subject: SubjectId,
        position: usize,
        size: usize,
    },

    /// The previous slot given is not for the same subject
    #[error("Slot {0:?} to be previous slot is not for subject {1:?}")]
    PreviousSlotIsNotInRightSubject(SlotId, SubjectId),

    /// An update tried to move the slot to a different subject
    #[error("slot ({0:?}) cannot change subject (from {1:?} to {2:?})")]
    CannotChangeSubject(SlotId, SubjectId, SubjectId),
}

impl crate::Data {
    /// Used internally by [crate::Data::force_apply]
    ///
    /// Force-applies a slot op: carve-out guards kept (returned as
    /// [SlotPrecheckError] — no-clobber, target existence, `AddAfter` same-subject
    /// anchor, position bounds, subject immutability), invariant guards
    /// stripped. May leave the state invalid; the caller owns checking and
    /// rollback.
    pub(crate) fn force_apply_slot(
        &mut self,
        slot_op: &AnnotatedSlotOp,
    ) -> std::result::Result<AnnotatedSlotOp, SlotPrecheckError> {
        match slot_op {
            AnnotatedSlotOp::AddAfter(new_id, after_id, slot) => {
                // The subject is authoritative from the slot itself.
                let subject_id = slot.subject_id;

                if self.inner_data.params.slots.find_slot(*new_id).is_some() {
                    return Err(SlotPrecheckError::SlotIdAlreadyExists(*new_id));
                }
                // stripped: validate_slot

                let position = match after_id {
                    Some(id) => {
                        let (sub_id, after_pos) = self
                            .inner_data
                            .params
                            .slots
                            .find_slot_subject_and_position(*id)
                            .ok_or(SlotPrecheckError::InvalidSlotId(*id))?;
                        if sub_id != subject_id {
                            return Err(SlotPrecheckError::PreviousSlotIsNotInRightSubject(
                                *id, subject_id,
                            ));
                        }

                        after_pos + 1
                    }
                    None => 0,
                };

                self.inner_data
                    .params
                    .slots
                    .insert_slot_at(*new_id, slot.clone(), position);

                Ok(AnnotatedSlotOp::Remove(*new_id))
            }
            AnnotatedSlotOp::ChangePosition(id, new_pos) => {
                let Some((subject_id, old_pos)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*id)
                else {
                    return Err(SlotPrecheckError::InvalidSlotId(*id));
                };

                let count = self
                    .inner_data
                    .params
                    .slots
                    .slot_count_for_subject(subject_id)
                    .expect("Subject id should be valid at this point");
                if *new_pos >= count {
                    return Err(SlotPrecheckError::PositionOutOfBounds {
                        subject: subject_id,
                        position: *new_pos,
                        size: count,
                    });
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
                    return Err(SlotPrecheckError::InvalidSlotId(*id));
                };

                // stripped: colloscope-row scan + slot-pairing reference scan

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

                Ok(AnnotatedSlotOp::AddAfter(*id, previous_id, old_slot))
            }
            AnnotatedSlotOp::Update(slot_id, new_slot) => {
                let Some((subject_id, _position)) = self
                    .inner_data
                    .params
                    .slots
                    .find_slot_subject_and_position(*slot_id)
                else {
                    return Err(SlotPrecheckError::InvalidSlotId(*slot_id));
                };

                // A slot cannot be moved to a different subject.
                if new_slot.subject_id != subject_id {
                    return Err(SlotPrecheckError::CannotChangeSubject(
                        *slot_id,
                        subject_id,
                        new_slot.subject_id,
                    ));
                }

                // stripped: validate_slot + the colloscope pattern-compat guard

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
