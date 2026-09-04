use super::*;
use serde_json::json;

fn spec_example() -> serde_json::Value {
    json!([
        { "subject_id": 3, "week_pattern_id": 12 }
    ])
}

#[test]
fn spec_example_round_trips() {
    let block: SubjectWeekPatterns = serde_json::from_value(spec_example()).unwrap();
    assert_eq!(serde_json::to_value(&block).unwrap(), spec_example());
}

#[test]
fn default_is_pinned() {
    assert_eq!(
        serde_json::to_value(SubjectWeekPatterns::default()).unwrap(),
        json!([])
    );
}

#[test]
fn duplicate_subject_id_is_rejected() {
    let value = json!([
        { "subject_id": 3, "week_pattern_id": 12 },
        { "subject_id": 3, "week_pattern_id": 13 }
    ]);
    assert!(serde_json::from_value::<SubjectWeekPatterns>(value).is_err());
}

#[test]
fn same_week_pattern_twice_is_accepted() {
    // Reusability is the point: several subjects share one pause pattern.
    let value = json!([
        { "subject_id": 3, "week_pattern_id": 12 },
        { "subject_id": 4, "week_pattern_id": 12 }
    ]);
    assert!(serde_json::from_value::<SubjectWeekPatterns>(value).is_ok());
}

#[test]
fn missing_field_is_rejected() {
    let value = json!({ "subject_id": 3 });
    assert!(serde_json::from_value::<SubjectWeekPattern>(value).is_err());
}

#[test]
fn unknown_field_is_rejected() {
    let value = json!({ "subject_id": 3, "week_pattern_id": 12, "extra": 1 });
    assert!(serde_json::from_value::<SubjectWeekPattern>(value).is_err());
}
