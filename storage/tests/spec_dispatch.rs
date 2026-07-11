//! Spec-version dispatch tests
//!
//! During the transition to spec 2, files are routed to the legacy
//! (spec 1) or spec 2 decoding pipeline based on the
//! `minimum_spec_version` values declared by their entries.
//! These tests pin the dispatch rules: all-1 documents stay on the
//! legacy path, and inconsistent version combinations are rejected.

use collomatique_storage::*;
use std::collections::BTreeSet;

fn document_with_entries(entries: &str) -> String {
    format!(
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
    "entries": [{}]
}}"#,
        entries
    )
}

#[test]
fn decode_fails_on_mixed_spec_versions() {
    let content = document_with_entries(
        r#"
        {
            "minimum_spec_version": 1,
            "needed_entry": false,
            "content": { "SomeEntry": {} }
        },
        {
            "minimum_spec_version": 2,
            "needed_entry": false,
            "content": { "SomeOtherEntry": {} }
        }
    "#,
    );

    let r = collomatique_storage::deserialize_data(&content);
    let error = r.expect_err("Mixed spec versions should be rejected");

    let DeserializationError::UnsupportedSpecVersions { versions } = error else {
        panic!("The error should be UnsupportedSpecVersions");
    };
    assert_eq!(versions, BTreeSet::from([1, 2]));
}

#[test]
fn decode_fails_on_spec_version_zero() {
    let content = document_with_entries(
        r#"
        {
            "minimum_spec_version": 0,
            "needed_entry": false,
            "content": { "SomeEntry": {} }
        }
    "#,
    );

    let r = collomatique_storage::deserialize_data(&content);
    let error = r.expect_err("Spec version 0 should be rejected");

    let DeserializationError::UnsupportedSpecVersions { versions } = error else {
        panic!("The error should be UnsupportedSpecVersions");
    };
    assert_eq!(versions, BTreeSet::from([0]));
}

#[test]
fn all_spec_1_document_still_uses_legacy_path() {
    let data = collomatique_state_colloscopes::Data::new();

    // The legacy writer produces a single InnerDataDump entry with
    // minimum_spec_version 1: it must keep decoding through the
    // legacy pipeline.
    let content = serialize_data(&data, true);
    assert!(content.contains("InnerDataDump"));
    assert!(content.contains("\"minimum_spec_version\": 1"));

    let (decoded, caveats) =
        deserialize_data(&content).expect("Legacy document should still decode");
    assert_eq!(decoded, data);
    assert!(caveats.is_empty());
}
