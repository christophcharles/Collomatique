use super::*;

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).unwrap()
}

// ============================================================================
// Construction tests
// ============================================================================

#[test]
fn none_is_unlimited() {
    let limit = TimeLimit::none();

    assert!(limit.is_none());
    assert!(!limit.is_some());
    assert_eq!(limit.get_seconds(), None);
}

#[test]
fn default_is_unlimited() {
    let limit = TimeLimit::default();

    assert!(limit.is_none());
    assert_eq!(limit, TimeLimit::none());
}

#[test]
fn seconds_stores_value() {
    let limit = TimeLimit::seconds(nz(30));

    assert!(limit.is_some());
    assert_eq!(limit.get_seconds(), Some(nz(30)));
}

#[test]
fn minutes_convert_to_seconds() {
    let limit = TimeLimit::minutes(nz(2));

    assert_eq!(limit.get_seconds(), Some(nz(120)));
}

#[test]
fn minutes_saturate_on_overflow() {
    let limit = TimeLimit::minutes(nz(u32::MAX));

    assert_eq!(limit.get_seconds(), Some(nz(u32::MAX)));
}

// ============================================================================
// Boundary idiom for raw u32
// ============================================================================

#[test]
fn raw_zero_maps_to_unlimited() {
    let raw: u32 = 0;
    let limit = NonZeroU32::new(raw)
        .map(TimeLimit::seconds)
        .unwrap_or_default();

    assert!(limit.is_none());
}

#[test]
fn raw_nonzero_maps_to_limit() {
    let raw: u32 = 45;
    let limit = NonZeroU32::new(raw)
        .map(TimeLimit::seconds)
        .unwrap_or_default();

    assert_eq!(limit.get_seconds(), Some(nz(45)));
}

// ============================================================================
// Duration conversion
// ============================================================================

#[test]
fn duration_none_when_unlimited() {
    assert_eq!(TimeLimit::none().duration(), None);
}

#[test]
fn duration_matches_seconds() {
    let limit = TimeLimit::seconds(nz(90));

    assert_eq!(limit.duration(), Some(std::time::Duration::from_secs(90)));
}

// ============================================================================
// Serde round-trips and validation
// ============================================================================

#[test]
fn serde_round_trip_some() {
    let limit = TimeLimit::seconds(nz(30));
    let json = serde_json::to_string(&limit).unwrap();
    let back: TimeLimit = serde_json::from_str(&json).unwrap();

    assert_eq!(limit, back);
}

#[test]
fn serde_round_trip_none() {
    let limit = TimeLimit::none();
    let json = serde_json::to_string(&limit).unwrap();
    let back: TimeLimit = serde_json::from_str(&json).unwrap();

    assert_eq!(limit, back);
}

#[test]
fn serde_rejects_zero_value() {
    // A JSON `0` inside the value is not a valid `NonZeroU32`.
    let result: Result<TimeLimit, _> = serde_json::from_str("0");

    assert!(result.is_err());
}
