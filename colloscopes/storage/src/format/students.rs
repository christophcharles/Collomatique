//! The `Students` block (spec §4.4)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec, UniqueVec};
use super::scalars::explicit_option;
use non_empty_string::NonEmptyString;

/// The students, keyed by `id`
///
/// Default: no students.
pub type Students = KeyedVec<Student>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Student {
    pub id: u64,
    pub surname: String,
    pub firstname: String,
    #[serde(deserialize_with = "explicit_option")]
    pub tel: Option<NonEmptyString>,
    #[serde(deserialize_with = "explicit_option")]
    pub email: Option<NonEmptyString>,
    /// Periods the student does not attend at all
    pub excluded_periods: UniqueVec<u64>,
}

impl KeyedRow for Student {
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
                "id": 5,
                "surname": "Granger",
                "firstname": "Hermione",
                "tel": null,
                "email": "hermione@poudlard.fr",
                "excluded_periods": [1]
            }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: Students = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(Students::default()).unwrap(),
            json!([])
        );
    }

    #[test]
    fn duplicate_id_is_rejected() {
        let value = json!([
            {
                "id": 5,
                "surname": "Granger",
                "firstname": "Hermione",
                "tel": null,
                "email": null,
                "excluded_periods": []
            },
            {
                "id": 5,
                "surname": "Potter",
                "firstname": "Harry",
                "tel": null,
                "email": null,
                "excluded_periods": []
            }
        ]);
        assert!(serde_json::from_value::<Students>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({
            "id": 5,
            "surname": "Granger",
            "firstname": "Hermione",
            "tel": null,
            "email": null
        });
        assert!(serde_json::from_value::<Student>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        // In particular a teacher-shaped row (`subjects`) is not a
        // student row
        let value = json!({
            "id": 5,
            "surname": "Granger",
            "firstname": "Hermione",
            "tel": null,
            "email": null,
            "excluded_periods": [],
            "subjects": []
        });
        assert!(serde_json::from_value::<Student>(value).is_err());
    }

    // `missing_field_is_rejected` drops `excluded_periods`, which serde
    // rejects on its own. The two `Option` fields only reject a missing key
    // because of their `explicit_option` attribute: lose it and serde
    // silently defaults them to `None`, so "no phone number" and "the field
    // was never written" stop being distinguishable. One pin per field.

    #[test]
    fn missing_tel_is_rejected() {
        let value = json!({
            "id": 5,
            "surname": "Granger",
            "firstname": "Hermione",
            "email": null,
            "excluded_periods": []
        });
        assert!(serde_json::from_value::<Student>(value).is_err());
    }

    #[test]
    fn missing_email_is_rejected() {
        let value = json!({
            "id": 5,
            "surname": "Granger",
            "firstname": "Hermione",
            "tel": null,
            "excluded_periods": []
        });
        assert!(serde_json::from_value::<Student>(value).is_err());
    }
}
