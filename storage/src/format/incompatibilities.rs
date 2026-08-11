//! The `Incompatibilities` block (spec §4.8)

use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

use super::keyed::{KeyedRow, KeyedVec};
use super::scalars::{DurationMinutes, TimeOfDay, Weekday, explicit_option};

/// Recurring external commitments making students unavailable, keyed by
/// `id`
///
/// Default: no incompatibilities.
pub type Incompatibilities = KeyedVec<Incompatibility>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Incompatibility {
    pub id: u64,
    pub subject_id: u64,
    pub name: String,
    /// Order-significant: time slots when students may be unavailable
    pub slots: Vec<IncompatibilitySlot>,
    /// How many of `slots` must be kept free
    pub minimum_free_slots: NonZeroU32,
    /// `null` = applies every week
    #[serde(deserialize_with = "explicit_option")]
    pub week_pattern_id: Option<u64>,
}

impl KeyedRow for Incompatibility {
    type Key = u64;

    fn key(&self) -> u64 {
        self.id
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncompatibilitySlot {
    pub day: Weekday,
    pub time: TimeOfDay,
    pub duration_minutes: DurationMinutes,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_example() -> serde_json::Value {
        json!([
            {
                "id": 9,
                "subject_id": 2,
                "name": "Option latin",
                "slots": [
                    { "day": "monday", "time": "08:00", "duration_minutes": 60 },
                    { "day": "thursday", "time": "10:00", "duration_minutes": 90 }
                ],
                "minimum_free_slots": 1,
                "week_pattern_id": null
            }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: Incompatibilities = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(Incompatibilities::default()).unwrap(),
            json!([])
        );
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let value = json!([
            {
                "id": 9,
                "subject_id": 2,
                "name": "A",
                "slots": [],
                "minimum_free_slots": 1,
                "week_pattern_id": null
            },
            {
                "id": 9,
                "subject_id": 2,
                "name": "B",
                "slots": [],
                "minimum_free_slots": 1,
                "week_pattern_id": null
            }
        ]);
        assert!(serde_json::from_value::<Incompatibilities>(value).is_err());
    }

    #[test]
    fn zero_minimum_free_slots_is_rejected() {
        let value = json!({
            "id": 9,
            "subject_id": 2,
            "name": "A",
            "slots": [],
            "minimum_free_slots": 0,
            "week_pattern_id": null
        });
        assert!(serde_json::from_value::<Incompatibility>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({
            "id": 9,
            "subject_id": 2,
            "name": "A",
            "slots": [],
            "minimum_free_slots": 1
        });
        assert!(serde_json::from_value::<Incompatibility>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({
            "day": "monday",
            "time": "08:00",
            "duration_minutes": 60,
            "extra": 1
        });
        assert!(serde_json::from_value::<IncompatibilitySlot>(value).is_err());
    }
}
