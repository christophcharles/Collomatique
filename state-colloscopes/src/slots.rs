//! Slots submodule
//!
//! This module defines the relevant types to describes the interrogation slots

use std::collections::{BTreeMap, BTreeSet};

use crate::ids::{
    ColloscopeSlotId, ColloscopeSubjectId, ColloscopeTeacherId, ColloscopeWeekPatternId, Id,
};

/// Description of the interrogation slots
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Builds an interrogation slot from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [SlotExternalData::validate_all] and [SlotExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: SlotExternalData) -> Self {
        Slot {
            teacher_id: unsafe { TeacherId::new(external_data.teacher_id) },
            start_time: external_data.start_time,
            extra_info: external_data.extra_info,
            week_pattern: external_data
                .week_pattern
                .map(|x| unsafe { WeekPatternId::new(x) }),
            cost: external_data.cost,
        }
    }

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
    /// Builds interrogation slots for a subject from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [SubjectSlotsExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: SubjectSlotsExternalData) -> Self {
        SubjectSlots {
            ordered_slots: external_data
                .ordered_slots
                .into_iter()
                .map(|(id, slot)| {
                    (unsafe { SlotId::new(id) }, unsafe {
                        Slot::from_external_data(slot)
                    })
                })
                .collect(),
        }
    }

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
    /// Builds interrogation slots from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [SlotsExternalData::validate_all] and [SlotsExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: SlotsExternalData) -> Self {
        Slots {
            subject_map: external_data
                .subject_map
                .into_iter()
                .map(|(id, slots)| {
                    (unsafe { SubjectId::new(id) }, unsafe {
                        SubjectSlots::from_external_data(slots)
                    })
                })
                .collect(),
        }
    }

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

/// Description of the interrogation slots but unchecked
///
/// This structure is an unchecked equivalent of [Slots].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SlotsExternalData {
    /// Slots for each subject
    ///
    /// Each item associates a subject id to a collection of slots
    /// There should be an entry for each valid subject with interrogations
    pub subject_map: BTreeMap<u64, SubjectSlotsExternalData>,
}

impl SlotsExternalData {
    /// Checks the validity of all [SlotsExternalData] in the map.
    pub fn validate_all(
        &self,
        subjects: &super::subjects::SubjectsExternalData,
        week_pattern_ids: &BTreeSet<u64>,
        teachers: &super::teachers::TeachersExternalData,
    ) -> bool {
        let subjects_with_interrogations_count = subjects
            .ordered_subject_list
            .iter()
            .filter(|(_id, subject)| subject.parameters.interrogation_parameters.is_some())
            .count();
        if self.subject_map.len() != subjects_with_interrogations_count {
            return false;
        }
        self.subject_map.iter().all(|(subject_id, data)| {
            data.validate(*subject_id, subjects, teachers, week_pattern_ids)
        })
    }
}

/// Description of the interrogation slots for a subject but unchecked
///
/// This structure is an unchecked equivalent of [SubjectSlots].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectSlotsExternalData {
    /// Slots for the subject in order
    pub ordered_slots: Vec<(u64, SlotExternalData)>,
}

impl SubjectSlotsExternalData {
    /// Checks the validity of a [SubjectSlotsExternalData].
    pub fn validate(
        &self,
        subject_id: u64,
        subjects: &super::subjects::SubjectsExternalData,
        teachers: &super::teachers::TeachersExternalData,
        week_pattern_ids: &BTreeSet<u64>,
    ) -> bool {
        self.ordered_slots
            .iter()
            .all(|(_id, slot)| slot.validate(subject_id, subjects, week_pattern_ids, teachers))
    }
}

impl<SlotId: Id, TeacherId: Id, WeekPatternId: Id>
    From<SubjectSlots<SlotId, TeacherId, WeekPatternId>> for SubjectSlotsExternalData
{
    fn from(value: SubjectSlots<SlotId, TeacherId, WeekPatternId>) -> Self {
        SubjectSlotsExternalData {
            ordered_slots: value
                .ordered_slots
                .into_iter()
                .map(|(id, slot)| (id.inner(), slot.into()))
                .collect(),
        }
    }
}

/// Description of a single slot but unchecked
///
/// This structure is an unchecked equivalent of [Slot].
/// The main difference is that there are no garantees for the
/// validity of the ids.
///
/// This should be used when extracting from a file for instance
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotExternalData {
    /// Teacher for the interrogation
    pub teacher_id: u64,
    /// Day and start time for the interrogation
    /// The duration is fixed by the subject
    pub start_time: collomatique_time::SlotStart,
    /// Extra info that can be exported (like the room number)
    pub extra_info: String,
    /// Week pattern for the interrogation
    ///
    /// If None, the interrogation happens everyweek
    pub week_pattern: Option<u64>,
    /// Cost for the interrogation
    ///
    /// An optional cost can be defined. By default, this will be 0.
    /// But a positive cost can be chosen to avoid a slot.
    /// A negative cost would rather favor a given slot
    pub cost: i32,
}

impl SlotExternalData {
    /// Checks the validity of a [SlotExternalData]
    pub fn validate(
        &self,
        subject_id: u64,
        subjects: &super::subjects::SubjectsExternalData,
        week_pattern_ids: &BTreeSet<u64>,
        teachers: &super::teachers::TeachersExternalData,
    ) -> bool {
        let Some(subject) = subjects.find_subject(subject_id) else {
            return false;
        };
        let Some(params) = &subject.parameters.interrogation_parameters else {
            return false;
        };

        let Some(teacher) = teachers.teacher_map.get(&self.teacher_id) else {
            return false;
        };
        if !teacher.subjects.contains(&subject_id) {
            return false;
        }
        if let Some(week_pattern_id) = &self.week_pattern {
            if !week_pattern_ids.contains(week_pattern_id) {
                return false;
            }
        }
        if collomatique_time::SlotWithDuration::new(
            self.start_time.clone(),
            params.duration.clone(),
        )
        .is_none()
        {
            return false;
        }
        true
    }
}

impl<TeacherId: Id, WeekPatternId: Id> From<Slot<TeacherId, WeekPatternId>> for SlotExternalData {
    fn from(value: Slot<TeacherId, WeekPatternId>) -> Self {
        SlotExternalData {
            teacher_id: value.teacher_id.inner(),
            start_time: value.start_time,
            extra_info: value.extra_info,
            week_pattern: value.week_pattern.map(|x| x.inner()),
            cost: value.cost,
        }
    }
}
