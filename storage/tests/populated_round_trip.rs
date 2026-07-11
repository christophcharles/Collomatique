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
use collomatique_storage::{deserialize_data, serialize_data};

#[test]
fn round_trip_identity() {
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data);
    let (decoded, caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    assert_eq!(decoded, data);
    assert!(caveats.is_empty());
}

#[test]
fn reserialize_is_stable() {
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data);
    let (decoded, _caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    assert_eq!(serialize_data(&decoded), serialized);
}

#[test]
fn deserialized_data_is_still_editable() {
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data);
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
