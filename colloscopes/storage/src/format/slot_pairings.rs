//! The `SlotPairings` block (spec §4.12)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec, UniqueVec};

/// Implication rules between two slots of the same subject, keyed by
/// `id`: "if the antecedent slot is used on some week, the consequent
/// condition must hold that week"
///
/// Default: no slot pairing rules.
pub type SlotPairings = KeyedVec<SlotPairing>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotPairing {
    pub id: u64,
    pub antecedent: SlotPairingPart,
    pub consequent: SlotPairingPart,
    /// Periods where the rule does not apply
    pub excluded_periods: UniqueVec<u64>,
    /// `true` = best-effort (optimized), `false` = hard constraint
    pub soft: bool,
}

impl KeyedRow for SlotPairing {
    type Key = u64;

    fn key(&self) -> u64 {
        self.id
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlotPairingPart {
    pub slot_id: u64,
    /// `true` = "the slot is used that week", `false` = "it is not"
    pub should_have: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_example() -> serde_json::Value {
        json!([
            {
                "id": 14,
                "antecedent": { "slot_id": 8, "should_have": true },
                "consequent": { "slot_id": 15, "should_have": true },
                "excluded_periods": [],
                "soft": false
            }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: SlotPairings = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(SlotPairings::default()).unwrap(),
            json!([])
        );
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let value = json!([
            {
                "id": 14,
                "antecedent": { "slot_id": 8, "should_have": true },
                "consequent": { "slot_id": 15, "should_have": true },
                "excluded_periods": [],
                "soft": false
            },
            {
                "id": 14,
                "antecedent": { "slot_id": 15, "should_have": true },
                "consequent": { "slot_id": 8, "should_have": true },
                "excluded_periods": [],
                "soft": false
            }
        ]);
        assert!(serde_json::from_value::<SlotPairings>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({ "slot_id": 8 });
        assert!(serde_json::from_value::<SlotPairingPart>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        // In particular a subject-pairing-shaped part (`subject_id`) is
        // not a slot pairing part
        let value = json!({ "subject_id": 8, "should_have": true });
        assert!(serde_json::from_value::<SlotPairingPart>(value).is_err());
    }
}
