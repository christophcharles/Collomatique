//! incompats submodule
//!
//! This module defines the incompats entry for the JSON description
//!
use super::*;

use collomatique_state_colloscopes::ids::Id;

use std::collections::BTreeMap;
use std::num::NonZeroU32;

/// JSON desc of incompats
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    /// map between incompat ids and corresponding schedule incompatibilities
    #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
    pub incompat_map: BTreeMap<u64, Incompatibility>,
}

/// JSON desc of a single schedule incompat
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incompatibility {
    pub subject_id: u64,
    pub name: String,
    pub slots: Vec<IncompatibilitySlot>,
    pub minimum_free_slots: NonZeroU32,
    pub week_pattern_id: Option<u64>,
}

/// JSON desc of a slot for a schedule incompat
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncompatibilitySlot {
    pub start_day: chrono::Weekday,
    pub start_time: chrono::NaiveTime,
    pub duration: NonZeroU32,
}

impl<SubjectId: Id, WeekPatternId: Id>
    From<&collomatique_state_colloscopes::incompats::Incompatibility<SubjectId, WeekPatternId>>
    for Incompatibility
{
    fn from(
        value: &collomatique_state_colloscopes::incompats::Incompatibility<
            SubjectId,
            WeekPatternId,
        >,
    ) -> Self {
        Incompatibility {
            subject_id: value.subject_id.inner(),
            name: value.name.clone(),
            slots: value
                .slots
                .iter()
                .map(|slot_with_duration| IncompatibilitySlot {
                    start_day: slot_with_duration.start().weekday.0,
                    start_time: slot_with_duration.start().start_time.inner().clone(),
                    duration: slot_with_duration.duration().get(),
                })
                .collect(),
            minimum_free_slots: value.minimum_free_slots,
            week_pattern_id: value.week_pattern_id.map(|x| x.inner()),
        }
    }
}

pub enum IncompatDecodeError {
    SlotOverlapsWithNextDay,
    TimeNotToTheMinute,
}

impl TryFrom<Incompatibility>
    for collomatique_state_colloscopes::incompats::IncompatibilityExternalData
{
    type Error = IncompatDecodeError;

    fn try_from(value: Incompatibility) -> Result<Self, IncompatDecodeError> {
        let mut slots = vec![];

        for incompatibility_slot in value.slots {
            slots.push(
                collomatique_time::SlotWithDuration::new(
                    collomatique_time::SlotStart {
                        weekday: collomatique_time::Weekday(incompatibility_slot.start_day),
                        start_time: collomatique_time::TimeOnMinutes::new(
                            incompatibility_slot.start_time,
                        )
                        .ok_or(IncompatDecodeError::TimeNotToTheMinute)?,
                    },
                    incompatibility_slot.duration.into(),
                )
                .ok_or(IncompatDecodeError::SlotOverlapsWithNextDay)?,
            );
        }
        Ok(
            collomatique_state_colloscopes::incompats::IncompatibilityExternalData {
                subject_id: value.subject_id,
                name: value.name,
                slots,
                minimum_free_slots: value.minimum_free_slots,
                week_pattern_id: value.week_pattern_id,
            },
        )
    }
}
