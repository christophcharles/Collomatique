use super::*;

// ============================================================================
// Construction tests - new()
// ============================================================================

#[test]
fn new_rejects_zero_duration() {
    let duration = NonZeroDurationInMinutes::new(0);

    assert_eq!(duration, None);
}

#[test]
fn new_accepts_one_minute_duration() {
    let duration = NonZeroDurationInMinutes::new(1);

    assert_eq!(
        duration,
        Some(NonZeroDurationInMinutes(NonZeroU32::new(1).unwrap()))
    );
}

#[test]
fn new_accepts_typical_duration() {
    let duration = NonZeroDurationInMinutes::new(90);

    assert!(duration.is_some());
    assert_eq!(duration.unwrap().get().get(), 90);
}

#[test]
fn new_accepts_max_duration() {
    let duration = NonZeroDurationInMinutes::new(u32::MAX);

    assert_eq!(
        duration,
        Some(NonZeroDurationInMinutes(NonZeroU32::new(u32::MAX).unwrap()))
    );
}

// ============================================================================
// Accessor tests
// ============================================================================

#[test]
fn get_returns_correct_value() {
    let duration = NonZeroDurationInMinutes::new(45).unwrap();

    assert_eq!(duration.get().get(), 45);
}

#[test]
fn get_returns_non_zero_u32() {
    let duration = NonZeroDurationInMinutes::new(120).unwrap();
    let value: NonZeroU32 = duration.get();

    assert_eq!(value.get(), 120);
}

// ============================================================================
// Conversion tests
// ============================================================================

#[test]
fn from_non_zero_u32() {
    let non_zero = NonZeroU32::new(60).unwrap();
    let duration: NonZeroDurationInMinutes = non_zero.into();

    assert_eq!(duration.get().get(), 60);
}

#[test]
fn time_delta_returns_correct_duration() {
    let duration = NonZeroDurationInMinutes::new(90).unwrap();
    let time_delta = duration.time_delta();

    assert_eq!(time_delta.num_minutes(), 90);
}

#[test]
fn time_delta_for_one_minute() {
    let duration = NonZeroDurationInMinutes::new(1).unwrap();
    let time_delta = duration.time_delta();

    assert_eq!(time_delta.num_minutes(), 1);
}

#[test]
fn time_delta_for_large_duration() {
    let duration = NonZeroDurationInMinutes::new(1440).unwrap(); // 24 hours
    let time_delta = duration.time_delta();

    assert_eq!(time_delta.num_minutes(), 1440);
    assert_eq!(time_delta.num_hours(), 24);
}

#[test]
fn into_time_delta() {
    let duration = NonZeroDurationInMinutes::new(75).unwrap();
    let time_delta: chrono::TimeDelta = duration.into();

    assert_eq!(time_delta.num_minutes(), 75);
}

// ============================================================================
// Display formatting tests
// ============================================================================

#[test]
fn display_shows_zero_minutes() {
    let duration = NonZeroDurationInMinutes::new(60).unwrap();

    assert_eq!(duration.to_string(), "1h00");
}

#[test]
fn display_shows_single_digit_minutes() {
    let duration = NonZeroDurationInMinutes::new(65).unwrap();

    assert_eq!(duration.to_string(), "1h05");
}

#[test]
fn display_shows_double_digit_minutes() {
    let duration = NonZeroDurationInMinutes::new(90).unwrap();

    assert_eq!(duration.to_string(), "1h30");
}

#[test]
fn display_for_less_than_one_hour() {
    let duration = NonZeroDurationInMinutes::new(45).unwrap();

    assert_eq!(duration.to_string(), "0h45");
}

#[test]
fn display_for_one_minute() {
    let duration = NonZeroDurationInMinutes::new(1).unwrap();

    assert_eq!(duration.to_string(), "0h01");
}

#[test]
fn display_for_multiple_hours() {
    let duration = NonZeroDurationInMinutes::new(185).unwrap(); // 3h05

    assert_eq!(duration.to_string(), "3h05");
}

#[test]
fn display_for_exactly_multiple_hours() {
    let duration = NonZeroDurationInMinutes::new(180).unwrap(); // 3h00

    assert_eq!(duration.to_string(), "3h00");
}

#[test]
fn display_for_full_day() {
    let duration = NonZeroDurationInMinutes::new(1440).unwrap(); // 24h00

    assert_eq!(duration.to_string(), "24h00");
}

// ============================================================================
// Ordering and equality tests
// ============================================================================

#[test]
fn ordering_less_than() {
    let duration1 = NonZeroDurationInMinutes::new(30).unwrap();
    let duration2 = NonZeroDurationInMinutes::new(60).unwrap();

    assert!(duration1 < duration2);
}

#[test]
fn ordering_greater_than() {
    let duration1 = NonZeroDurationInMinutes::new(120).unwrap();
    let duration2 = NonZeroDurationInMinutes::new(90).unwrap();

    assert!(duration1 > duration2);
}

#[test]
fn equality() {
    let duration1 = NonZeroDurationInMinutes::new(75).unwrap();
    let duration2 = NonZeroDurationInMinutes::new(75).unwrap();

    assert_eq!(duration1, duration2);
}

#[test]
fn inequality() {
    let duration1 = NonZeroDurationInMinutes::new(60).unwrap();
    let duration2 = NonZeroDurationInMinutes::new(90).unwrap();

    assert_ne!(duration1, duration2);
}
