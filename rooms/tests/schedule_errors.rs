use std::path::PathBuf;

use std::collections::BTreeMap;
use std::num::NonZeroU32;

use collomatique_ilp::solvers::collo_cbc::ColloCbcSolver;
use collomatique_rooms::ScheduleError;
use collomatique_rooms::parsing;
use collomatique_rooms::{
    Config, DemandConflictKind, DemandKind, Hour, InterrogationRoomPreference, Periods,
    PrepRoomPreference, Request, Room, RoomPreferenceWarning, ScheduleData, Window,
};
use collomatique_time::Weekday;
use non_empty_string::NonEmptyString;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn run(rooms: &str, requests: &str) -> Result<(), ScheduleError> {
    collomatique_rooms::run(
        &fixture(rooms),
        &fixture(requests),
        None,
        false,
        Default::default(),
        0,
    )
}

fn run_with_incompats(rooms: &str, requests: &str, incompats: &str) -> Result<(), ScheduleError> {
    collomatique_rooms::run(
        &fixture(rooms),
        &fixture(requests),
        Some(&fixture(incompats)),
        false,
        Default::default(),
        0,
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
    let (requests, _) = parsing::parse_requests(&fixture("valid_requests.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    let r = &requests[0];
    assert!(r.periods.p1);
    assert!(!r.periods.p2);
    assert!(r.periods.p3);
    assert_eq!(r.day, Weekday(chrono::Weekday::Mon));
    assert_eq!(r.hour, Hour::new(8).unwrap());
    assert_eq!(r.subjects, vec![nes("Mathématiques")]);
    assert_eq!(r.classes, vec![nes("MP"), nes("PC")]);
    assert_eq!(r.requester, nes("Dupont"));
    assert_eq!(r.teacher, nes("Martin"));
    assert_eq!(r.blackboards, 1);
    assert!(!r.window);
    assert_eq!(r.students.get(), 3);
    assert_eq!(r.prep_students, 2);
    assert_eq!(
        r.room_preference,
        vec![InterrogationRoomPreference::Suggestion {
            room: nes("A101"),
            can_share_with_prep: false,
        }]
    );
    assert!(r.prep_preference.is_empty());
}

#[test]
fn parse_requests_room_demand() {
    let (requests, _) = parsing::parse_requests(&fixture("requests_room_demand.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].room_preference,
        vec![InterrogationRoomPreference::Demand {
            room: nes("A101"),
            can_share_with_prep: false,
        }]
    );
}

#[test]
fn parse_schedule_valid() {
    let (data, _) = parsing::parse_schedule(
        &fixture("valid_rooms.csv"),
        &fixture("valid_requests.csv"),
        None,
        Default::default(),
    )
    .unwrap();
    assert_eq!(data.rooms.len(), 1);
    assert_eq!(data.requests.len(), 1);
    let unreg = data.unregistered_rooms();
    assert!(unreg.suggested.is_empty());
    assert!(unreg.demanded.is_empty());
}

#[test]
fn unregistered_suggested_room_detected() {
    let (data, _) = parsing::parse_schedule(
        &fixture("valid_rooms.csv"),
        &fixture("requests_unregistered_room.csv"),
        None,
        Default::default(),
    )
    .unwrap();
    let unreg = data.unregistered_rooms();
    assert_eq!(unreg.suggested, vec!["Z999"]);
    assert!(unreg.demanded.is_empty());
}

#[test]
fn unregistered_demanded_room_detected() {
    let (data, _) = parsing::parse_schedule(
        &fixture("valid_rooms.csv"),
        &fixture("requests_unregistered_demanded_room.csv"),
        None,
        Default::default(),
    )
    .unwrap();
    let unreg = data.unregistered_rooms();
    assert!(unreg.suggested.is_empty());
    assert_eq!(unreg.demanded, vec!["Z999"]);
}

// --- Incompats happy path ---

#[test]
fn parse_incompats_valid() {
    let incompats = parsing::parse_incompats(&fixture("valid_incompats.csv")).unwrap();
    assert_eq!(incompats.len(), 1);
    let i = &incompats[0];
    assert_eq!(i.room, nes("A101"));
    assert!(i.periods.p1);
    assert!(!i.periods.p2);
    assert!(i.periods.p3);
    assert_eq!(i.day, Weekday(chrono::Weekday::Mon));
    assert_eq!(i.hour, Hour::new(8).unwrap());
}

#[test]
fn parse_schedule_with_incompats() {
    let (data, _) = parsing::parse_schedule(
        &fixture("valid_rooms.csv"),
        &fixture("valid_requests.csv"),
        Some(&fixture("valid_incompats.csv")),
        Default::default(),
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

// --- Demand conflict detection ---

fn make_room(capacity: u32) -> Room {
    Room {
        floor: 0,
        x: 0.0,
        y: 0.0,
        blackboards: 0,
        whiteboards: 0,
        capacity: NonZeroU32::new(capacity).unwrap(),
        window: Window::None,
        priority: Some(0),
        reserved: false,
    }
}

fn make_request(
    day: chrono::Weekday,
    hour: u32,
    periods: (bool, bool, bool),
    room_preference: Vec<InterrogationRoomPreference>,
    prep_preference: Vec<PrepRoomPreference>,
    prep_students: u32,
) -> Request {
    Request {
        periods: Periods {
            p1: periods.0,
            p2: periods.1,
            p3: periods.2,
        },
        day: Weekday(day),
        hour: Hour::new(hour).unwrap(),
        subjects: vec![nes("Mathématiques")],
        classes: vec![nes("MP")],
        requester: nes("Dupont"),
        teacher: nes("Martin"),
        blackboards: 0,
        window: false,
        students: NonZeroU32::new(3).unwrap(),
        prep_students,
        room_preference,
        floor_suggestions: vec![],
        prep_preference,
        skip_room_continuity: false,
    }
}

#[test]
fn demand_no_conflict_non_overlapping_periods() {
    let mut rooms = BTreeMap::new();
    rooms.insert(nes("A101"), make_room(30));
    let data = ScheduleData {
        rooms,
        requests: vec![
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![InterrogationRoomPreference::Demand {
                    room: nes("A101"),
                    can_share_with_prep: false,
                }],
                vec![],
                0,
            ),
            make_request(
                chrono::Weekday::Mon,
                8,
                (false, true, false),
                vec![InterrogationRoomPreference::Demand {
                    room: nes("A101"),
                    can_share_with_prep: false,
                }],
                vec![],
                0,
            ),
        ],
        incompats: vec![],
        config: Config::default(),
    };
    assert!(data.demand_conflicts().is_empty());
}

#[test]
fn demand_interro_interro_conflict() {
    let mut rooms = BTreeMap::new();
    rooms.insert(nes("A101"), make_room(30));
    let data = ScheduleData {
        rooms,
        requests: vec![
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![InterrogationRoomPreference::Demand {
                    room: nes("A101"),
                    can_share_with_prep: false,
                }],
                vec![],
                0,
            ),
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, true, false),
                vec![InterrogationRoomPreference::Demand {
                    room: nes("A101"),
                    can_share_with_prep: false,
                }],
                vec![],
                0,
            ),
        ],
        incompats: vec![],
        config: Config::default(),
    };
    let conflicts = data.demand_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].kind,
        DemandConflictKind::InterrogationInterrogation
    );
    assert_eq!(conflicts[0].requests.len(), 2);
    assert_eq!(conflicts[0].requests[0], (0, DemandKind::Interrogation));
    assert_eq!(conflicts[0].requests[1], (1, DemandKind::Interrogation));
}

