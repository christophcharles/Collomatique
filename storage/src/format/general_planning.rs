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

/// The shape this block had before weeks carried an id — read-only
///
/// Transitional: weeks used to be purely positional, and everything that
/// named a week named its *global week index* (its position in the
/// concatenation of the periods). The decoder synthesized the week ids on
/// load. This struct exists only so files written before the change still
/// open; nothing produces it, and it disappears once those files are gone.
///
/// It is exactly today's [GeneralPlanning] minus the week ids, which is
/// what makes the two tellable apart with no guessing: records deny
/// unknown fields and have no defaults, so an old week fails to parse as a
/// new one (missing `id`) and a new one fails to parse as an old one
/// (unknown field `id`).
#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct LegacyGeneralPlanning {
    #[serde(deserialize_with = "explicit_option")]
    pub first_week: Option<WeekStartDate>,
    /// Order-significant: the concatenated `weeks` arrays define
    /// global week numbering
    pub periods: Vec<LegacyPeriod>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyPeriod {
    pub id: u64,
    /// Positional: one record per week of the period (may be empty)
    pub weeks: Vec<LegacyWeek>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyWeek {
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

    /// The same document in the shape this block had before week ids
    fn legacy_example() -> serde_json::Value {
        json!({
            "first_week": "2026-08-31",
            "periods": [
                {
                    "id": 1,
                    "weeks": [
                        { "interrogations": true, "annotation": "Rentrée" },
                        { "interrogations": false, "annotation": null }
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
    fn legacy_example_parses_as_legacy_only() {
        assert!(serde_json::from_value::<LegacyGeneralPlanning>(legacy_example()).is_ok());
        // The two shapes cannot be confused: an old week has no `id`, a
        // new one has an `id` the old shape refuses.
        assert!(serde_json::from_value::<GeneralPlanning>(legacy_example()).is_err());
        assert!(serde_json::from_value::<LegacyGeneralPlanning>(spec_example()).is_err());
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
