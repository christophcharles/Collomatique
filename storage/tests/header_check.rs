use std::collections::BTreeSet;

use collomatique_storage::*;

#[test]
fn decode_invalid_file_type() {
    let content = r#"{
    "header": {
        "file_type": "Collomatico",
        "produced_with_version": "0.1.0-alpha.0.99",
        "file_content": "Colloscope"
    },
    "entries": []
}"#;

    let r = collomatique_storage::deserialize_data(content);
    let error = r.expect_err("invalid_file_type should lead to invalid file");

    // An unrecognized discriminant is not a malformed document: it parses,
    // and the header check names it — same treatment as `file_content`.
    let DeserializationError::Decode(decode_error) = error else {
        panic!("The error should be in the decode process")
    };

    let expected_error =
        DecodeError::UnknownFileType(Version::parse("0.1.0-alpha.0.99").expect("valid semver"));
    assert_eq!(decode_error, expected_error);
}

#[test]
fn decode_invalid_file_content() {
    let content = r#"{
    "header": {
        "file_type": "Collomatique",
        "produced_with_version": "0.1.0-alpha.0.99",
        "file_content": "Colloscopes"
    },
    "entries": []
}"#;

    let r = collomatique_storage::deserialize_data(content);
    let error = r.expect_err("invalid_file_type should lead to invalid file");

    let DeserializationError::Decode(decode_error) = error else {
        panic!("The error should be in the decode process")
    };

    let expected_error =
        DecodeError::UnknownFileContent(Version::parse("0.1.0-alpha.0.99").expect("valid semver"));
    assert_eq!(decode_error, expected_error);
}

/// The envelope of the three tests below: a header, and one entry
/// carrying an empty (and therefore trivially valid) `Students` block
///
/// `extra_header_field` and `extra_entry_field` are spliced in verbatim,
/// each with its leading comma.
fn envelope(extra_header_field: &str, extra_entry_field: &str) -> String {
    format!(
        r#"{{
    "header": {{
        "file_type": "Collomatique",
        "produced_with_version": "0.1.0-alpha.0.99",
        "file_content": "Colloscope"{extra_header_field}
    }},
    "entries": [
        {{
            "minimum_spec_version": 2,
            "needed_entry": true,
            "content": {{ "Students": [] }}{extra_entry_field}
        }}
    ]
}}"#
    )
}

#[test]
fn decode_envelope_without_extra_fields() {
    // The control: the envelope the two tests below perturb is valid.
    let content = envelope("", "");

    let (_data, caveats) =
        collomatique_storage::deserialize_data(&content).expect("The plain envelope should decode");
    assert!(caveats.is_empty());
}

#[test]
fn decode_header_with_unknown_field() {
    // The header is a record (spec §2), and "a record with a missing
    // field or an unknown field is invalid" (§3).
    let content = envelope(r#", "junk": 1"#, "");

    let error = collomatique_storage::deserialize_data(&content)
        .expect_err("An unknown header field should lead to an invalid file");

    let DeserializationError::InvalidJson(_) = error else {
        panic!("The error should be in the JSON deserialization process, got {error:?}")
    };
}

#[test]
fn decode_entry_with_unknown_field() {
    // Likewise for an entry: its three fields are fixed by §2.
    let content = envelope("", r#", "junk": 1"#);

    let error = collomatique_storage::deserialize_data(&content)
        .expect_err("An unknown entry field should lead to an invalid file");

    let DeserializationError::InvalidJson(_) = error else {
        panic!("The error should be in the JSON deserialization process, got {error:?}")
    };
}

#[test]
fn decode_more_recent_file() {
    // `Version::new` drops any prerelease, so the built version is a plain
    // release strictly above the current one whatever the package version is.
    let current = current_version();
    let new_version = Version::new(current.major, current.minor + 1, current.patch);

    let content = format!(
        r#"{{
    "header": {{
        "file_type": "Collomatique",
        "produced_with_version": "{new_version}",
        "file_content": "Colloscope"
    }},
    "entries": []
}}"#
    );

    let (inner, caveats) = collomatique_storage::deserialize_data(&content)
        .expect("Too recent version should not lead to invalid decoding");
    let data = collomatique_state_colloscopes::Data::from_inner_data(inner)
        .expect("decoded documents must pass the invariant gate");

    let expected_data = collomatique_state_colloscopes::Data::new();
    let expected_caveats = BTreeSet::from([Caveat::CreatedWithNewerVersion(new_version)]);
    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}

/// A prerelease in the header survives into the caveat, intact
///
/// The prerelease part is the whole point of storing the version as a
/// semver string: the three-integer record that preceded it had no room
/// for `-beta.2` and could not have parsed the string in the first place.
/// The version here is far above any real one, so this stays valid
/// whatever the package version becomes.
#[test]
fn decode_file_produced_with_a_prerelease() {
    let content = r#"{
    "header": {
        "file_type": "Collomatique",
        "produced_with_version": "999.0.0-beta.2",
        "file_content": "Colloscope"
    },
    "entries": []
}"#;

    let (_inner, caveats) = collomatique_storage::deserialize_data(content)
        .expect("A prerelease version should not lead to invalid decoding");

    let expected_version = Version::parse("999.0.0-beta.2").expect("valid semver");
    let expected_caveats = BTreeSet::from([Caveat::CreatedWithNewerVersion(expected_version)]);
    assert_eq!(caveats, expected_caveats);
}

/// A header version that is not a semantic version invalidates the document
///
/// `produced_with_version` is informational, but it is still a record
/// field: an unparsable value fails the envelope, exactly as a malformed
/// number did before.
#[test]
fn decode_file_with_a_malformed_version() {
    let content = r#"{
    "header": {
        "file_type": "Collomatique",
        "produced_with_version": "banana",
        "file_content": "Colloscope"
    },
    "entries": []
}"#;

    let error = collomatique_storage::deserialize_data(content)
        .expect_err("A malformed version should lead to an invalid file");

    let DeserializationError::InvalidJson(_) = error else {
        panic!("The error should be in the JSON deserialization process, got {error:?}")
    };
}