#[test]
fn demand_interro_prep_conflict() {
    let mut rooms = BTreeMap::new();
    rooms.insert(nes("A101"), make_room(30));
    let data = ScheduleData {
        rooms,
        requests: vec![
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![InterrogationRoomPreference::Demand {
                    room: nes("A101"),
                    can_share_with_prep: false,
                }],
                vec![],
                0,
            ),
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, true, false),
                vec![],
                vec![PrepRoomPreference::Demand(nes("A101"))],
                5,
            ),
        ],
        incompats: vec![],
        config: Config::default(),
    };
    let conflicts = data.demand_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].kind,
        DemandConflictKind::InterrogationPrep {
            can_share_with_prep: false,
        }
    );
    assert_eq!(conflicts[0].requests[0], (0, DemandKind::Interrogation));
    assert_eq!(conflicts[0].requests[1], (1, DemandKind::Prep));
}

#[test]
fn demand_prep_prep_over_capacity() {
    let mut rooms = BTreeMap::new();
    rooms.insert(nes("A101"), make_room(10));
    let data = ScheduleData {
        rooms,
        requests: vec![
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![],
                vec![PrepRoomPreference::Demand(nes("A101"))],
                6,
            ),
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![],
                vec![PrepRoomPreference::Demand(nes("A101"))],
                7,
            ),
        ],
        incompats: vec![],
        config: Config::default(),
    };
    let conflicts = data.demand_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].kind,
        DemandConflictKind::PrepOverCapacity {
            total_students: 13,
            capacity: NonZeroU32::new(10).unwrap(),
        }
    );
}

