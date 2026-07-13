use collomatique_storage::*;
use std::collections::BTreeSet;

#[test]
fn decode_empty_file_with_correct_header() {
    let content = r#"{
    "header": {
        "file_type": "Collomatique",
        "produced_with_version": {
            "major": 0,
            "minor": 1,
            "patch": 0
        },
        "file_content": "Colloscope"
    },
    "entries": []
}"#;

    let (data, caveats) = deserialize_data(content).expect("Empty file should be valid");

    let expected_data = collomatique_state_colloscopes::Data::new();
    let expected_caveats = BTreeSet::new();

    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}

#[test]
fn decode_legacy_empty_fixture() {
    // A committed spec-1 (legacy) document must still decode to empty
    // data, with the deprecated-format caveat, until the legacy reader
    // is retired. The fixture was produced by the (now removed) legacy
    // writer.
    let content = include_str!("fixtures/spec1_empty.json");
    let (decoded_data, caveats) =
        deserialize_data(content).expect("Legacy fixture should still decode");

    let expected_data = collomatique_state_colloscopes::Data::new();
    let expected_caveats = BTreeSet::from([Caveat::DeprecatedFormat]);
    assert_eq!(decoded_data, expected_data);
    assert_eq!(caveats, expected_caveats);
}
