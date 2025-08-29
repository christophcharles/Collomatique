//! incompats submodule
//!
//! This module defines the incompats entry for the JSON description
//!
use super::*;

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
    pub start_day: chrono::Weekday,
    pub start_time: chrono::NaiveTime,
    pub duration: NonZeroU32,
    pub week_pattern_id: Option<u64>,
}

impl From<&collomatique_state_colloscopes::incompats::Incompatibility> for Incompatibility {
    fn from(value: &collomatique_state_colloscopes::incompats::Incompatibility) -> Self {
        Incompatibility {
            subject_id: value.subject_id.inner(),
            start_day: value.slot.start().weekday.0,
            start_time: value.slot.start().start_time.clone(),
            duration: value.slot.duration().get(),
            week_pattern_id: value.week_pattern_id.map(|x| x.inner()),
        }
    }
}

pub enum IncompatDecodeError {
    SlotOverlapsWithNextDay,
}

impl TryFrom<Incompatibility>
    for collomatique_state_colloscopes::incompats::IncompatibilityExternalData
{
    type Error = IncompatDecodeError;

    fn try_from(value: Incompatibility) -> Result<Self, IncompatDecodeError> {
        let start = collomatique_time::SlotStart {
            weekday: collomatique_time::Weekday(value.start_day),
            start_time: value.start_time,
        };
        let duration = value.duration.into();
        let slot = collomatique_time::SlotWithDuration::new(start, duration)
            .ok_or(IncompatDecodeError::SlotOverlapsWithNextDay)?;
        Ok(
            collomatique_state_colloscopes::incompats::IncompatibilityExternalData {
                subject_id: value.subject_id,
                slot,
                week_pattern_id: value.week_pattern_id,
            },
        )
    }
}
