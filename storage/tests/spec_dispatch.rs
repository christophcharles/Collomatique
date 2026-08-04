//! Spec-version dispatch tests
//!
//! A file is routed based on the `minimum_spec_version` values declared
//! by its entries: spec 1 (the retired pre-alpha dump format) is rejected
//! with the tombstone error, spec version 0 cannot exist, and everything
//! else (spec 2 and later) goes to the spec-2 pipeline. These tests pin
//! those rules.

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
fn spec1_fixture_is_rejected_as_retired() {
    // The committed spec-1 document (a single InnerDataDump entry with
    // minimum_spec_version 1) must no longer open: it is rejected with
    // the retired-format tombstone. The fixture was produced by the
    // (removed) legacy writer.
    let content = include_str!("fixtures/spec1_empty.json");
    assert!(content.contains("InnerDataDump"));
    assert!(content.contains("\"minimum_spec_version\": 1"));

    let error = deserialize_data(content).expect_err("Spec-1 documents must be rejected");
    assert!(matches!(error, DeserializationError::RetiredSpec1Format));
}

#[test]
fn any_spec1_entry_is_rejected_as_retired() {
    // The tombstone is purely version-driven: a spec-1 entry triggers it
    // even mixed with a spec-2 entry, and without recognizing the block.
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

    let error = deserialize_data(&content).expect_err("A spec-1 entry must be rejected as retired");
    assert!(matches!(error, DeserializationError::RetiredSpec1Format));
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
fn spec_2_document_decodes_cleanly() {
    let data = collomatique_state_colloscopes::Data::new();

    // The spec-2 writer must not emit any InnerDataDump entry, and its
    // documents must decode without caveats.
    let content = serialize_data(&data).expect("Data should be writable");
    assert!(!content.contains("InnerDataDump"));

    let (decoded, caveats) = deserialize_data(&content).expect("Spec-2 document should decode");
    assert_eq!(decoded, data);
    assert!(caveats.is_empty());
}
