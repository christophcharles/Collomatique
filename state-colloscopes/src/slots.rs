//! Slots submodule
//!
//! This module defines the relevant types to describes the interrogation slots

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{
    ColloscopeSlotId, ColloscopeSubjectId, ColloscopeTeacherId, ColloscopeWeekPatternId, Id,
};

/// Description of the interrogation slots
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slots<SubjectId: Id, SlotId: Id, TeacherId: Id, WeekPatternId: Id> {
    /// Slots for each subject
    ///
    /// Each item associates a subject id to a collection of slots
    /// There should be an entry for each valid subject with interrogations
    pub subject_map: BTreeMap<SubjectId, SubjectSlots<SlotId, TeacherId, WeekPatternId>>,
}

impl<SubjectId: Id, SlotId: Id, TeacherId: Id, WeekPatternId: Id> Default
    for Slots<SubjectId, SlotId, TeacherId, WeekPatternId>
{
    fn default() -> Self {
        Slots {
            subject_map: BTreeMap::new(),
        }
    }
}

/// Description of the interrogation slots for a subject
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectSlots<SlotId: Id, TeacherId: Id, WeekPatternId: Id> {
    /// Slots for the subject in order
    pub ordered_slots: Vec<(SlotId, Slot<TeacherId, WeekPatternId>)>,
}

impl<SlotId: Id, TeacherId: Id, WeekPatternId: Id> Default
    for SubjectSlots<SlotId, TeacherId, WeekPatternId>
{
    fn default() -> Self {
        SubjectSlots {
            ordered_slots: vec![],
        }
    }
}

/// Description of a single slot
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot<TeacherId: Id, WeekPatternId: Id> {
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

impl<TeacherId: Id, WeekPatternId: Id> Slot<TeacherId, WeekPatternId> {
    pub(crate) fn duplicate_with_id_maps(
        &self,
        teachers_map: &BTreeMap<TeacherId, ColloscopeTeacherId>,
        week_patterns_map: &BTreeMap<WeekPatternId, ColloscopeWeekPatternId>,
    ) -> Option<Slot<ColloscopeTeacherId, ColloscopeWeekPatternId>> {
        let week_pattern = match &self.week_pattern {
            Some(id) => Some(*week_patterns_map.get(id)?),
            None => None,
        };

        Some(Slot {
            teacher_id: *teachers_map.get(&self.teacher_id)?,
            start_time: self.start_time.clone(),
            extra_info: self.extra_info.clone(),
            week_pattern,
            cost: self.cost,
        })
    }
}

impl<SlotId: Id, TeacherId: Id, WeekPatternId: Id> SubjectSlots<SlotId, TeacherId, WeekPatternId> {
    pub fn find_slot_position(&self, slot_id: SlotId) -> Option<usize> {
        for (pos, (id, _slot)) in self.ordered_slots.iter().enumerate() {
            if slot_id == *id {
                return Some(pos);
            }
        }
        None
    }

    pub fn find_slot(&self, slot_id: SlotId) -> Option<&Slot<TeacherId, WeekPatternId>> {
        let pos = self.find_slot_position(slot_id)?;

        Some(
            &self
                .ordered_slots
                .get(pos)
                .expect("Position should be valid at this point")
                .1,
        )
    }

    pub(crate) fn duplicate_with_id_maps(
        &self,
        slots_map: &BTreeMap<SlotId, ColloscopeSlotId>,
        teachers_map: &BTreeMap<TeacherId, ColloscopeTeacherId>,
        week_patterns_map: &BTreeMap<WeekPatternId, ColloscopeWeekPatternId>,
    ) -> Option<SubjectSlots<ColloscopeSlotId, ColloscopeTeacherId, ColloscopeWeekPatternId>> {
        let mut ordered_slots = vec![];

        for (slot_id, slot) in &self.ordered_slots {
            let new_id = slots_map.get(slot_id)?;
            let new_slot = slot.duplicate_with_id_maps(teachers_map, week_patterns_map)?;
            ordered_slots.push((*new_id, new_slot));
        }

        Some(SubjectSlots { ordered_slots })
    }
}

impl<SubjectId: Id, SlotId: Id, TeacherId: Id, WeekPatternId: Id>
    Slots<SubjectId, SlotId, TeacherId, WeekPatternId>
{
    pub fn find_slot_subject_and_position(&self, slot_id: SlotId) -> Option<(SubjectId, usize)> {
        for (subject_id, subject_slots) in &self.subject_map {
            if let Some(pos) = subject_slots.find_slot_position(slot_id) {
                return Some((*subject_id, pos));
            }
        }
        None
    }

    pub fn find_slot(&self, slot_id: SlotId) -> Option<&Slot<TeacherId, WeekPatternId>> {
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

    pub(crate) fn duplicate_with_id_maps(
        &self,
        subjects_map: &BTreeMap<SubjectId, ColloscopeSubjectId>,
        slots_map: &BTreeMap<SlotId, ColloscopeSlotId>,
        teachers_map: &BTreeMap<TeacherId, ColloscopeTeacherId>,
        week_patterns_map: &BTreeMap<WeekPatternId, ColloscopeWeekPatternId>,
    ) -> Option<
        Slots<ColloscopeSubjectId, ColloscopeSlotId, ColloscopeTeacherId, ColloscopeWeekPatternId>,
    > {
        let mut subject_map = BTreeMap::new();

        for (subject_id, subject_slots) in &self.subject_map {
            let new_id = subjects_map.get(subject_id)?;
            let new_subject_slots =
                subject_slots.duplicate_with_id_maps(slots_map, teachers_map, week_patterns_map)?;
            subject_map.insert(*new_id, new_subject_slots);
        }

        Some(Slots { subject_map })
    }
}
