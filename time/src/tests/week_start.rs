use super::*;

// ============================================================================
// Construction tests - new()
// ============================================================================

#[test]
fn new_accepts_monday() {
    let monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    assert!(WeekStart::new(monday).is_some());
}

#[test]
fn new_rejects_tuesday() {
    let tuesday = chrono::NaiveDate::from_ymd_opt(2025, 9, 2).unwrap();
    assert!(WeekStart::new(tuesday).is_none());
}

#[test]
fn new_rejects_wednesday() {
    let wednesday = chrono::NaiveDate::from_ymd_opt(2025, 9, 3).unwrap();
    assert!(WeekStart::new(wednesday).is_none());
}

#[test]
fn new_rejects_thursday() {
    let thursday = chrono::NaiveDate::from_ymd_opt(2025, 9, 4).unwrap();
    assert!(WeekStart::new(thursday).is_none());
}

#[test]
fn new_rejects_friday() {
    let friday = chrono::NaiveDate::from_ymd_opt(2025, 9, 5).unwrap();
    assert!(WeekStart::new(friday).is_none());
}

#[test]
fn new_rejects_saturday() {
    let saturday = chrono::NaiveDate::from_ymd_opt(2025, 9, 6).unwrap();
    assert!(WeekStart::new(saturday).is_none());
}

#[test]
fn new_rejects_sunday() {
    let sunday = chrono::NaiveDate::from_ymd_opt(2025, 9, 7).unwrap();
    assert!(WeekStart::new(sunday).is_none());
}

#[test]
fn new_accepts_first_monday_of_year() {
    // January 6, 2025 is a Monday
    let monday = chrono::NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
    assert!(WeekStart::new(monday).is_some());
}

#[test]
fn new_accepts_last_monday_of_year() {
    // December 29, 2025 is a Monday
    let monday = chrono::NaiveDate::from_ymd_opt(2025, 12, 29).unwrap();
    assert!(WeekStart::new(monday).is_some());
}

// ============================================================================
// Construction tests - round_from()
// ============================================================================

#[test]
fn round_from_returns_same_date_for_monday() {
    let monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    let week_start = WeekStart::round_from(monday);

    assert_eq!(*week_start.monday(), monday);
}

#[test]
fn round_from_rounds_tuesday_to_monday() {
    let tuesday = chrono::NaiveDate::from_ymd_opt(2025, 9, 2).unwrap();
    let expected_monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    let week_start = WeekStart::round_from(tuesday);

    assert_eq!(*week_start.monday(), expected_monday);
}

#[test]
fn round_from_rounds_wednesday_to_monday() {
    let wednesday = chrono::NaiveDate::from_ymd_opt(2025, 9, 3).unwrap();
    let expected_monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    let week_start = WeekStart::round_from(wednesday);

    assert_eq!(*week_start.monday(), expected_monday);
}

#[test]
fn round_from_rounds_thursday_to_monday() {
    let thursday = chrono::NaiveDate::from_ymd_opt(2025, 9, 4).unwrap();
    let expected_monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    let week_start = WeekStart::round_from(thursday);

    assert_eq!(*week_start.monday(), expected_monday);
}

#[test]
fn round_from_rounds_friday_to_monday() {
    let friday = chrono::NaiveDate::from_ymd_opt(2025, 9, 5).unwrap();
    let expected_monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    let week_start = WeekStart::round_from(friday);

    assert_eq!(*week_start.monday(), expected_monday);
}

#[test]
fn round_from_rounds_saturday_to_monday() {
    let saturday = chrono::NaiveDate::from_ymd_opt(2025, 9, 6).unwrap();
    let expected_monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    let week_start = WeekStart::round_from(saturday);

    assert_eq!(*week_start.monday(), expected_monday);
}

#[test]
fn round_from_rounds_sunday_to_monday() {
    let sunday = chrono::NaiveDate::from_ymd_opt(2025, 9, 7).unwrap();
    let expected_monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    let week_start = WeekStart::round_from(sunday);

    assert_eq!(*week_start.monday(), expected_monday);
}

