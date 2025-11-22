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
fn encode_and_decode_empty_data() {
    let data = collomatique_state_colloscopes::Data::new();

    let content = serialize_data(&data);
    let (decoded_data, caveats) =
        deserialize_data(&content).expect("Produced file should be valid");

    let expected_caveats = BTreeSet::new();
    assert_eq!(decoded_data, data);
    assert_eq!(caveats, expected_caveats);
}
