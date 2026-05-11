use std::path::PathBuf;

use collomatique_rooms::schedule::{self, ScheduleError};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run(rooms: &str, requests: &str) -> Result<(), ScheduleError> {
    schedule::run(&[fixture(rooms), fixture(requests)])
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
