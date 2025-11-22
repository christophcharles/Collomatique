use collomatique_storage::*;
use std::collections::BTreeSet;

#[test]
fn decode_unknown_unneeded_entry() {
    let content = format!(
        r#"{{
    "header": {{
        "file_type": "Collomatique",
        "produced_with_version": {{
            "major": 0,
            "minor": 1,
            "patch": 0
        }},
        "file_content": "Colloscope"
    }},
    "entries": [
        {{
            "minimum_spec_version": {},
            "needed_entry": false,
            "content": {{
                "YouShouldReallyNeverCallAnEntryThisWay": {{
                    "some_complicated_data_you_cannot_fathom": [42, 43, 44, 45, 0],
                    "some_other_data": true
                }}
            }}
        }}
    ]
}}"#,
        CURRENT_SPEC_VERSION + 1
    );

    let (data, caveats) =
        collomatique_storage::deserialize_data(&content).expect("File structure should be valid");
    let expected_data = collomatique_state_colloscopes::Data::new();
    let expected_caveats = BTreeSet::from([Caveat::UnknownEntries]);

    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}

#[test]
fn decode_fails_with_unknown_needed_entry() {
    let content = format!(
        r#"{{
    "header": {{
        "file_type": "Collomatique",
        "produced_with_version": {{
            "major": 0,
            "minor": 1,
            "patch": 0
        }},
        "file_content": "Colloscope"
    }},
    "entries": [
        {{
            "minimum_spec_version": {},
            "needed_entry": true,
            "content": {{
                "YouShouldReallyNeverCallAnEntryThisWay": {{
                    "some_complicated_data_you_cannot_fathom": [42, 43, 44, 45, 0],
                    "some_other_data": true
                }}
            }}
        }}
    ]
}}"#,
        CURRENT_SPEC_VERSION + 1
    );

    let r = collomatique_storage::deserialize_data(&content);
    let error = r.expect_err("Should have an error");
    let DeserializationError::Decode(decode_error) = error else {
        panic!("Error should be in the decode process");
    };

    assert_eq!(
        decode_error,
        DecodeError::UnknownNeededEntry(Version::new(0, 1, 0))
    );
}

#[test]
fn decode_fails_with_unknown_entry_with_wrong_minimum_spec() {
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
    "entries": [
        {
            "minimum_spec_version": 1,
            "needed_entry": true,
            "content": {
                "YouShouldReallyNeverCallAnEntryThisWay": {
                    "some_complicated_data_you_cannot_fathom": [42, 43, 44, 45, 0],
                    "some_other_data": true
                }
            }
        }
    ]
}"#;

    let r = collomatique_storage::deserialize_data(&content);
    let error = r.expect_err("Should have an error");
    let DeserializationError::Decode(decode_error) = error else {
        panic!("Error should be in the decode process");
    };

    assert_eq!(decode_error, DecodeError::ProbablyIllformedEntry);
}
