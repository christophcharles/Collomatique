use super::*;

#[test]
fn works_for_monday_2025_09_01() {
    assert!(NaiveMondayDate::new(chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap()).is_some());
}

#[test]
fn fails_for_tuesday_2025_09_02() {
    assert!(NaiveMondayDate::new(chrono::NaiveDate::from_ymd_opt(2025, 9, 2).unwrap()).is_none());
}
