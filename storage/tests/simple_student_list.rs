use collomatique_storage::*;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn decode_empty_student_list() {
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
                        
                    }
                }
            }
        }
    ]
}"#;

    let (data, caveats) = deserialize_data(&content).expect("Should be valid input");

    let expected_data = collomatique_state_colloscopes::Data::new();
    let expected_caveats = BTreeSet::new();

    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}

#[test]
fn decode_simple_student_list() {
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

    let (data, caveats) = deserialize_data(&content).expect("Should be valid input");

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
    let expected_data = collomatique_state_colloscopes::Data::from_lists(expected_student_list)
        .expect("Expected data should not have ID errors");
    let expected_caveats = BTreeSet::new();

    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}

#[test]
fn encode_and_redecode_simple_student_list() {
    let student_list = BTreeMap::from([
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
    let orig_data = collomatique_state_colloscopes::Data::from_lists(student_list)
        .expect("Expected data should not have ID errors");

    let content = serialize_data(&orig_data);
    let (data, caveats) = deserialize_data(&content).expect("Should be valid input");

    let expected_caveats = BTreeSet::new();

    assert_eq!(data, orig_data);
    assert_eq!(caveats, expected_caveats);
}