#[test]
fn demand_prep_prep_fits() {
    let mut rooms = BTreeMap::new();
    rooms.insert(nes("A101"), make_room(20));
    let data = ScheduleData {
        rooms,
        requests: vec![
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![],
                vec![PrepRoomPreference::Demand(nes("A101"))],
                6,
            ),
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![],
                vec![PrepRoomPreference::Demand(nes("A101"))],
                7,
            ),
        ],
        incompats: vec![],
        config: Config::default(),
    };
    assert!(data.demand_conflicts().is_empty());
}

#[test]
fn demand_prep_prep_unlisted_room() {
    let data = ScheduleData {
        rooms: BTreeMap::new(),
        requests: vec![
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![],
                vec![PrepRoomPreference::Demand(nes("Z999"))],
                3,
            ),
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![],
                vec![PrepRoomPreference::Demand(nes("Z999"))],
                4,
            ),
        ],
        incompats: vec![],
        config: Config::default(),
    };
    let conflicts = data.demand_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].kind,
        DemandConflictKind::PrepUnknownCapacity { total_students: 7 }
    );
}

#[test]
fn demand_suggestions_ignored() {
    let mut rooms = BTreeMap::new();
    rooms.insert(nes("A101"), make_room(30));
    let data = ScheduleData {
        rooms,
        requests: vec![
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![InterrogationRoomPreference::Suggestion {
                    room: nes("A101"),
                    can_share_with_prep: false,
                }],
                vec![],
                0,
            ),
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![InterrogationRoomPreference::Suggestion {
                    room: nes("A101"),
                    can_share_with_prep: false,
                }],
                vec![],
                0,
            ),
        ],
        incompats: vec![],
        config: Config::default(),
    };
    assert!(data.demand_conflicts().is_empty());
}

fn assert_checker_feasible(rooms: &str, requests: &str) {
    let (data, _) = parsing::parse_schedule(
        &fixture(rooms),
        &fixture(requests),
        None,
        Default::default(),
    )
    .unwrap();
    let model = collomatique_constraints_rooms::build_model(&data);
    let solver = ColloCbcSolver::with_disable_logging(true);
    assert!(
        model.solve_checker(&solver).is_some(),
        "expected feasible checker problem for {rooms} + {requests}"
    );
}

#[test]
fn priority_base_feasible() {
    assert_checker_feasible("priority_base_rooms.csv", "priority_base_requests.csv");
}

#[test]
fn priority_demand_feasible() {
    assert_checker_feasible("priority_demand_rooms.csv", "priority_demand_requests.csv");
}

#[test]
fn priority_overflow_feasible() {
    assert_checker_feasible(
        "priority_overflow_rooms.csv",
        "priority_overflow_requests.csv",
    );
}

#[test]
fn priority_many_rooms_feasible() {
    assert_checker_feasible("priority_many_rooms.csv", "priority_many_requests.csv");
}

