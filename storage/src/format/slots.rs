//! The `Slots` block (spec §4.7)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec};
use super::scalars::{DayTime, explicit_option};

/// Interrogation slots, grouped by subject, keyed by `subject_id`
///
/// The key set is derived: the meaningful keys are exactly the subjects
/// with interrogations. A row with an empty `slots` array is valid but
/// redundant (same meaning as its absence).
///
/// Default: no subject has any slots.
pub type Slots = KeyedVec<SubjectSlots>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectSlots {
    pub subject_id: u64,
    /// Order-significant (user order)
    pub slots: Vec<Slot>,
}

impl KeyedRow for SubjectSlots {
    type Key = u64;

    fn key(&self) -> u64 {
        self.subject_id
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slot {
    pub id: u64,
    pub teacher_id: u64,
    /// The duration comes from the subject's `duration_minutes`
    pub start: DayTime,
    /// Free info for exports (room number…), may be empty
    pub extra_info: String,
    /// `null` = the slot exists every week
    #[serde(deserialize_with = "explicit_option")]
    pub week_pattern_id: Option<u64>,
    /// Solver preference: positive avoids the slot, negative favours it
    pub cost: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_example() -> serde_json::Value {
        json!([
            {
                "subject_id": 2,
                "slots": [
                    {
                        "id": 8,
                        "teacher_id": 4,
                        "start": { "day": "monday", "time": "14:00" },
                        "extra_info": "Salle 101",
                        "week_pattern_id": 7,
                        "cost": 0
                    }
                ]
            }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: Slots = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(serde_json::to_value(Slots::default()).unwrap(), json!([]));
    }

    #[test]
    fn duplicate_subject_id_is_rejected() {
        let value = json!([
            { "subject_id": 2, "slots": [] },
            { "subject_id": 2, "slots": [] }
        ]);
        assert!(serde_json::from_value::<Slots>(value).is_err());
    }

    #[test]
    fn negative_cost_is_accepted() {
        let value = json!({
            "id": 8,
            "teacher_id": 4,
            "start": { "day": "monday", "time": "14:00" },
            "extra_info": "",
            "week_pattern_id": null,
            "cost": -10
        });
        let slot: Slot = serde_json::from_value(value).unwrap();
        assert_eq!(slot.cost, -10);
    }

    #[test]
    fn missing_optional_field_is_rejected() {
        // `week_pattern_id` is optional in value (`null`) but the field
        // itself must be present
        let value = json!({
            "id": 8,
            "teacher_id": 4,
            "start": { "day": "monday", "time": "14:00" },
            "extra_info": "",
            "cost": 0
        });
        assert!(serde_json::from_value::<Slot>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({
            "id": 8,
            "teacher_id": 4,
            "start": { "day": "monday", "time": "14:00" },
            "extra_info": "",
            "week_pattern_id": null,
            "cost": 0,
            "extra": 1
        });
        assert!(serde_json::from_value::<Slot>(value).is_err());
    }
}
