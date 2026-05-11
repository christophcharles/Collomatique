use std::path::PathBuf;

use collomatique_rooms::schedule::{self, ScheduleError};
use collomatique_time::Weekday;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run(rooms: &str, requests: &str) -> Result<(), ScheduleError> {
    schedule::run(&fixture(rooms), &fixture(requests))
}

#[test]
fn rooms_missing_column() {
    let err = run("rooms_missing_column.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RoomsMissingColumn(col) if col == "Étage"));
}

#[test]
fn rooms_empty_characteristic() {
    let err = run("rooms_empty_characteristic.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RoomsUnknownColumn(_)));
}

#[test]
fn requests_missing_column() {
    let err = run("valid_rooms.csv", "requests_missing_column.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RequestsMissingColumn(col) if col == "Jour"));
}

#[test]
fn requests_unknown_extra() {
    let err = run("valid_rooms.csv", "requests_unknown_extra.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RequestsUnknownColumn(_)));
}

#[test]
fn requests_missing_extra() {
    let err = run("valid_rooms.csv", "requests_missing_extra.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RequestsMissingColumn(col) if col == "Tableaux Min"));
}

#[test]
fn requests_invalid_day() {
    let err = run("valid_rooms.csv", "requests_invalid_day.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::InvalidDay { row: 1, .. }));
}

#[test]
fn requests_min_gt_max() {
    let err = run("valid_rooms.csv", "requests_min_gt_max.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::MinGreaterThanMax {
            row: 1,
            min: 5,
            max: 2,
            ..
        }
    ));
}

#[test]
fn rooms_bad_floor() {
    let err = run("rooms_bad_floor.csv", "valid_requests.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::RoomsRowError { row: 1, .. }));
}

#[test]
fn requests_bad_prep() {
    let err = run("valid_rooms.csv", "requests_bad_prep.csv").unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RequestsRowError { row: 1, .. }
    ));
}

#[test]
fn parse_rooms_valid() {
    let (characteristics, rooms) = schedule::parse_rooms(&fixture("valid_rooms.csv")).unwrap();
    assert_eq!(characteristics, vec!["Tableaux"]);
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0].name, "A101");
    assert_eq!(rooms[0].floor, 1);
    assert_eq!(rooms[0].x, 2.5);
    assert_eq!(rooms[0].y, 3.0);
    assert_eq!(rooms[0].characteristic_values, vec![2]);
}

#[test]
fn parse_requests_valid() {
    let characteristics = vec!["Tableaux".to_string()];
    let requests =
        schedule::parse_requests(&fixture("valid_requests.csv"), &characteristics).unwrap();
    assert_eq!(requests.len(), 1);
    let r = &requests[0];
    assert_eq!(r.period, 1);
    assert_eq!(r.day, Weekday(chrono::Weekday::Mon));
    assert_eq!(r.hour, 8);
    assert_eq!(r.subject, "Maths");
    assert_eq!(r.responsible, "Dupont");
    assert_eq!(r.colleur, "Martin");
    assert_eq!(r.floor, 1);
    assert_eq!(r.x, 2.5);
    assert_eq!(r.y, 3.0);
    assert!(!r.prep);
    assert_eq!(r.constraints.len(), 1);
    assert_eq!(r.constraints[0].min, Some(1));
    assert_eq!(r.constraints[0].max, Some(3));
}

#[test]
fn parse_schedule_valid() {
    let data =
        schedule::parse_schedule(&fixture("valid_rooms.csv"), &fixture("valid_requests.csv"))
            .unwrap();
    assert_eq!(data.characteristics, vec!["Tableaux"]);
    assert_eq!(data.rooms.len(), 1);
    assert_eq!(data.requests.len(), 1);
}
