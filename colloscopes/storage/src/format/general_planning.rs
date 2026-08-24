//! The `GeneralPlanning` block (spec §4.1)

use serde::{Deserialize, Serialize};

use super::scalars::{WeekStartDate, explicit_option};
use non_empty_string::NonEmptyString;

/// The period structure and start date
///
/// Default: no start date, no periods.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct GeneralPlanning {
    #[serde(deserialize_with = "explicit_option")]
    pub first_week: Option<WeekStartDate>,
    /// Order-significant: the period order, and the week order inside
    /// each period, define the schedule's display order
    pub periods: Vec<Period>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Period {
    pub id: u64,
    /// Order-significant: the weeks of the period, in order (may be empty)
    pub weeks: Vec<Week>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Week {
    /// This block defines week ids, like it defines period ids
    pub id: u64,
    pub interrogations: bool,
    #[serde(deserialize_with = "explicit_option")]
    pub annotation: Option<NonEmptyString>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_example() -> serde_json::Value {
        json!({
            "first_week": "2026-08-31",
            "periods": [
                {
                    "id": 1,
                    "weeks": [
                        { "id": 4, "interrogations": true, "annotation": "Rentrée" },
                        { "id": 5, "interrogations": false, "annotation": null }
                    ]
                }
            ]
        })
    }

    #[test]
    fn spec_example_round_trips() {
        let block: GeneralPlanning = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(GeneralPlanning::default()).unwrap(),
            json!({ "first_week": null, "periods": [] })
        );
    }

    #[test]
    fn missing_optional_field_is_rejected() {
        // `first_week` is optional in value (`null`) but the field itself
        // must be present
        assert!(serde_json::from_value::<GeneralPlanning>(json!({ "periods": [] })).is_err());
        assert!(
            serde_json::from_value::<Week>(json!({ "id": 4, "interrogations": true })).is_err()
        );
    }

    #[test]
    fn unknown_field_is_rejected() {
        assert!(
            serde_json::from_value::<GeneralPlanning>(
                json!({ "first_week": null, "periods": [], "extra": 1 })
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<Week>(
                json!({ "id": 4, "interrogations": true, "annotation": null, "extra": 1 })
            )
            .is_err()
        );
    }

    #[test]
    fn empty_annotation_is_rejected() {
        assert!(
            serde_json::from_value::<Week>(
                json!({ "id": 4, "interrogations": true, "annotation": "" })
            )
            .is_err()
        );
    }
}
