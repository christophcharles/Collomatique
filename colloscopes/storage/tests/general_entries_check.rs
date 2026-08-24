use collomatique_storage::*;
use std::collections::BTreeSet;

#[test]
fn decode_unknown_unneeded_entry() {
    let content = format!(
        r#"{{
    "header": {{
        "file_type": "Collomatique",
        "produced_with_version": "0.1.0-alpha.0.99",
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

    let (inner, caveats) =
        collomatique_storage::deserialize_data(&content).expect("File structure should be valid");
    let data = collomatique_state_colloscopes::Data::from_inner_data(inner)
        .expect("decoded documents must pass the invariant gate");
    let expected_data = collomatique_state_colloscopes::Data::new();
    // The caveat names the block that was dropped, and the spec version it
    // asked for: that is what makes it actionable to whoever reads it.
    let expected_caveats = BTreeSet::from([Caveat::UnknownEntry {
        block_name: "YouShouldReallyNeverCallAnEntryThisWay".to_string(),
        minimum_spec_version: CURRENT_SPEC_VERSION + 1,
    }]);

    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}

#[test]
fn decode_fails_with_unknown_needed_entry() {
    let content = format!(
        r#"{{
    "header": {{
        "file_type": "Collomatique",
        "produced_with_version": "0.1.0-alpha.0.99",
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
        DecodeError::UnknownNeededEntry(Version::parse("0.1.0-alpha.0.99").expect("valid semver"))
    );
}

#[test]
fn decode_fails_on_retired_spec1_entry() {
    // A spec-1 entry is rejected as a retired-format file before any
    // payload interpretation — the tombstone fires on the declared
    // version, regardless of the (here unknown) block name.
    let content = r#"{
    "header": {
        "file_type": "Collomatique",
        "produced_with_version": "0.1.0-alpha.0.99",
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
    assert!(matches!(error, DeserializationError::RetiredSpec1Format));
}
