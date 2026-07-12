//! The `Settings` block (spec §4.13)

use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

use super::keyed::{KeyedRow, KeyedVec};
use super::scalars::{SoftParam, explicit_option};

/// Global and per-student interrogation-load limits
///
/// Default: no limits at all.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub global: Limits,
    /// Keyed by `student_id`; each row overrides the global limits for
    /// that student. The key set is free: a row existing is itself
    /// state (an override exists), whatever its values.
    pub students: KeyedVec<StudentOverride>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    #[serde(deserialize_with = "explicit_option")]
    pub interrogations_per_week_min: Option<SoftParam<u32>>,
    #[serde(deserialize_with = "explicit_option")]
    pub interrogations_per_week_max: Option<SoftParam<u32>>,
    #[serde(deserialize_with = "explicit_option")]
    pub max_interrogations_per_day: Option<SoftParam<NonZeroU32>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudentOverride {
    pub student_id: u64,
    pub limits: Limits,
}

impl KeyedRow for StudentOverride {
    type Key = u64;

    fn key(&self) -> u64 {
        self.student_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_example() -> serde_json::Value {
        json!({
            "global": {
                "interrogations_per_week_min": { "soft": false, "value": 1 },
                "interrogations_per_week_max": { "soft": true, "value": 4 },
                "max_interrogations_per_day": { "soft": false, "value": 2 }
            },
            "students": [
                {
                    "student_id": 5,
                    "limits": {
                        "interrogations_per_week_min": null,
                        "interrogations_per_week_max": { "soft": true, "value": 3 },
                        "max_interrogations_per_day": null
                    }
                }
            ]
        })
    }

    #[test]
    fn spec_example_round_trips() {
        let block: Settings = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(Settings::default()).unwrap(),
            json!({
                "global": {
                    "interrogations_per_week_min": null,
                    "interrogations_per_week_max": null,
                    "max_interrogations_per_day": null
                },
                "students": []
            })
        );
    }

    #[test]
    fn duplicate_student_id_is_rejected() {
        let value = json!({
            "global": {
                "interrogations_per_week_min": null,
                "interrogations_per_week_max": null,
                "max_interrogations_per_day": null
            },
            "students": [
                {
                    "student_id": 5,
                    "limits": {
                        "interrogations_per_week_min": null,
                        "interrogations_per_week_max": null,
                        "max_interrogations_per_day": null
                    }
                },
                {
                    "student_id": 5,
                    "limits": {
                        "interrogations_per_week_min": null,
                        "interrogations_per_week_max": null,
                        "max_interrogations_per_day": null
                    }
                }
            ]
        });
        assert!(serde_json::from_value::<Settings>(value).is_err());
    }

    #[test]
    fn per_week_limits_accept_zero_but_per_day_does_not() {
        let limits: Limits = serde_json::from_value(json!({
            "interrogations_per_week_min": { "soft": false, "value": 0 },
            "interrogations_per_week_max": null,
            "max_interrogations_per_day": null
        }))
        .unwrap();
        assert_eq!(
            limits.interrogations_per_week_min,
            Some(SoftParam {
                soft: false,
                value: 0
            })
        );

        assert!(
            serde_json::from_value::<Limits>(json!({
                "interrogations_per_week_min": null,
                "interrogations_per_week_max": null,
                "max_interrogations_per_day": { "soft": false, "value": 0 }
            }))
            .is_err()
        );
    }

    #[test]
    fn missing_optional_field_is_rejected() {
        // A limits record has exactly the three fields, `null` included
        let value = json!({
            "interrogations_per_week_min": null,
            "interrogations_per_week_max": null
        });
        assert!(serde_json::from_value::<Limits>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({
            "interrogations_per_week_min": null,
            "interrogations_per_week_max": null,
            "max_interrogations_per_day": null,
            "extra": 1
        });
        assert!(serde_json::from_value::<Limits>(value).is_err());
    }
}