#[test]
fn priority_prep_feasible() {
    assert_checker_feasible("priority_prep_rooms.csv", "priority_prep_requests.csv");
}

#[test]
fn priority_prep_full_feasible() {
    assert_checker_feasible("priority_prep_rooms.csv", "priority_prep_full_requests.csv");
}

#[test]
fn priority_reserved_feasible() {
    assert_checker_feasible(
        "priority_reserved_rooms.csv",
        "priority_prep_full_requests.csv",
    );
}

#[test]
fn priority_global_feasible() {
    assert_checker_feasible("priority_global_rooms.csv", "priority_global_requests.csv");
}

// --- Prep sharing (+ suffix) ---

#[test]
fn parse_requests_suggestion_sharing() {
    let (requests, _) =
        parsing::parse_requests(&fixture("requests_room_suggestion_sharing.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].room_preference,
        vec![InterrogationRoomPreference::Suggestion {
            room: nes("A101"),
            can_share_with_prep: true,
        }]
    );
}

#[test]
fn parse_requests_demand_sharing() {
    let (requests, _) =
        parsing::parse_requests(&fixture("requests_room_demand_sharing.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].room_preference,
        vec![InterrogationRoomPreference::Demand {
            room: nes("A101"),
            can_share_with_prep: true,
        }]
    );
}

#[test]
fn sharing_feasible() {
    assert_checker_feasible("sharing_rooms.csv", "sharing_requests.csv");
}

#[test]
fn demand_interro_prep_conflict_with_sharing() {
    let mut rooms = BTreeMap::new();
    rooms.insert(nes("A101"), make_room(30));
    let data = ScheduleData {
        rooms,
        requests: vec![
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, false, false),
                vec![InterrogationRoomPreference::Demand {
                    room: nes("A101"),
                    can_share_with_prep: true,
                }],
                vec![],
                0,
            ),
            make_request(
                chrono::Weekday::Mon,
                8,
                (true, true, false),
                vec![],
                vec![PrepRoomPreference::Demand(nes("A101"))],
                5,
            ),
        ],
        incompats: vec![],
        config: Config::default(),
    };
    let conflicts = data.demand_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].kind,
        DemandConflictKind::InterrogationPrep {
            can_share_with_prep: true,
        }
    );
}

// --- Multiple room preferences ---

