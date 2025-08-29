//! slots submodule
//!
//! This module defines the slots entry for the JSON description
//!
use super::*;

use std::collections::BTreeMap;

/// JSON desc of slots
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct List {
    /// map between subject ids and corresponding slots
    #[serde(with = "serde_with::rust::maps_duplicate_key_is_error")]
    pub subject_map: BTreeMap<u64, SubjectSlots>,
}

/// JSON desc of the slots of a single subject
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectSlots {
    pub ordered_slot_list: Vec<(u64, Slot)>,
}

/// JSON desc of a single slot
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub teacher_id: u64,
    pub start_day: chrono::Weekday,
    pub start_time: chrono::NaiveTime,
    pub extra_info: String,
    pub week_pattern: Option<u64>,
    pub cost: i32,
}

impl From<&collomatique_state_colloscopes::slots::Slot> for Slot {
    fn from(value: &collomatique_state_colloscopes::slots::Slot) -> Self {
        Slot {
            teacher_id: value.teacher_id.inner(),
            start_day: value.start_time.weekday.0,
            start_time: value.start_time.start_time.clone(),
            extra_info: value.extra_info.clone(),
            week_pattern: value.week_pattern.map(|x| x.inner()),
            cost: value.cost,
        }
    }
}

impl From<Slot> for collomatique_state_colloscopes::slots::SlotExternalData {
    fn from(value: Slot) -> Self {
        collomatique_state_colloscopes::slots::SlotExternalData {
            teacher_id: value.teacher_id,
            start_time: collomatique_time::SlotStart {
                weekday: collomatique_time::Weekday(value.start_day),
                start_time: value.start_time,
            },
            extra_info: value.extra_info,
            week_pattern: value.week_pattern,
            cost: value.cost,
        }
    }
}
