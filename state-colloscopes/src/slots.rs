//! Slots submodule
//!
//! This module defines the relevant types to describes the interrogation slots

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{SlotId, SubjectId, TeacherId, WeekPatternId};

/// Description of the interrogation slots
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slots {
    /// Slots for each subject
    ///
    /// Each item associates a subject id to a collection of slots
    /// There should be an entry for each valid subject with interrogations
    pub subject_map: BTreeMap<SubjectId, SubjectSlots>,
}

/// Description of the interrogation slots for a subject
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectSlots {
    /// Slots for the subject in order
    pub ordered_slots: Vec<(SlotId, Slot)>,
}

/// Description of a single slot
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
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

impl SubjectSlots {
    pub fn find_slot_position(&self, slot_id: SlotId) -> Option<usize> {
        for (pos, (id, _slot)) in self.ordered_slots.iter().enumerate() {
            if slot_id == *id {
                return Some(pos);
            }
        }
        None
    }

    pub fn find_slot(&self, slot_id: SlotId) -> Option<&Slot> {
        let pos = self.find_slot_position(slot_id)?;

        Some(
            &self
                .ordered_slots
                .get(pos)
                .expect("Position should be valid at this point")
                .1,
        )
    }
}

impl Slots {
    pub fn find_slot_subject_and_position(&self, slot_id: SlotId) -> Option<(SubjectId, usize)> {
        for (subject_id, subject_slots) in &self.subject_map {
            if let Some(pos) = subject_slots.find_slot_position(slot_id) {
                return Some((*subject_id, pos));
            }
        }
        None
    }

    pub fn find_slot(&self, slot_id: SlotId) -> Option<&Slot> {
        let (subject_id, pos) = self.find_slot_subject_and_position(slot_id)?;

        Some(
            &self
                .subject_map
                .get(&subject_id)
                .expect("Subject id should be valid at this point")
                .ordered_slots
                .get(pos)
                .expect("Position should be valid at this point")
                .1,
        )
    }
}
