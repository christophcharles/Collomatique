use std::collections::BTreeSet;

use collomatique_storage::*;

#[test]
fn decode_invalid_file_type() {
    let content = r#"{
    "header": {
        "file_type": "Collomatico",
        "produced_with_version": {
            "major": 0,
            "minor": 1,
            "patch": 0
        },
        "file_content": "Colloscope"
    },
    "entries": []
}"#;

    let r = collomatique_storage::deserialize_data(content);
    let error = r.expect_err("invalid_file_type should lead to invalid file");

    let DeserializationError::InvalidJson(_) = error else {
        panic!("The error should be in the JSON deserialization process")
    };
}

#[test]
fn decode_invalid_file_content() {
    let content = r#"{
    "header": {
        "file_type": "Collomatique",
        "produced_with_version": {
            "major": 0,
            "minor": 1,
            "patch": 0
        },
        "file_content": "Colloscopes"
    },
    "entries": []
}"#;

    let r = collomatique_storage::deserialize_data(content);
    let error = r.expect_err("invalid_file_type should lead to invalid file");

    let DeserializationError::Decode(decode_error) = error else {
        panic!("The error should be in the decode process")
    };

    let expected_error = DecodeError::UnknownFileType(Version::new(0, 1, 0));
    assert_eq!(decode_error, expected_error);
}

#[test]
fn decode_more_recent_file() {
    let current_version = Version::current();
    let new_version = Version {
        major: current_version.major,
        minor: current_version.minor + 1,
        patch: current_version.patch,
    };

    let content = format!(
        r#"{{
    "header": {{
        "file_type": "Collomatique",
        "produced_with_version": {{
            "major": {},
            "minor": {},
            "patch": {}
        }},
        "file_content": "Colloscope"
    }},
    "entries": []
}}"#,
        new_version.major, new_version.minor, new_version.patch
    );

    let (data, caveats) = collomatique_storage::deserialize_data(&content)
        .expect("Too recent version should not lead to invalid decoding");

    let expected_data = collomatique_state_colloscopes::Data::new();
    let expected_caveats = BTreeSet::from([Caveat::CreatedWithNewerVersion(new_version)]);
    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}
