//! Round-trip tests on a fully populated document
//!
//! The pre-existing storage tests only cover empty data; these ones
//! exercise every serialized section (see `builder::build_rich_data`)
//! and pin the format determinism that the phase-1 golden fixtures
//! will rely on.

mod populated_round_trip {
    pub mod builder;
}

use populated_round_trip::builder;

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{NewId, Op, StudentOp, students::Student};
use collomatique_storage::{Caveat, deserialize_data, serialize_data};
use std::collections::BTreeSet;

#[test]
fn round_trip_identity() {
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data, true);
    let (decoded, caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    assert_eq!(decoded, data);
    // A legacy document decodes with the deprecated-format caveat.
    assert_eq!(caveats, BTreeSet::from([Caveat::DeprecatedFormat]));
}

#[test]
fn reserialize_is_stable() {
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data, true);
    let (decoded, _caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    assert_eq!(serialize_data(&decoded, true), serialized);
}

#[test]
fn deserialized_data_is_still_editable() {
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data, true);
    let (decoded, _caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    // The rebuilt IdIssuer must issue fresh ids that do not collide
    // with the ids already present in the loaded document
    let mut state = AppState::<_, String>::new(decoded);
    let result = state.apply(
        Op::Student(StudentOp::Add(Student::default())),
        "Add a student after reload".to_string(),
    );
    assert!(matches!(result, Ok(Some(NewId::StudentId(_)))));
}

#[test]
fn round_trip_identity_spec2() {
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data, false);
    let (decoded, caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    assert_eq!(decoded, data);
    assert!(caveats.is_empty());
}

#[test]
fn reserialize_is_stable_spec2() {
    // Pins the canonical-form guarantee: one state, one byte sequence
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data, false);
    let (decoded, _caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    assert_eq!(serialize_data(&decoded, false), serialized);
}

#[test]
fn cross_format_round_trip() {
    // Legacy write -> read -> spec-2 write -> read must preserve the
    // state exactly: this is the bulk-conversion path for existing files
    let data = builder::build_rich_data();

    let legacy = serialize_data(&data, true);
    let (from_legacy, _caveats) =
        deserialize_data(&legacy).expect("Legacy document should deserialize");

    let spec2 = serialize_data(&from_legacy, false);
    let (from_spec2, caveats) =
        deserialize_data(&spec2).expect("Spec-2 document should deserialize");

    assert_eq!(from_spec2, data);
    assert!(caveats.is_empty());
}

#[test]
fn deserialized_spec2_data_is_still_editable() {
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data, false);
    let (decoded, _caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    let mut state = AppState::<_, String>::new(decoded);
    let result = state.apply(
        Op::Student(StudentOp::Add(Student::default())),
        "Add a student after reload".to_string(),
    );
    assert!(matches!(result, Ok(Some(NewId::StudentId(_)))));
}
