//! The `Balancing` block (spec §4.14)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec};
use super::scalars::SoftFlag;

/// Global and per-subject balancing options for the solver
///
/// Default: a soft teacher rotation, everything else off.
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
    /// `null` = off, `{"soft": true}` = optimisation goal,
    /// `{"soft": false}` = strict constraint.
    pub teacher_rotation: Option<SoftFlag>,
    /// Same three states as [`Self::teacher_rotation`].
    pub slot_rotation: Option<SoftFlag>,
    /// Same three states as [`Self::teacher_rotation`].
    pub avoid_twice_in_a_row: Option<SoftFlag>,
    pub year_teacher_rotation: bool,
    pub period_teacher_rotation: bool,
}

// The spec's default (§4.14). It must stay equal to
// `mem::balancing::BalancingOptions::default()`: the encoder omits the block
// when it matches, and the decoder fills it back in, so the two together pin
// "a file with no Balancing block == a default document".
impl Default for Options {
    fn default() -> Self {
        Options {
            teacher_rotation: Some(SoftFlag { soft: true }),
            slot_rotation: None,
            avoid_twice_in_a_row: None,
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
                    "avoid_twice_in_a_row": null,
                    "year_teacher_rotation": false,
                    "period_teacher_rotation": false
                },
                "subjects": []
            })
        );
    }

    #[test]
    fn populated_block_round_trips() {
        // All three states of the three-state fields appear at least once.
        let value = json!({
            "global": {
                "teacher_rotation": { "soft": true },
                "slot_rotation": { "soft": false },
                "avoid_twice_in_a_row": null,
                "year_teacher_rotation": true,
                "period_teacher_rotation": false
            },
            "subjects": [
                {
                    "subject_id": 2,
                    "options": {
                        "teacher_rotation": null,
                        "slot_rotation": { "soft": true },
                        "avoid_twice_in_a_row": { "soft": false },
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
            "avoid_twice_in_a_row": null,
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
    fn bool_goal_is_rejected() {
        // A plain boolean says nothing about the third state, so it is not a
        // valid encoding of the three-state goals.
        for field in ["teacher_rotation", "slot_rotation", "avoid_twice_in_a_row"] {
            let mut value = json!({
                "teacher_rotation": null,
                "slot_rotation": null,
                "avoid_twice_in_a_row": null,
                "year_teacher_rotation": false,
                "period_teacher_rotation": false
            });
            value[field] = json!(true);
            assert!(
                serde_json::from_value::<Options>(value).is_err(),
                "a plain bool must be rejected for {field}"
            );
        }
    }

    #[test]
    fn stray_value_in_a_goal_is_rejected() {
        let value = json!({
            "teacher_rotation": { "soft": true, "value": null },
            "slot_rotation": null,
            "avoid_twice_in_a_row": null,
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
            "avoid_twice_in_a_row": { "soft": false },
            "year_teacher_rotation": false
        });
        assert!(serde_json::from_value::<Options>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({
            "teacher_rotation": null,
            "slot_rotation": null,
            "avoid_twice_in_a_row": { "soft": false },
            "year_teacher_rotation": false,
            "period_teacher_rotation": false,
            "extra": 1
        });
        assert!(serde_json::from_value::<Options>(value).is_err());
    }
}
