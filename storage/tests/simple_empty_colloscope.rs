use collomatique_storage::*;
use std::collections::BTreeSet;

#[test]
fn decode_empty_file_with_correct_header() {
    let content = r#"{
    "header": {
        "file_type": "Collomatique",
        "produced_with_version": "0.1.0-alpha.0.99",
        "file_content": "Colloscope"
    },
    "entries": []
}"#;

    let (inner, caveats) = deserialize_data(content).expect("Empty file should be valid");
    let data = collomatique_state_colloscopes::Data::from_inner_data(inner)
        .expect("decoded documents must pass the invariant gate");

    let expected_data = collomatique_state_colloscopes::Data::new();
    let expected_caveats = BTreeSet::new();

    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}
