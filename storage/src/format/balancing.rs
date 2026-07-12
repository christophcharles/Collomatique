//! The `Balancing` block (spec §4.14)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec};
use super::scalars::{SoftFlag, explicit_option};

/// Global and per-subject balancing options for the solver
///
/// Default: soft teacher rotation, avoid-twice-in-a-row, nothing else.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Balancing {
    pub global: Options,
    /// Keyed by `subject_id`; each row overrides the global options for
    /// that subject. The key set is free: a row is an override,
    /// whatever its values.
    pub subjects: KeyedVec<SubjectOverride>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    /// Rotate teachers across groups (`null` = off)
    #[serde(deserialize_with = "explicit_option")]
    pub teacher_rotation: Option<SoftFlag>,
    /// Rotate time slots across groups (`null` = off)
    #[serde(deserialize_with = "explicit_option")]
    pub slot_rotation: Option<SoftFlag>,
    pub avoid_twice_in_a_row: bool,
    pub year_teacher_rotation: bool,
    pub period_teacher_rotation: bool,
}

// The spec's frozen default (§4.14), NOT the all-off record
impl Default for Options {
    fn default() -> Self {
        Options {
            teacher_rotation: Some(SoftFlag { soft: true }),
            slot_rotation: None,
            avoid_twice_in_a_row: true,
            year_teacher_rotation: false,
            period_teacher_rotation: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectOverride {
    pub subject_id: u64,
    pub options: Options,
}

impl KeyedRow for SubjectOverride {
    type Key = u64;

    fn key(&self) -> u64 {
        self.subject_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(Balancing::default()).unwrap(),
            json!({
                "global": {
                    "teacher_rotation": { "soft": true },
                    "slot_rotation": null,
                    "avoid_twice_in_a_row": true,
                    "year_teacher_rotation": false,
                    "period_teacher_rotation": false
                },
                "subjects": []
            })
        );
    }

    #[test]
    fn populated_block_round_trips() {
        let value = json!({
            "global": {
                "teacher_rotation": null,
                "slot_rotation": { "soft": false },
                "avoid_twice_in_a_row": false,
                "year_teacher_rotation": true,
                "period_teacher_rotation": false
            },
            "subjects": [
                {
                    "subject_id": 2,
                    "options": {
                        "teacher_rotation": { "soft": true },
                        "slot_rotation": null,
                        "avoid_twice_in_a_row": true,
                        "year_teacher_rotation": false,
                        "period_teacher_rotation": true
                    }
                }
            ]
        });
        let block: Balancing = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), value);
    }

    #[test]
    fn duplicate_subject_id_is_rejected() {
        let options = json!({
            "teacher_rotation": null,
            "slot_rotation": null,
            "avoid_twice_in_a_row": false,
            "year_teacher_rotation": false,
            "period_teacher_rotation": false
        });
        let value = json!({
            "global": options,
            "subjects": [
                { "subject_id": 2, "options": options },
                { "subject_id": 2, "options": options }
            ]
        });
        assert!(serde_json::from_value::<Balancing>(value).is_err());
    }

    #[test]
    fn rotation_with_a_value_is_rejected() {
        let value = json!({
            "teacher_rotation": { "soft": true, "value": 3 },
            "slot_rotation": null,
            "avoid_twice_in_a_row": true,
            "year_teacher_rotation": false,
            "period_teacher_rotation": false
        });
        assert!(serde_json::from_value::<Options>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({
            "teacher_rotation": null,
            "slot_rotation": null,
            "avoid_twice_in_a_row": true,
            "year_teacher_rotation": false
        });
        assert!(serde_json::from_value::<Options>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({
            "teacher_rotation": null,
            "slot_rotation": null,
            "avoid_twice_in_a_row": true,
            "year_teacher_rotation": false,
            "period_teacher_rotation": false,
            "extra": 1
        });
        assert!(serde_json::from_value::<Options>(value).is_err());
    }
}
