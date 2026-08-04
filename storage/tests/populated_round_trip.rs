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

/// Runs the in-memory invariant gate on a decoded document
///
/// The decoder returns a raw [InnerData](collomatique_state_colloscopes::InnerData)
/// and diagnoses every constraint of the file format itself, so this is
/// expected to always succeed — holding it to that is exactly why the
/// tests below call it.
fn gate(inner: collomatique_state_colloscopes::InnerData) -> collomatique_state_colloscopes::Data {
    collomatique_state_colloscopes::Data::from_inner_data(inner)
        .expect("decoded documents must pass the invariant gate")
}

#[test]
fn round_trip_identity() {
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data).expect("Data should be writable");
    let (decoded, caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    // Week ids are synthesized on decode — the file never stores them — so the
    // decoded state can differ from the original only in those internal id
    // values (nothing references them yet). The meaningful round-trip identity
    // is therefore byte-level: re-encoding the decoded state reproduces the
    // original document exactly.
    assert_eq!(
        serialize_data(&gate(decoded)).expect("Decoded data should be writable"),
        serialized
    );
    assert!(caveats.is_empty());
}

#[test]
fn reserialize_is_stable() {
    // Pins the canonical-form guarantee: one state, one byte sequence
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data).expect("Data should be writable");
    let (decoded, _caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    assert_eq!(
        serialize_data(&gate(decoded)).expect("Decoded data should be writable"),
        serialized
    );
}

/// The bytes `serialize_data` produced for [builder::build_rich_data] on
/// the day the fixture was written
///
/// The two tests above pin *self*-consistency: one build's writer against
/// the same build's writer. They stay green if the writer changes its
/// indentation, its key order or its number formatting, as long as it does
/// so consistently. This fixture is the outside witness that catches that.
const GOLDEN: &str = include_str!("fixtures/spec2_populated_golden.json");

/// Blanks out the header's `produced_with_version` object
///
/// That object is the package version, not a format decision, so a version
/// bump must not force a fixture regeneration. Everything else — the
/// header's own shape and key order included — stays under the byte
/// comparison. The version object has no nested braces, so the first `}`
/// closes it.
fn mask_version(document: &str) -> String {
    let start = document
        .find("\"produced_with_version\": {")
        .expect("the header carries a produced_with_version object");
    let end = start
        + document[start..]
            .find('}')
            .expect("the version object is closed")
        + 1;
    format!(
        "{}\"produced_with_version\": <masked>{}",
        &document[..start],
        &document[end..]
    )
}

#[test]
fn writer_output_matches_the_golden_fixture() {
    // On a *deliberate* format evolution, regenerate the fixture with
    //     cargo test -p collomatique-storage --test populated_round_trip \
    //         -- --ignored regenerate_golden_fixture
    // and read the resulting diff before committing it. A diff nobody
    // intended is this test doing its job.
    let serialized = serialize_data(&builder::build_rich_data()).expect("Data should be writable");

    assert_eq!(mask_version(&serialized), mask_version(GOLDEN));
}

#[test]
#[ignore = "writes the golden fixture; run explicitly on a deliberate format change"]
fn regenerate_golden_fixture() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/spec2_populated_golden.json");
    let serialized = serialize_data(&builder::build_rich_data()).expect("Data should be writable");
    std::fs::write(&path, serialized).expect("The fixture should be writable");
}

#[test]
fn deserialized_data_is_still_editable() {
    let data = builder::build_rich_data();

    let serialized = serialize_data(&data).expect("Data should be writable");
    let (decoded, _caveats) =
        deserialize_data(&serialized).expect("Serialized data should deserialize");

    // The rebuilt IdIssuer must issue fresh ids that do not collide
    // with the ids already present in the loaded document
    let mut state = AppState::<_, String>::new(gate(decoded));
    let result = state.apply(
        Op::Student(StudentOp::Add(Student::default())),
        "Add a student after reload".to_string(),
    );
    assert!(matches!(result, Ok(Some(NewId::StudentId(_)))));
}
