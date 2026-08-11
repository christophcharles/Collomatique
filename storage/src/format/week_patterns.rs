//! The `WeekPatterns` block (spec §4.6)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec, UniqueVec};

/// Named week masks used by slots and incompatibilities, keyed by `id`
///
/// Default: no week patterns.
pub type WeekPatterns = KeyedVec<WeekPattern>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeekPattern {
    pub id: u64,
    pub name: String,
    /// The weeks the pattern turns off, by week id; every other week of
    /// the schedule is active. Sparse, so it says nothing about weeks
    /// added or removed later.
    pub excluded_weeks: UniqueVec<u64>,
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
                "excluded_weeks": [5, 9]
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
            { "id": 7, "name": "A", "excluded_weeks": [] },
            { "id": 7, "name": "B", "excluded_weeks": [] }
        ]);
        assert!(serde_json::from_value::<WeekPatterns>(value).is_err());
    }

    #[test]
    fn duplicate_excluded_week_is_rejected() {
        let value = json!({ "id": 7, "name": "A", "excluded_weeks": [5, 5] });
        assert!(serde_json::from_value::<WeekPattern>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({ "id": 7, "name": "A" });
        assert!(serde_json::from_value::<WeekPattern>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({ "id": 7, "name": "A", "excluded_weeks": [], "extra": 1 });
        assert!(serde_json::from_value::<WeekPattern>(value).is_err());
    }
}
