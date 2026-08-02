//! The `Pairings` block (spec §4.11)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec, UniqueVec};

/// Implication rules between subjects, keyed by `id`: "if a student has
/// an interrogation in the antecedent subject on some week, the
/// consequent condition must hold that week"
///
/// Default: no pairing rules.
pub type Pairings = KeyedVec<Pairing>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pairing {
    pub id: u64,
    pub antecedent: PairingPart,
    pub consequent: PairingPart,
    /// Periods where the rule does not apply
    pub excluded_periods: UniqueVec<u64>,
    /// `true` = best-effort (optimized), `false` = hard constraint
    pub soft: bool,
}

impl KeyedRow for Pairing {
    type Key = u64;

    fn key(&self) -> u64 {
        self.id
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PairingPart {
    pub subject_id: u64,
    /// `true` = "has an interrogation that week", `false` = "has none"
    pub should_have: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_example() -> serde_json::Value {
        json!([
            {
                "id": 12,
                "antecedent": { "subject_id": 2, "should_have": true },
                "consequent": { "subject_id": 13, "should_have": false },
                "excluded_periods": [1],
                "soft": true
            }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: Pairings = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(Pairings::default()).unwrap(),
            json!([])
        );
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let value = json!([
            {
                "id": 12,
                "antecedent": { "subject_id": 2, "should_have": true },
                "consequent": { "subject_id": 13, "should_have": false },
                "excluded_periods": [],
                "soft": true
            },
            {
                "id": 12,
                "antecedent": { "subject_id": 13, "should_have": true },
                "consequent": { "subject_id": 2, "should_have": false },
                "excluded_periods": [],
                "soft": true
            }
        ]);
        assert!(serde_json::from_value::<Pairings>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({
            "id": 12,
            "antecedent": { "subject_id": 2, "should_have": true },
            "consequent": { "subject_id": 13, "should_have": false },
            "excluded_periods": []
        });
        assert!(serde_json::from_value::<Pairing>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        // In particular a slot-pairing-shaped part (`slot_id`) is not a
        // subject pairing part
        let value = json!({ "slot_id": 2, "should_have": true });
        assert!(serde_json::from_value::<PairingPart>(value).is_err());
    }
}
