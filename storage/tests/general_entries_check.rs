use collomatique_storage::*;
use std::collections::{BTreeMap, BTreeSet};

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
fn decode_unknown_unneeded_entry_with_known_data_aside() {
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
        }},
        {{
            "minimum_spec_version": 1,
            "needed_entry": true,
            "content": {{
                "StudentList": {{
                    "map": {{
                        "0": {{
                            "firstname": "Mathieu",
                            "surname": "DURAND",
                            "telephone": null,
                            "email": "mathieu.durand@monfai.fr"
                        }},
                        "42": {{
                            "firstname": "Christelle",
                            "surname": "DUPONT",
                            "telephone": "06 06 06 06 06",
                            "email": null
                        }}
                    }}
                }}
            }}
        }}
    ]
}}"#,
        CURRENT_SPEC_VERSION + 1
    );

    let (data, caveats) =
        collomatique_storage::deserialize_data(&content).expect("File structure should be valid");
    let expected_student_list = BTreeMap::from([
        (
            0,
            collomatique_state_colloscopes::PersonWithContact {
                firstname: "Mathieu".to_string(),
                surname: "DURAND".to_string(),
                tel: None,
                email: Some("mathieu.durand@monfai.fr".to_string()),
            },
        ),
        (
            42,
            collomatique_state_colloscopes::PersonWithContact {
                firstname: "Christelle".to_string(),
                surname: "DUPONT".to_string(),
                tel: Some("06 06 06 06 06".to_string()),
                email: None,
            },
        ),
    ]);
    let expected_data = collomatique_state_colloscopes::Data::from_data(expected_student_list)
        .expect("Expected data should not have ID errors");
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

    assert_eq!(decode_error, DecodeError::UnknownNeededEntry);
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

    assert_eq!(decode_error, DecodeError::MismatchedSpecRequirementInEntry);
}

#[test]
fn decode_fails_with_known_data_aside_with_wrong_minimum_spec() {
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
            "minimum_spec_version": 2,
            "needed_entry": true,
            "content": {
                "StudentList": {
                    "map": {
                        "0": {
                            "firstname": "Mathieu",
                            "surname": "DURAND",
                            "telephone": null,
                            "email": "mathieu.durand@monfai.fr"
                        },
                        "42": {
                            "firstname": "Christelle",
                            "surname": "DUPONT",
                            "telephone": "06 06 06 06 06",
                            "email": null
                        }
                    }
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

    assert_eq!(decode_error, DecodeError::MismatchedSpecRequirementInEntry);
}

#[test]
fn decode_fails_with_known_data_aside_with_wrong_neediness() {
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
            "needed_entry": false,
            "content": {
                "StudentList": {
                    "map": {
                        "0": {
                            "firstname": "Mathieu",
                            "surname": "DURAND",
                            "telephone": null,
                            "email": "mathieu.durand@monfai.fr"
                        },
                        "42": {
                            "firstname": "Christelle",
                            "surname": "DUPONT",
                            "telephone": "06 06 06 06 06",
                            "email": null
                        }
                    }
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

    assert_eq!(decode_error, DecodeError::MismatchedSpecRequirementInEntry);
}

#[test]
fn decode_fails_on_duplicate_known_data() {
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
                "StudentList": {
                    "map": {
                        "0": {
                            "firstname": "Mathieu",
                            "surname": "DURAND",
                            "telephone": null,
                            "email": "mathieu.durand@monfai.fr"
                        }
                    }
                }
            }
        },
        {
            "minimum_spec_version": 1,
            "needed_entry": true,
            "content": {
                "StudentList": {
                    "map": {
                        "42": {
                            "firstname": "Christelle",
                            "surname": "DUPONT",
                            "telephone": "06 06 06 06 06",
                            "email": null
                        }
                    }
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

    assert_eq!(
        decode_error,
        DecodeError::DuplicatedEntry(EntryTag::StudentList)
    );
}
