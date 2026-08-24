//! The `GroupLists` block (spec §4.9)

use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

use super::keyed::{KeyedRow, KeyedVec, UniqueVec};
use super::scalars::Range;
use non_empty_string::NonEmptyString;

/// The group lists themselves, keyed by `id` (their association to
/// subjects is the separate `GroupListAssociations` block)
///
/// Default: no group lists.
pub type GroupLists = KeyedVec<GroupList>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupList {
    pub id: u64,
    pub name: String,
    pub students_per_group: Range<NonZeroU32>,
    /// Order-significant; its length is the group count and group
    /// numbers used elsewhere are 0-based indices into it. `null` =
    /// unnamed group.
    pub group_names: Vec<Option<NonEmptyString>>,
    pub filling: Filling,
}

impl KeyedRow for GroupList {
    type Key = u64;

    fn key(&self) -> u64 {
        self.id
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Filling {
    Prefilled(Prefilled),
    Automatic(Automatic),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prefilled {
    /// Order-significant, aligned with `group_names` (same length — a
    /// constraint checked at a later layer)
    pub groups: Vec<Group>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub students: UniqueVec<u64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Automatic {
    pub excluded_students: UniqueVec<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_example() -> serde_json::Value {
        json!([
            {
                "id": 10,
                "name": "Groupes de maths",
                "students_per_group": { "min": 2, "max": 3 },
                "group_names": ["Gryffondor", null],
                "filling": {
                    "Prefilled": {
                        "groups": [
                            { "students": [5, 6] },
                            { "students": [] }
                        ]
                    }
                }
            },
            {
                "id": 11,
                "name": "Groupes de physique",
                "students_per_group": { "min": 1, "max": 2 },
                "group_names": [null, null, null],
                "filling": { "Automatic": { "excluded_students": [6] } }
            }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: GroupLists = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(GroupLists::default()).unwrap(),
            json!([])
        );
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let value = json!([
            {
                "id": 10,
                "name": "A",
                "students_per_group": { "min": 1, "max": 2 },
                "group_names": [],
                "filling": { "Automatic": { "excluded_students": [] } }
            },
            {
                "id": 10,
                "name": "B",
                "students_per_group": { "min": 1, "max": 2 },
                "group_names": [],
                "filling": { "Automatic": { "excluded_students": [] } }
            }
        ]);
        assert!(serde_json::from_value::<GroupLists>(value).is_err());
    }

    #[test]
    fn filling_with_two_variant_keys_is_rejected() {
        let value = json!({
            "Prefilled": { "groups": [] },
            "Automatic": { "excluded_students": [] }
        });
        assert!(serde_json::from_value::<Filling>(value).is_err());
    }

    #[test]
    fn empty_group_name_is_rejected() {
        let value = json!({
            "id": 10,
            "name": "A",
            "students_per_group": { "min": 1, "max": 2 },
            "group_names": [""],
            "filling": { "Automatic": { "excluded_students": [] } }
        });
        assert!(serde_json::from_value::<GroupList>(value).is_err());
    }

    #[test]
    fn duplicate_student_in_a_group_is_rejected() {
        let value = json!({ "students": [5, 5] });
        assert!(serde_json::from_value::<Group>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({
            "id": 10,
            "name": "A",
            "students_per_group": { "min": 1, "max": 2 },
            "group_names": []
        });
        assert!(serde_json::from_value::<GroupList>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({
            "id": 10,
            "name": "A",
            "students_per_group": { "min": 1, "max": 2 },
            "group_names": [],
            "filling": { "Automatic": { "excluded_students": [] } },
            "extra": 1
        });
        assert!(serde_json::from_value::<GroupList>(value).is_err());
    }
}
