//! The `Subjects` block (spec §4.2)

use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

use super::keyed::UniqueVec;
use super::scalars::{DurationMinutes, Range, explicit_option};

/// The subjects, in user order (order-significant)
///
/// Default: no subjects.
pub type Subjects = Vec<Subject>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub id: u64,
    pub name: String,
    /// `null` means the subject has no interrogations (it still exists
    /// for assignments)
    #[serde(deserialize_with = "explicit_option")]
    pub interrogation_parameters: Option<InterrogationParameters>,
    pub excluded_periods: UniqueVec<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InterrogationParameters {
    pub students_per_group: Range<NonZeroU32>,
    pub groups_per_interrogation: Range<NonZeroU32>,
    pub duration_minutes: DurationMinutes,
    pub take_duration_into_account: bool,
    pub periodicity: Periodicity,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Periodicity {
    OnceForEveryBlockOfWeeks(OnceForEveryBlockOfWeeks),
    ExactlyPeriodic(ExactlyPeriodic),
    AmountInYear(AmountInYear),
    AmountForEveryArbitraryBlock(AmountForEveryArbitraryBlock),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnceForEveryBlockOfWeeks {
    pub weeks_per_block: NonZeroU32,
    /// Cannot be 0: at most one interrogation per block already forbids
    /// two in the same week
    pub minimum_week_separation: NonZeroU32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactlyPeriodic {
    pub periodicity_in_weeks: NonZeroU32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmountInYear {
    pub interrogation_count_in_year: Range<u32>,
    /// `0` allows two interrogations in one week
    pub minimum_week_separation: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmountForEveryArbitraryBlock {
    /// Order-significant; may be empty (no interrogations can be
    /// scheduled)
    pub blocks: Vec<PeriodicityBlock>,
    pub minimum_week_separation: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeriodicityBlock {
    /// Weeks since the end of the previous block (or since week 0 for
    /// the first block)
    pub delay_in_weeks: u32,
    pub size_in_weeks: NonZeroU32,
    pub interrogation_count_in_block: Range<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_example() -> serde_json::Value {
        json!([
            {
                "id": 2,
                "name": "Mathématiques",
                "interrogation_parameters": {
                    "students_per_group": { "min": 2, "max": 3 },
                    "groups_per_interrogation": { "min": 1, "max": 1 },
                    "duration_minutes": 60,
                    "take_duration_into_account": true,
                    "periodicity": { "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }
                },
                "excluded_periods": []
            },
            {
                "id": 3,
                "name": "Sport",
                "interrogation_parameters": null,
                "excluded_periods": [1]
            }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: Subjects = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(Subjects::default()).unwrap(),
            json!([])
        );
    }

    #[test]
    fn all_periodicity_variants_round_trip() {
        let variants = [
            json!({ "OnceForEveryBlockOfWeeks": { "weeks_per_block": 3, "minimum_week_separation": 1 } }),
            json!({ "ExactlyPeriodic": { "periodicity_in_weeks": 2 } }),
            json!({ "AmountInYear": {
                "interrogation_count_in_year": { "min": 0, "max": 5 },
                "minimum_week_separation": 0
            } }),
            json!({ "AmountForEveryArbitraryBlock": {
                "blocks": [
                    {
                        "delay_in_weeks": 0,
                        "size_in_weeks": 4,
                        "interrogation_count_in_block": { "min": 1, "max": 2 }
                    }
                ],
                "minimum_week_separation": 1
            } }),
        ];
        for value in variants {
            let periodicity: Periodicity = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(&periodicity).unwrap(), value);
        }
    }

    #[test]
    fn periodicity_with_two_variant_keys_is_rejected() {
        let value = json!({
            "ExactlyPeriodic": { "periodicity_in_weeks": 2 },
            "AmountInYear": {
                "interrogation_count_in_year": { "min": 0, "max": 5 },
                "minimum_week_separation": 0
            }
        });
        assert!(serde_json::from_value::<Periodicity>(value).is_err());
    }

    #[test]
    fn once_for_every_block_rejects_zero_separation() {
        let value = json!({
            "OnceForEveryBlockOfWeeks": { "weeks_per_block": 3, "minimum_week_separation": 0 }
        });
        assert!(serde_json::from_value::<Periodicity>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({
            "id": 2,
            "name": "Mathématiques",
            "excluded_periods": []
        });
        assert!(serde_json::from_value::<Subject>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({
            "id": 3,
            "name": "Sport",
            "interrogation_parameters": null,
            "excluded_periods": [],
            "extra": 1
        });
        assert!(serde_json::from_value::<Subject>(value).is_err());
    }

    #[test]
    fn duplicate_excluded_period_is_rejected() {
        let value = json!({
            "id": 3,
            "name": "Sport",
            "interrogation_parameters": null,
            "excluded_periods": [1, 1]
        });
        assert!(serde_json::from_value::<Subject>(value).is_err());
    }
}
