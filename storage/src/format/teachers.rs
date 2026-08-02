//! The `Teachers` block (spec §4.3)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec, UniqueVec};
use super::scalars::explicit_option;
use non_empty_string::NonEmptyString;

/// The teachers, keyed by `id`
///
/// Default: no teachers.
pub type Teachers = KeyedVec<Teacher>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Teacher {
    pub id: u64,
    pub surname: String,
    pub firstname: String,
    #[serde(deserialize_with = "explicit_option")]
    pub tel: Option<NonEmptyString>,
    #[serde(deserialize_with = "explicit_option")]
    pub email: Option<NonEmptyString>,
    /// Subjects the teacher can interrogate in
    pub subjects: UniqueVec<u64>,
}

impl KeyedRow for Teacher {
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
                "id": 4,
                "surname": "Rogue",
                "firstname": "Severus",
                "tel": "0605060708",
                "email": null,
                "subjects": [2]
            }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: Teachers = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(Teachers::default()).unwrap(),
            json!([])
        );
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let value = json!([
            {
                "id": 4,
                "surname": "Rogue",
                "firstname": "Severus",
                "tel": null,
                "email": null,
                "subjects": []
            },
            {
                "id": 4,
                "surname": "McGonagall",
                "firstname": "Minerva",
                "tel": null,
                "email": null,
                "subjects": []
            }
        ]);
        assert!(serde_json::from_value::<Teachers>(value).is_err());
    }

    #[test]
    fn missing_optional_field_is_rejected() {
        // `tel` is optional in value (`null`) but the field itself must
        // be present
        let value = json!({
            "id": 4,
            "surname": "Rogue",
            "firstname": "Severus",
            "email": null,
            "subjects": []
        });
        assert!(serde_json::from_value::<Teacher>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({
            "id": 4,
            "surname": "Rogue",
            "firstname": "Severus",
            "tel": null,
            "email": null,
            "subjects": [],
            "extra": 1
        });
        assert!(serde_json::from_value::<Teacher>(value).is_err());
    }

    #[test]
    fn empty_contact_info_is_rejected() {
        let value = json!({
            "id": 4,
            "surname": "Rogue",
            "firstname": "Severus",
            "tel": "",
            "email": null,
            "subjects": []
        });
        assert!(serde_json::from_value::<Teacher>(value).is_err());
    }
}
