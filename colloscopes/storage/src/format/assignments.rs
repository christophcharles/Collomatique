//! The `Assignments` block (spec §4.5)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec, UniqueVec};

/// Which students take which subject on which period, keyed by
/// `(period_id, subject_id)`
///
/// The key set is derived: the meaningful keys are exactly the
/// (period × subject not excluded from that period) pairs. A row with an
/// empty `students` array is valid but redundant (same meaning as its
/// absence).
///
/// Default: no student is assigned to anything.
pub type Assignments = KeyedVec<Assignment>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignment {
    pub period_id: u64,
    pub subject_id: u64,
    pub students: UniqueVec<u64>,
}

impl KeyedRow for Assignment {
    type Key = (u64, u64);

    fn key(&self) -> (u64, u64) {
        (self.period_id, self.subject_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn spec_example() -> serde_json::Value {
        json!([
            { "period_id": 1, "subject_id": 2, "students": [5, 6] }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: Assignments = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(Assignments::default()).unwrap(),
            json!([])
        );
    }

    #[test]
    fn duplicate_pair_key_is_rejected() {
        let value = json!([
            { "period_id": 1, "subject_id": 2, "students": [5] },
            { "period_id": 1, "subject_id": 2, "students": [6] }
        ]);
        assert!(serde_json::from_value::<Assignments>(value).is_err());
    }

    #[test]
    fn same_subject_on_two_periods_is_accepted() {
        let value = json!([
            { "period_id": 1, "subject_id": 2, "students": [5] },
            { "period_id": 3, "subject_id": 2, "students": [5] }
        ]);
        assert!(serde_json::from_value::<Assignments>(value).is_ok());
    }

    #[test]
    fn duplicate_student_is_rejected() {
        let value = json!([
            { "period_id": 1, "subject_id": 2, "students": [5, 5] }
        ]);
        assert!(serde_json::from_value::<Assignments>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({ "period_id": 1, "subject_id": 2 });
        assert!(serde_json::from_value::<Assignment>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({ "period_id": 1, "subject_id": 2, "students": [], "extra": 1 });
        assert!(serde_json::from_value::<Assignment>(value).is_err());
    }
}
