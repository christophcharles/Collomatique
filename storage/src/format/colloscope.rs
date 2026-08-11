//! The `Colloscope` block (spec §4.15)

use serde::{Deserialize, Serialize};

use super::keyed::{KeyedRow, KeyedVec, UniqueVec};

/// The colloscope itself: which groups sit which interrogation, and how
/// automatic group lists were filled
///
/// Default: an unsolved colloscope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Colloscope {
    /// Keyed by `(slot_id, week_id)`, with a derived key set: the cells
    /// that can host an interrogation are fully determined by the other
    /// blocks. A row with an empty `assigned_groups` is valid but
    /// redundant (same meaning as its absence).
    pub interrogations: KeyedVec<Interrogation>,
    /// Keyed by `group_list_id`, with a derived key set: the meaningful
    /// keys are exactly the automatic (non-prefilled) group lists. A
    /// row with an empty `students` is valid but redundant.
    pub group_lists: KeyedVec<FilledGroupList>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interrogation {
    pub slot_id: u64,
    /// The week of the interrogation (which also determines the period)
    pub week_id: u64,
    /// 0-based group numbers
    pub assigned_groups: UniqueVec<u32>,
}

impl KeyedRow for Interrogation {
    type Key = (u64, u64);

    fn key(&self) -> (u64, u64) {
        (self.slot_id, self.week_id)
    }
}

/// The shape this block had before week ids — read-only
///
/// Transitional, like [super::general_planning::LegacyGeneralPlanning]: an
/// interrogation used to name its week by *global week index* rather than
/// by id. Only the interrogation rows differ; the group-list rows are
/// unchanged and shared with the current shape.
#[derive(Clone, Debug, PartialEq, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct LegacyColloscope {
    pub interrogations: KeyedVec<LegacyInterrogation>,
    pub group_lists: KeyedVec<FilledGroupList>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyInterrogation {
    pub slot_id: u64,
    /// Global week index (the week determines the period)
    pub week: u32,
    /// 0-based group numbers
    pub assigned_groups: UniqueVec<u32>,
}

impl KeyedRow for LegacyInterrogation {
    type Key = (u64, u32);

    fn key(&self) -> (u64, u32) {
        (self.slot_id, self.week)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilledGroupList {
    pub group_list_id: u64,
    pub students: KeyedVec<StudentPlacement>,
}

impl KeyedRow for FilledGroupList {
    type Key = u64;

    fn key(&self) -> u64 {
        self.group_list_id
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StudentPlacement {
    pub student_id: u64,
    /// 0-based group number
    pub group: u32,
}

impl KeyedRow for StudentPlacement {
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
            "interrogations": [
                { "slot_id": 8, "week_id": 4, "assigned_groups": [0] },
                { "slot_id": 8, "week_id": 6, "assigned_groups": [0, 1] }
            ],
            "group_lists": [
                {
                    "group_list_id": 11,
                    "students": [
                        { "student_id": 5, "group": 0 },
                        { "student_id": 6, "group": 1 }
                    ]
                }
            ]
        })
    }

    /// The same block in the shape it had before week ids
    fn legacy_example() -> serde_json::Value {
        json!({
            "interrogations": [
                { "slot_id": 8, "week": 0, "assigned_groups": [0] },
                { "slot_id": 8, "week": 2, "assigned_groups": [0, 1] }
            ],
            "group_lists": []
        })
    }

    #[test]
    fn spec_example_round_trips() {
        let block: Colloscope = serde_json::from_value(spec_example()).unwrap();
        assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
    }

    #[test]
    fn legacy_example_parses_as_legacy_only() {
        assert!(serde_json::from_value::<LegacyColloscope>(legacy_example()).is_ok());
        assert!(serde_json::from_value::<Colloscope>(legacy_example()).is_err());
        assert!(serde_json::from_value::<LegacyColloscope>(spec_example()).is_err());
    }

    #[test]
    fn default_is_pinned() {
        assert_eq!(
            serde_json::to_value(Colloscope::default()).unwrap(),
            json!({ "interrogations": [], "group_lists": [] })
        );
    }

    #[test]
    fn duplicate_cell_key_is_rejected() {
        let value = json!({
            "interrogations": [
                { "slot_id": 8, "week_id": 4, "assigned_groups": [0] },
                { "slot_id": 8, "week_id": 4, "assigned_groups": [1] }
            ],
            "group_lists": []
        });
        assert!(serde_json::from_value::<Colloscope>(value).is_err());
    }

    #[test]
    fn same_slot_on_two_weeks_is_accepted() {
        let value = json!({
            "interrogations": [
                { "slot_id": 8, "week_id": 4, "assigned_groups": [0] },
                { "slot_id": 8, "week_id": 5, "assigned_groups": [0] }
            ],
            "group_lists": []
        });
        assert!(serde_json::from_value::<Colloscope>(value).is_ok());
    }

    #[test]
    fn duplicate_assigned_group_is_rejected() {
        let value = json!({ "slot_id": 8, "week_id": 4, "assigned_groups": [0, 0] });
        assert!(serde_json::from_value::<Interrogation>(value).is_err());
    }

    #[test]
    fn duplicate_student_placement_is_rejected() {
        let value = json!({
            "group_list_id": 11,
            "students": [
                { "student_id": 5, "group": 0 },
                { "student_id": 5, "group": 1 }
            ]
        });
        assert!(serde_json::from_value::<FilledGroupList>(value).is_err());
    }

    #[test]
    fn missing_field_is_rejected() {
        let value = json!({ "slot_id": 8, "week_id": 4 });
        assert!(serde_json::from_value::<Interrogation>(value).is_err());
    }

    #[test]
    fn unknown_field_is_rejected() {
        let value = json!({ "student_id": 5, "group": 0, "extra": 1 });
        assert!(serde_json::from_value::<StudentPlacement>(value).is_err());
    }
}
