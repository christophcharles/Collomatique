//! Slots submodule
//!
//! This module defines the relevant types to describes the interrogation slots

use std::collections::{BTreeMap, BTreeSet};

use crate::ids::{SlotId, SubjectId, TeacherId, WeekPatternId};

/// Description of the interrogation slots
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Slots {
    /// Slots for each subject
    ///
    /// Each item associates a subject id to a collection of slots
    /// There should be an entry for each valid subject with interrogations
    pub subject_map: BTreeMap<SubjectId, SubjectSlots>,
}

/// Description of the interrogation slots for a subject
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SubjectSlots {
    /// Slots for the subject in order
    pub ordered_slots: Vec<(SlotId, Slot)>,
}

/// Description of a single slot
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

impl Slot {
    /// Builds an interrogation slot from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [SlotExternalData::validate_all] and [SlotExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: SlotExternalData) -> Slot {
        Slot {
            teacher_id: unsafe { TeacherId::new(external_data.teacher_id) },
            start_time: external_data.start_time,
            extra_info: external_data.extra_info,
            week_pattern: external_data
                .week_pattern
                .map(|x| unsafe { WeekPatternId::new(x) }),
        }
    }
}

impl SubjectSlots {
    /// Builds interrogation slots for a subject from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [SubjectSlotsExternalData::validate].
    pub(crate) unsafe fn from_external_data(
        external_data: SubjectSlotsExternalData,
    ) -> SubjectSlots {
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
}

impl Slots {
    /// Builds interrogation slots from external data
    ///
    /// No checks is done for consistency so this is unsafe.
    /// See [SlotsExternalData::validate_all] and [SlotsExternalData::validate].
    pub(crate) unsafe fn from_external_data(external_data: SlotsExternalData) -> Slots {
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

impl From<SubjectSlots> for SubjectSlotsExternalData {
    fn from(value: SubjectSlots) -> Self {
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

impl From<Slot> for SlotExternalData {
    fn from(value: Slot) -> Self {
        SlotExternalData {
            teacher_id: value.teacher_id.inner(),
            start_time: value.start_time,
            extra_info: value.extra_info,
            week_pattern: value.week_pattern.map(|x| x.inner()),
        }
    }
}