#[test]
fn round_from_handles_year_boundary() {
    // January 2, 2025 is a Thursday
    // Should round to December 30, 2024 (Monday)
    let thursday = chrono::NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
    let expected_monday = chrono::NaiveDate::from_ymd_opt(2024, 12, 30).unwrap();
    let week_start = WeekStart::round_from(thursday);

    assert_eq!(*week_start.monday(), expected_monday);
}

#[test]
fn round_from_handles_month_boundary() {
    // October 1, 2025 is a Wednesday
    // Should round to September 29, 2025 (Monday)
    let wednesday = chrono::NaiveDate::from_ymd_opt(2025, 10, 1).unwrap();
    let expected_monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 29).unwrap();
    let week_start = WeekStart::round_from(wednesday);

    assert_eq!(*week_start.monday(), expected_monday);
}

// ============================================================================
// Construction tests - from_today()
// ============================================================================

#[test]
fn from_today_returns_a_monday() {
    let week_start = WeekStart::from_today();

    use chrono::Datelike;
    // Verify that the returned date is actually a Monday
    assert_eq!(week_start.monday().weekday(), chrono::Weekday::Mon);
}

#[test]
fn from_today_is_consistent_with_round_from() {
    let today = chrono::Local::now().naive_local().date();
    let from_today = WeekStart::from_today();
    let from_round = WeekStart::round_from(today);

    assert_eq!(*from_today.monday(), *from_round.monday());
}

// ============================================================================
// Accessor tests
// ============================================================================

#[test]
fn monday_returns_correct_date() {
    let monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    let week_start = WeekStart::new(monday).unwrap();

    assert_eq!(*week_start.monday(), monday);
}

#[test]
fn into_monday_consumes_and_returns_date() {
    let monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    let week_start = WeekStart::new(monday).unwrap();

    let extracted = week_start.into_monday();
    assert_eq!(extracted, monday);
}

#[test]
fn deref_allows_access_to_naive_date_methods() {
    let monday = chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
    let week_start = WeekStart::new(monday).unwrap();

    use chrono::Datelike;
    // Should be able to call NaiveDate methods directly
    assert_eq!(week_start.year(), 2025);
    assert_eq!(week_start.month(), 9);
    assert_eq!(week_start.day(), 1);
}

// ============================================================================
// Ordering and equality tests
// ============================================================================

#[test]
fn ordering_by_date() {
    let week1 = WeekStart::new(chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap()).unwrap();
    let week2 = WeekStart::new(chrono::NaiveDate::from_ymd_opt(2025, 9, 8).unwrap()).unwrap();

    assert!(week1 < week2);
}

#[test]
fn ordering_across_months() {
    let week1 = WeekStart::new(chrono::NaiveDate::from_ymd_opt(2025, 9, 29).unwrap()).unwrap();
    let week2 = WeekStart::new(chrono::NaiveDate::from_ymd_opt(2025, 10, 6).unwrap()).unwrap();

    assert!(week1 < week2);
}

#[test]
fn ordering_across_years() {
    let week1 = WeekStart::new(chrono::NaiveDate::from_ymd_opt(2024, 12, 30).unwrap()).unwrap();
    let week2 = WeekStart::new(chrono::NaiveDate::from_ymd_opt(2025, 1, 6).unwrap()).unwrap();

    assert!(week1 < week2);
}

#[test]
fn equality() {
    let week1 = WeekStart::new(chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap()).unwrap();
    let week2 = WeekStart::new(chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap()).unwrap();

    assert_eq!(week1, week2);
}

#[test]
fn inequality() {
    let week1 = WeekStart::new(chrono::NaiveDate::from_ymd_opt(2025, 9, 1).unwrap()).unwrap();
    let week2 = WeekStart::new(chrono::NaiveDate::from_ymd_opt(2025, 9, 8).unwrap()).unwrap();

    assert_ne!(week1, week2);
}
