use std::path::PathBuf;

use collomatique_rooms::ScheduleError;
use collomatique_rooms::data_model;
use collomatique_rooms::types::Hour;
use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run(rooms: &str, requests: &str) -> Result<(), ScheduleError> {
    collomatique_rooms::run(&fixture(rooms), &fixture(requests))
}

fn nes(s: &str) -> NonEmptyString {
    NonEmptyString::try_from(s).unwrap()
}

// --- Rooms header errors ---

#[test]
fn rooms_missing_column() {
    let err = run("rooms_missing_column.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RoomsMissingColumn(col) if col == "Étage"
    ));
}

#[test]
fn rooms_unknown_column() {
    let err = run("rooms_unknown_column.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RoomsUnknownColumn(col) if col == "Extra"
    ));
}

// --- Rooms row errors ---

#[test]
fn rooms_bad_floor() {
    let err = run("rooms_bad_floor.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RoomsRowError { row: 1, .. }));
}

#[test]
fn rooms_empty_name() {
    let err = run("rooms_empty_name.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RoomsRowError { row: 1, .. }));
}

#[test]
fn rooms_zero_capacity() {
    let err = run("rooms_zero_capacity.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RoomsRowError { row: 1, .. }));
}

#[test]
fn rooms_bad_bool() {
    let err = run("rooms_bad_bool.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RoomsRowError { row: 1, .. }));
}

// --- Requests header errors ---

#[test]
fn requests_missing_column() {
    let err = run("valid_rooms.csv", "requests_missing_column.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RequestsMissingColumn(col) if col == "Jour"
    ));
}

#[test]
fn requests_unknown_column() {
    let err = run("valid_rooms.csv", "requests_unknown_column.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RequestsUnknownColumn(col) if col == "Extra"
    ));
}

// --- Requests row errors ---

#[test]
fn requests_invalid_day() {
    let err = run("valid_rooms.csv", "requests_invalid_day.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::InvalidDay { row: 1, .. }));
}

#[test]
fn requests_invalid_hour() {
    let err = run("valid_rooms.csv", "requests_invalid_hour.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::InvalidHour { row: 1, value: 20 }
    ));
}

#[test]
fn requests_bad_bool() {
    let err = run("valid_rooms.csv", "requests_bad_bool.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RequestsRowError { row: 1, .. }
    ));
}

#[test]
fn requests_bad_subject() {
    let err = run("valid_rooms.csv", "requests_bad_subject.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RequestsRowError { row: 1, ref message }
        if message.contains("unknown subject")
    ));
}

#[test]
fn requests_bad_class() {
    let err = run("valid_rooms.csv", "requests_bad_class.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RequestsRowError { row: 1, ref message }
        if message.contains("unknown class")
    ));
}

#[test]
fn requests_empty_classes() {
    let err = run("valid_rooms.csv", "requests_empty_classes.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RequestsRowError { row: 1, ref message }
        if message.contains("at least one class")
    ));
}

// --- Happy-path parsing ---

#[test]
fn parse_rooms_valid() {
    let rooms = data_model::parse_rooms(&fixture("valid_rooms.csv")).unwrap();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0].name, nes("A101"));
    assert_eq!(rooms[0].floor, 1);
    assert_eq!(rooms[0].x, 2.5);
    assert_eq!(rooms[0].y, 3.0);
    assert_eq!(rooms[0].blackboards, 2);
    assert_eq!(rooms[0].capacity.get(), 30);
    assert!(rooms[0].window);
}

#[test]
fn parse_requests_valid() {
    let requests = data_model::parse_requests(&fixture("valid_requests.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    let r = &requests[0];
    assert!(r.p1);
    assert!(!r.p2);
    assert!(r.p3);
    assert_eq!(r.day, Weekday(chrono::Weekday::Mon));
    assert_eq!(r.hour, Hour::new(8).unwrap());
    assert_eq!(r.subject, nes("mathématiques"));
    assert_eq!(r.classes, vec![nes("MP"), nes("PC")]);
    assert_eq!(r.requester, nes("Dupont"));
    assert_eq!(r.teacher, nes("Martin"));
    assert_eq!(r.blackboards, 1);
    assert!(!r.window);
    assert_eq!(r.students.get(), 3);
    assert_eq!(r.prep_students, 2);
    assert_eq!(r.room_suggestion, Some(nes("A101")));
    assert!(r.prep_suggestion.is_none());
}

#[test]
fn parse_schedule_valid() {
    let data =
        data_model::parse_schedule(&fixture("valid_rooms.csv"), &fixture("valid_requests.csv"))
            .unwrap();
    assert_eq!(data.rooms.len(), 1);
    assert_eq!(data.requests.len(), 1);
}
