use std::path::PathBuf;

use collomatique_rooms::ScheduleError;
use collomatique_rooms::parsing;
use collomatique_rooms::{Hour, RoomPreference, Window};
use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run(rooms: &str, requests: &str) -> Result<(), ScheduleError> {
    collomatique_rooms::run(&fixture(rooms), &fixture(requests), None)
}

fn run_with_incompats(rooms: &str, requests: &str, incompats: &str) -> Result<(), ScheduleError> {
    collomatique_rooms::run(
        &fixture(rooms),
        &fixture(requests),
        Some(&fixture(incompats)),
    )
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
fn rooms_bad_priority() {
    let err = run("rooms_bad_priority.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RoomsRowError { row: 1, .. }));
}

#[test]
fn rooms_priority_minus_one() {
    let rooms = parsing::parse_rooms(&fixture("rooms_priority_minus_one.csv")).unwrap();
    assert_eq!(rooms.len(), 1);
    let (_, room) = rooms.iter().next().unwrap();
    assert_eq!(room.priority, None);
}

#[test]
fn rooms_bad_window() {
    let err = run("rooms_bad_window.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RoomsRowError { row: 1, .. }));
}

#[test]
fn rooms_duplicate_name() {
    let err = run("rooms_duplicate_name.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RoomsDuplicateName {
            first_row: 1,
            duplicate_row: 2,
            ..
        }
    ));
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
fn requests_bad_subject_case() {
    let err = run("valid_rooms.csv", "requests_bad_subject_case.csv").unwrap_err();
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
    let rooms = parsing::parse_rooms(&fixture("valid_rooms.csv")).unwrap();
    assert_eq!(rooms.len(), 1);
    let room = &rooms[&nes("A101")];
    assert_eq!(room.floor, 1);
    assert_eq!(room.x, 2.5);
    assert_eq!(room.y, 3.0);
    assert_eq!(room.blackboards, 2);
    assert_eq!(room.whiteboards, 1);
    assert_eq!(room.capacity.get(), 30);
    assert_eq!(room.window, Window::Exterior);
    assert_eq!(room.priority, Some(0));
    assert!(!room.reserved);
}

#[test]
fn rooms_reserved() {
    let rooms = parsing::parse_rooms(&fixture("rooms_reserved.csv")).unwrap();
    assert_eq!(rooms.len(), 1);
    let (_, room) = rooms.iter().next().unwrap();
    assert!(room.reserved);
}

#[test]
fn parse_requests_valid() {
    let requests = parsing::parse_requests(&fixture("valid_requests.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    let r = &requests[0];
    assert!(r.p1);
    assert!(!r.p2);
    assert!(r.p3);
    assert_eq!(r.day, Weekday(chrono::Weekday::Mon));
    assert_eq!(r.hour, Hour::new(8).unwrap());
    assert_eq!(r.subject, nes("Mathématiques"));
    assert_eq!(r.classes, vec![nes("MP"), nes("PC")]);
    assert_eq!(r.requester, nes("Dupont"));
    assert_eq!(r.teacher, nes("Martin"));
    assert_eq!(r.blackboards, 1);
    assert!(!r.window);
    assert_eq!(r.students.get(), 3);
    assert_eq!(r.prep_students, 2);
    assert_eq!(
        r.room_preference,
        Some(RoomPreference::Suggestion(nes("A101")))
    );
    assert!(r.prep_preference.is_none());
}

#[test]
fn parse_requests_room_demand() {
    let requests = parsing::parse_requests(&fixture("requests_room_demand.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].room_preference,
        Some(RoomPreference::Demand(nes("A101")))
    );
}

#[test]
fn parse_schedule_valid() {
    let data = parsing::parse_schedule(
        &fixture("valid_rooms.csv"),
        &fixture("valid_requests.csv"),
        None,
    )
    .unwrap();
    assert_eq!(data.rooms.len(), 1);
    assert_eq!(data.requests.len(), 1);
    assert!(data.unregistered_rooms().is_empty());
}

#[test]
fn unregistered_room_detected() {
    let data = parsing::parse_schedule(
        &fixture("valid_rooms.csv"),
        &fixture("requests_unregistered_room.csv"),
        None,
    )
    .unwrap();
    assert_eq!(data.unregistered_rooms(), vec!["Z999"]);
}

// --- Incompats happy path ---

#[test]
fn parse_incompats_valid() {
    let incompats = parsing::parse_incompats(&fixture("valid_incompats.csv")).unwrap();
    assert_eq!(incompats.len(), 1);
    let i = &incompats[0];
    assert_eq!(i.room, nes("A101"));
    assert!(i.p1);
    assert!(!i.p2);
    assert!(i.p3);
    assert_eq!(i.day, Weekday(chrono::Weekday::Mon));
    assert_eq!(i.hour, Hour::new(8).unwrap());
}

#[test]
fn parse_schedule_with_incompats() {
    let data = parsing::parse_schedule(
        &fixture("valid_rooms.csv"),
        &fixture("valid_requests.csv"),
        Some(&fixture("valid_incompats.csv")),
    )
    .unwrap();
    assert_eq!(data.incompats.len(), 1);
}

// --- Incompats header errors ---

#[test]
fn incompats_missing_column() {
    let err = run_with_incompats(
        "valid_rooms.csv",
        "valid_requests.csv",
        "incompats_missing_column.csv",
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::IncompatsMissingColumn(col) if col == "Jour"
    ));
}

#[test]
fn incompats_unknown_column() {
    let err = run_with_incompats(
        "valid_rooms.csv",
        "valid_requests.csv",
        "incompats_unknown_column.csv",
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::IncompatsUnknownColumn(col) if col == "Extra"
    ));
}

// --- Incompats row errors ---

#[test]
fn incompats_bad_bool() {
    let err = run_with_incompats(
        "valid_rooms.csv",
        "valid_requests.csv",
        "incompats_bad_bool.csv",
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::IncompatsRowError { row: 1, .. }
    ));
}

#[test]
fn incompats_bad_day() {
    let err = run_with_incompats(
        "valid_rooms.csv",
        "valid_requests.csv",
        "incompats_bad_day.csv",
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::IncompatsRowError { row: 1, ref message }
        if message.contains("invalid day")
    ));
}

#[test]
fn incompats_bad_hour() {
    let err = run_with_incompats(
        "valid_rooms.csv",
        "valid_requests.csv",
        "incompats_bad_hour.csv",
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::IncompatsRowError { row: 1, ref message }
        if message.contains("hour must be between 8 and 19")
    ));
}

// --- Incompats cross-file validation ---

#[test]
fn incompats_undeclared_room() {
    let err = run_with_incompats(
        "valid_rooms.csv",
        "valid_requests.csv",
        "incompats_undeclared_room.csv",
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::IncompatsUndeclaredRoom { row: 1, ref room }
        if room == "Z999"
    ));
}
