use collomatique_state_colloscopes::{
    assignments::AssignmentsExternalData, group_lists::GroupListsExternalData,
    incompats::IncompatsExternalData, periods::PeriodsExternalData, rules::RulesExternalData,
    slots::SlotsExternalData, subjects::SubjectsExternalData, teachers::TeachersExternalData,
    week_patterns::WeekPatternsExternalData,
};
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
                    "student_map": {
                        
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
                    "student_map": {
                        "0": {
                            "desc": {
                                "firstname": "Mathieu",
                                "surname": "DURAND",
                                "telephone": null,
                                "email": "mathieu.durand@monfai.fr"
                            },
                            "excluded_periods": []
                        },
                        "42": {
                            "desc": {
                                "firstname": "Christelle",
                                "surname": "DUPONT",
                                "telephone": "06 06 06 06 06",
                                "email": null
                            },
                            "excluded_periods": []
                        }
                    }
                }
            }
        }
    ]
}"#;

    let (data, caveats) = deserialize_data(&content).expect("Should be valid input");

    let expected_students = collomatique_state_colloscopes::students::StudentsExternalData {
        student_map: BTreeMap::from([
            (
                0,
                collomatique_state_colloscopes::students::StudentExternalData {
                    desc: collomatique_state_colloscopes::PersonWithContact {
                        firstname: "Mathieu".to_string(),
                        surname: "DURAND".to_string(),
                        tel: None,
                        email: Some(
                            non_empty_string::NonEmptyString::new(
                                "mathieu.durand@monfai.fr".to_string(),
                            )
                            .unwrap(),
                        ),
                    },
                    excluded_periods: BTreeSet::new(),
                },
            ),
            (
                42,
                collomatique_state_colloscopes::students::StudentExternalData {
                    desc: collomatique_state_colloscopes::PersonWithContact {
                        firstname: "Christelle".to_string(),
                        surname: "DUPONT".to_string(),
                        tel: Some(
                            non_empty_string::NonEmptyString::new("06 06 06 06 06".to_string())
                                .unwrap(),
                        ),
                        email: None,
                    },
                    excluded_periods: BTreeSet::new(),
                },
            ),
        ]),
    };
    let expected_data = collomatique_state_colloscopes::Data::from_data(
        PeriodsExternalData::default(),
        SubjectsExternalData::default(),
        TeachersExternalData::default(),
        expected_students,
        AssignmentsExternalData::default(),
        WeekPatternsExternalData::default(),
        SlotsExternalData::default(),
        IncompatsExternalData::default(),
        GroupListsExternalData::default(),
        RulesExternalData::default(),
        collomatique_state_colloscopes::settings::GeneralSettings::default(),
    )
    .expect("Expected data should not have ID errors");
    let expected_caveats = BTreeSet::new();

    assert_eq!(data, expected_data);
    assert_eq!(caveats, expected_caveats);
}

#[test]
fn encode_and_redecode_simple_student_list() {
    let students = collomatique_state_colloscopes::students::StudentsExternalData {
        student_map: BTreeMap::from([
            (
                0,
                collomatique_state_colloscopes::students::StudentExternalData {
                    desc: collomatique_state_colloscopes::PersonWithContact {
                        firstname: "Mathieu".to_string(),
                        surname: "DURAND".to_string(),
                        tel: None,
                        email: Some(
                            non_empty_string::NonEmptyString::new(
                                "mathieu.durand@monfai.fr".to_string(),
                            )
                            .unwrap(),
                        ),
                    },
                    excluded_periods: BTreeSet::new(),
                },
            ),
            (
                42,
                collomatique_state_colloscopes::students::StudentExternalData {
                    desc: collomatique_state_colloscopes::PersonWithContact {
                        firstname: "Christelle".to_string(),
                        surname: "DUPONT".to_string(),
                        tel: Some(
                            non_empty_string::NonEmptyString::new("06 06 06 06 06".to_string())
                                .unwrap(),
                        ),
                        email: None,
                    },
                    excluded_periods: BTreeSet::new(),
                },
            ),
        ]),
    };

    let orig_data = collomatique_state_colloscopes::Data::from_data(
        PeriodsExternalData::default(),
        SubjectsExternalData::default(),
        TeachersExternalData::default(),
        students,
        AssignmentsExternalData::default(),
        WeekPatternsExternalData::default(),
        SlotsExternalData::default(),
        IncompatsExternalData::default(),
        GroupListsExternalData::default(),
        RulesExternalData::default(),
        collomatique_state_colloscopes::settings::GeneralSettings::default(),
    )
    .expect("Expected data should not have ID errors");

    let content = serialize_data(&orig_data);
    let (data, caveats) = deserialize_data(&content).expect("Should be valid input");

    let expected_caveats = BTreeSet::new();

    assert_eq!(data, orig_data);
    assert_eq!(caveats, expected_caveats);
}

#[test]
fn duplicate_id_should_fail() {
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
                    "student_map": {
                        "0": {
                            "desc": {
                                "firstname": "Mathieu",
                                "surname": "DURAND",
                                "telephone": null,
                                "email": "mathieu.durand@monfai.fr"
                            },
                            "excluded_periods": []
                        },
                        "0": {
                            "desc": {
                                "firstname": "Christelle",
                                "surname": "DUPONT",
                                "telephone": "06 06 06 06 06",
                                "email": null
                            },
                            "excluded_periods": []
                        }
                    }
                }
            }
        }
    ]
}"#;

    assert!(deserialize_data(&content).is_err());
}
