use super::*;

#[test]
fn valid_whole_minute_succeeds() {
    let time = chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap();
    let whole_minute_time = WholeMinuteTime::new(time);

    assert!(whole_minute_time.is_some());
    assert_eq!(*whole_minute_time.unwrap().inner(), time);
}

#[test]
fn time_with_seconds_fails() {
    let time = chrono::NaiveTime::from_hms_opt(14, 30, 45).unwrap();
    let whole_minute_time = WholeMinuteTime::new(time);

    assert!(whole_minute_time.is_none());
}

#[test]
fn time_with_nanoseconds_fails() {
    let time = chrono::NaiveTime::from_hms_nano_opt(14, 30, 0, 500_000_000).unwrap();
    let whole_minute_time = WholeMinuteTime::new(time);

    assert!(whole_minute_time.is_none());
}

#[test]
fn time_with_both_seconds_and_nanoseconds_fails() {
    let time = chrono::NaiveTime::from_hms_nano_opt(14, 30, 15, 250_000_000).unwrap();
    let whole_minute_time = WholeMinuteTime::new(time);

    assert!(whole_minute_time.is_none());
}

#[test]
fn midnight_succeeds() {
    let time = chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let whole_minute_time = WholeMinuteTime::new(time);

    assert!(whole_minute_time.is_some());
}

#[test]
fn last_minute_of_day_succeeds() {
    let time = chrono::NaiveTime::from_hms_opt(23, 59, 0).unwrap();
    let whole_minute_time = WholeMinuteTime::new(time);

    assert!(whole_minute_time.is_some());
}

#[test]
fn try_from_valid_time_succeeds() {
    let time = chrono::NaiveTime::from_hms_opt(9, 15, 0).unwrap();
    let result: Result<WholeMinuteTime, _> = time.try_into();

    assert!(result.is_ok());
}

#[test]
fn try_from_invalid_time_fails() {
    let time = chrono::NaiveTime::from_hms_opt(9, 15, 30).unwrap();
    let result: Result<WholeMinuteTime, _> = time.try_into();

    assert!(result.is_err());
}

#[test]
fn try_from_error_type() {
    let time = chrono::NaiveTime::from_hms_opt(9, 15, 30).unwrap();
    let result: Result<WholeMinuteTime, NotWholeMinuteError> = time.try_into();

    match result {
        Err(NotWholeMinuteError) => {
            // Expected
        }
        Ok(_) => panic!("Expected error, got Ok"),
    }
}

#[test]
fn display_formatting() {
    let time = chrono::NaiveTime::from_hms_opt(9, 5, 0).unwrap();
    let whole_minute_time = WholeMinuteTime::new(time).unwrap();

    assert_eq!(whole_minute_time.to_string(), "09h05");
}

#[test]
fn display_formatting_afternoon() {
    let time = chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap();
    let whole_minute_time = WholeMinuteTime::new(time).unwrap();

    assert_eq!(whole_minute_time.to_string(), "14h30");
}

#[test]
fn deref_to_naive_time() {
    let time = chrono::NaiveTime::from_hms_opt(10, 45, 0).unwrap();
    let whole_minute_time = WholeMinuteTime::new(time).unwrap();

    // Should be able to call NaiveTime methods directly
    assert_eq!(whole_minute_time.hour(), 10);
    assert_eq!(whole_minute_time.minute(), 45);
}

#[test]
fn ordering() {
    let time1 = WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap()).unwrap();
    let time2 = WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap()).unwrap();

    assert!(time1 < time2);
    assert!(time2 > time1);
}

#[test]
fn equality() {
    let time1 = WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(10, 30, 0).unwrap()).unwrap();
    let time2 = WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(10, 30, 0).unwrap()).unwrap();

    assert_eq!(time1, time2);
}