#[test]
fn parse_requests_multi_room() {
    let (requests, warnings) =
        parsing::parse_requests(&fixture("requests_multi_room.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert!(warnings.is_empty());
    let r = &requests[0];
    assert_eq!(
        r.room_preference,
        vec![
            InterrogationRoomPreference::Suggestion {
                room: nes("A101"),
                can_share_with_prep: true,
            },
            InterrogationRoomPreference::Demand {
                room: nes("B302"),
                can_share_with_prep: false,
            },
        ]
    );
    assert_eq!(
        r.prep_preference,
        vec![
            PrepRoomPreference::Demand(nes("C205")),
            PrepRoomPreference::Suggestion(nes("D410")),
        ]
    );
}

#[test]
fn parse_requests_redundant_merge() {
    let (requests, warnings) =
        parsing::parse_requests(&fixture("requests_redundant_room.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    let r = &requests[0];
    assert_eq!(
        r.room_preference,
        vec![InterrogationRoomPreference::Demand {
            room: nes("A101"),
            can_share_with_prep: true,
        }]
    );
    assert_eq!(
        r.prep_preference,
        vec![PrepRoomPreference::Demand(nes("B302"))]
    );
    assert_eq!(warnings.len(), 2);
    assert!(matches!(
        &warnings[0],
        RoomPreferenceWarning::Redundancy {
            column: "Salle",
            room,
            merged_result,
            ..
        } if room == "A101" && merged_result == "!A101+"
    ));
    assert!(matches!(
        &warnings[1],
        RoomPreferenceWarning::Redundancy {
            column: "Prep",
            room,
            merged_result,
            ..
        } if room == "B302" && merged_result == "!B302"
    ));
}

#[test]
fn parse_requests_interro_prep_no_sharing_warning() {
    let (requests, warnings) =
        parsing::parse_requests(&fixture("requests_interro_prep_no_sharing.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        &warnings[0],
        RoomPreferenceWarning::InterrogationAndPrepWithoutSharing { row: 1, room }
        if room == "A101"
    ));
}

#[test]
fn parse_requests_interro_prep_with_sharing_no_warning() {
    let (requests, warnings) =
        parsing::parse_requests(&fixture("requests_interro_prep_with_sharing.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert!(warnings.is_empty());
}

// --- Avoidance and exclusion ---

#[test]
fn parse_requests_avoidance() {
    let (requests, warnings) = parsing::parse_requests(&fixture("requests_avoidance.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert!(warnings.is_empty());
    assert_eq!(
        requests[0].room_preference,
        vec![InterrogationRoomPreference::Avoidance { room: nes("A101") }]
    );
}

#[test]
fn parse_requests_exclusion() {
    let (requests, warnings) = parsing::parse_requests(&fixture("requests_exclusion.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert!(warnings.is_empty());
    assert_eq!(
        requests[0].room_preference,
        vec![InterrogationRoomPreference::Exclusion { room: nes("A101") }]
    );
}

#[test]
fn parse_requests_conflicting_prefs() {
    let (requests, warnings) =
        parsing::parse_requests(&fixture("requests_conflicting_prefs.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].room_preference.is_empty());
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        &warnings[0],
        RoomPreferenceWarning::ConflictingPreferences {
            row: 1,
            room,
            ..
        } if room == "A101"
    ));
}

#[test]
fn parse_requests_negative_merge() {
    let (requests, warnings) =
        parsing::parse_requests(&fixture("requests_negative_merge.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].room_preference,
        vec![InterrogationRoomPreference::Exclusion { room: nes("A101") }]
    );
    assert_eq!(warnings.len(), 1);
    assert!(matches!(
        &warnings[0],
        RoomPreferenceWarning::Redundancy {
            column: "Salle",
            room,
            merged_result,
            ..
        } if room == "A101" && merged_result == "~A101"
    ));
}

#[test]
fn conflicting_prefs_is_fatal_error() {
    let err = run("valid_rooms.csv", "requests_conflicting_prefs.csv").unwrap_err();
    assert!(matches!(err, ScheduleError::ConflictingRoomPreferences(_)));
}

#[test]
fn exclusion_constraint_feasible() {
    assert_checker_feasible("exclusion_rooms.csv", "exclusion_requests.csv");
}

#[test]
fn exclusion_constraint_assigns_other_room() {
    let (data, _) = parsing::parse_schedule(
        &fixture("exclusion_rooms.csv"),
        &fixture("exclusion_requests.csv"),
        None,
        Default::default(),
    )
    .unwrap();
    let model = collomatique_constraints_rooms::build_model(&data);
    let solver = ColloCbcSolver::with_disable_logging(true);
    let solved = model.solve_checker(&solver).unwrap();
    let config = solved.get_data();
    let assignments = collomatique_constraints_rooms::extract_assignments(&data, &config);
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].room, nes("ROOM_B"));
}

#[test]
fn parse_requests_multi_subject() {
    let (requests, _) = parsing::parse_requests(&fixture("requests_multi_subject.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    let r = &requests[0];
    assert_eq!(r.subjects, vec![nes("Physique"), nes("Chimie")]);
}

#[test]
fn parse_requests_floor_suggestion_with_room() {
    let (requests, _) = parsing::parse_requests(&fixture("requests_floor_suggestion.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    let r = &requests[0];
    assert_eq!(
        r.room_preference,
        vec![InterrogationRoomPreference::Suggestion {
            room: nes("A101"),
            can_share_with_prep: false,
        }]
    );
    assert_eq!(r.floor_suggestions, vec![2]);
}

#[test]
fn parse_requests_floor_only() {
    let (requests, _) = parsing::parse_requests(&fixture("requests_floor_only.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    let r = &requests[0];
    assert!(r.room_preference.is_empty());
    assert_eq!(r.floor_suggestions, vec![2]);
}

#[test]
fn parse_requests_floor_multiple() {
    let (requests, _) = parsing::parse_requests(&fixture("requests_floor_multiple.csv")).unwrap();
    assert_eq!(requests.len(), 1);
    let r = &requests[0];
    assert_eq!(r.floor_suggestions, vec![2, 3]);
}

#[test]
fn parse_requests_floor_bad() {
    let err = parsing::parse_requests(&fixture("requests_floor_bad.csv")).unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RequestsRowError { row: 1, ref message }
        if message.contains("invalid floor suggestion")
    ));
}

#[test]
fn parse_requests_floor_empty() {
    let err = parsing::parse_requests(&fixture("requests_floor_empty.csv")).unwrap_err();
    assert!(matches!(
        err,
        ScheduleError::RequestsRowError { row: 1, ref message }
        if message.contains("invalid floor suggestion")
    ));
}

// --- Room continuity ---

#[test]
fn continuity_same_room() {
    let (data, _) = parsing::parse_schedule(
        &fixture("continuity_rooms.csv"),
        &fixture("continuity_requests.csv"),
        None,
        Default::default(),
    )
    .unwrap();
    let model = collomatique_constraints_rooms::build_model(&data);
    let solver = ColloCbcSolver::with_disable_logging(true);
    let solved = model.solve_checker(&solver).unwrap();
    let config = solved.get_data();
    let assignments = collomatique_constraints_rooms::extract_assignments(&data, &config);
    assert_eq!(assignments.len(), 2);
    assert_eq!(assignments[0].room, assignments[1].room);
}

#[test]
fn continuity_isolated_feasible() {
    assert_checker_feasible("continuity_rooms.csv", "continuity_isolated_requests.csv");
}

#[test]
fn teacher_conflict_detected() {
    let mut rooms = BTreeMap::new();
    rooms.insert(nes("A101"), make_room(30));
    let data = ScheduleData {
        rooms,
        requests: vec![
            make_request(
                chrono::Weekday::Mon,
                10,
                (true, false, true),
                vec![],
                vec![],
                0,
            ),
            make_request(
                chrono::Weekday::Mon,
                10,
                (false, false, true),
                vec![],
                vec![],
                0,
            ),
        ],
        incompats: vec![],
        config: Config::default(),
    };
    let conflicts = data.teacher_continuity_conflicts();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].teacher, nes("Martin"));
    assert_eq!(conflicts[0].hour, Hour::new(10).unwrap());
    assert_eq!(conflicts[0].requests, vec![0, 1]);
}

#[test]
fn teacher_conflict_no_overlap() {
    let mut rooms = BTreeMap::new();
    rooms.insert(nes("A101"), make_room(30));
    let data = ScheduleData {
        rooms,
        requests: vec![
            make_request(
                chrono::Weekday::Mon,
                10,
                (true, false, false),
                vec![],
                vec![],
                0,
            ),
            make_request(
                chrono::Weekday::Mon,
                10,
                (false, false, true),
                vec![],
                vec![],
                0,
            ),
        ],
        incompats: vec![],
        config: Config::default(),
    };
    let conflicts = data.teacher_continuity_conflicts();
    assert!(conflicts.is_empty());
}

#[test]
fn teacher_conflict_skipped_when_isolated() {
    let mut rooms = BTreeMap::new();
    rooms.insert(nes("A101"), make_room(30));
    let mut req1 = make_request(
        chrono::Weekday::Mon,
        10,
        (true, false, true),
        vec![],
        vec![],
        0,
    );
    let req2 = make_request(
        chrono::Weekday::Mon,
        10,
        (false, false, true),
        vec![],
        vec![],
        0,
    );
    req1.skip_room_continuity = true;
    let data = ScheduleData {
        rooms,
        requests: vec![req1, req2],
        incompats: vec![],
        config: Config::default(),
    };
    let conflicts = data.teacher_continuity_conflicts();
    assert!(conflicts.is_empty());
}
