use collomatique_storage::*;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn decode_unknown_unneeded_entry() {
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
            "needed_entry": false,
            "content": {
                "YouShouldReallyNeverCallAnEntryThisWay": {
                    "some_complicated_data_you_cannot_fathom": [42, 43, 44, 45, 0],
                    "some_other_data": true
                }
            }
        }
    ]
}"#;

    let (data, caveats) =
        collomatique_storage::deserialize_data(content).expect("File structure should be valid");
    let expected_data = collomatique_state_colloscopes::Data::new();
    let expected_caveats = BTreeSet::from([Caveat::UnknownEntries]);

    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}

#[test]
fn decode_unknown_unneeded_entry_with_known_data_aside() {
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
            "needed_entry": false,
            "content": {
                "YouShouldReallyNeverCallAnEntryThisWay": {
                    "some_complicated_data_you_cannot_fathom": [42, 43, 44, 45, 0],
                    "some_other_data": true
                }
            }
        },
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

    let (data, caveats) =
        collomatique_storage::deserialize_data(content).expect("File structure should be valid");
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
    let expected_caveats = BTreeSet::from([Caveat::UnknownEntries]);

    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}
