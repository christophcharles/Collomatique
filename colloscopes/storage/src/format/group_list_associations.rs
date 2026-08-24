//! The `GroupListAssociations` block (spec §4.10)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec};

/// Which group list a subject uses on a period, keyed by
/// `(period_id, subject_id)`
///
/// The key set is free: every present row carries real state
/// (`group_list_id`), so there is no neutral-content rule here.
///
/// Default: no associations.
pub type GroupListAssociations = KeyedVec<GroupListAssociation>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GroupListAssociation {
    pub period_id: u64,
    pub subject_id: u64,
    pub group_list_id: u64,
}

impl KeyedRow for GroupListAssociation {
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
            { "period_id": 1, "subject_id": 2, "group_list_id": 10 }
        ])
    }

    #[test]
    fn spec_example_round_trips() {
        let block: GroupListAssociations = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(GroupListAssociations::default()).unwrap(),
            json!([])
        );
    }

    #[test]
    fn duplicate_pair_key_is_rejected() {
        let value = json!([
            { "period_id": 1, "subject_id": 2, "group_list_id": 10 },
            { "period_id": 1, "subject_id": 2, "group_list_id": 11 }
        ]);
        assert!(serde_json::from_value::<GroupListAssociations>(value).is_err());
    }

    #[test]
    fn same_group_list_twice_is_accepted() {
        let value = json!([
            { "period_id": 1, "subject_id": 2, "group_list_id": 10 },
            { "period_id": 3, "subject_id": 2, "group_list_id": 10 }
        ]);
        assert!(serde_json::from_value::<GroupListAssociations>(value).is_ok());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({ "period_id": 1, "subject_id": 2 });
        assert!(serde_json::from_value::<GroupListAssociation>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({
            "period_id": 1,
            "subject_id": 2,
            "group_list_id": 10,
            "extra": 1
        });
        assert!(serde_json::from_value::<GroupListAssociation>(value).is_err());
    }
}
