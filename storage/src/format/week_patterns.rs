//! The `WeekPatterns` block (spec §4.6)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec};

/// Named week masks used by slots and incompatibilities, keyed by `id`
///
/// Default: no week patterns.
pub type WeekPatterns = KeyedVec<WeekPattern>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeekPattern {
    pub id: u64,
    pub name: String,
    /// Positional: `weeks[w]` = pattern active on global week `w`. Well-formed
    /// files carry exactly one element per week of the schedule (as produced by
    /// the encoder); decode maps each bit to its week in global order, so any
    /// surplus bits are ignored and missing ones default to active.
    pub weeks: Vec<bool>,
}

impl KeyedRow for WeekPattern {
    type Key = u64;

    fn key(&self) -> u64 {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_example() -> serde_json::Value {
        json!([
            {
                "id": 7,
                "name": "Quinzaine A",
                "weeks": [true, false, true, false, true, false, true]
            }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: WeekPatterns = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(WeekPatterns::default()).unwrap(),
            json!([])
        );
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let value = json!([
            { "id": 7, "name": "A", "weeks": [] },
            { "id": 7, "name": "B", "weeks": [] }
        ]);
        assert!(serde_json::from_value::<WeekPatterns>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({ "id": 7, "name": "A" });
        assert!(serde_json::from_value::<WeekPattern>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({ "id": 7, "name": "A", "weeks": [], "extra": 1 });
        assert!(serde_json::from_value::<WeekPattern>(value).is_err());
    }
}
