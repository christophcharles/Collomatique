use super::*;

#[test]
fn zero_duration_fails() {
    let duration = NonZeroDurationInMinutes::new(0);

    assert_eq!(duration, None);
}

#[test]
fn one_minute_duration_succeeds() {
    let duration = NonZeroDurationInMinutes::new(1);

    assert_eq!(
        duration,
        Some(NonZeroDurationInMinutes(NonZeroU32::new(1).unwrap()))
    );
}

#[test]
fn max_duration_succeeds() {
    let duration = NonZeroDurationInMinutes::new(u32::MAX);

    assert_eq!(
        duration,
        Some(NonZeroDurationInMinutes(NonZeroU32::new(u32::MAX).unwrap()))
    );
}
