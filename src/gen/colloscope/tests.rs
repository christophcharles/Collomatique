use super::*;

#[test]
fn trivial_validated_data() {
    let general = GeneralData {
        teacher_count: 0,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = SubjectList::new();
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();

    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    let expected_result = ValidatedData {
        general: general.clone(),
        subjects: subjects.clone(),
        incompatibilities: incompatibilities.clone(),
        students: students.clone(),
        slot_groupings: slot_groupings.clone(),
        slot_grouping_incompats: grouping_incompats.clone(),
    };

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Ok(expected_result)
    );
}

#[test]
fn simple_validated_data() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            ],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
            ],
            not_assigned: BTreeSet::from([6, 7, 8]),
        },
    }];
    let incompatibilities = vec![Incompatibility {
        slots: vec![Slot {
            duration: NonZeroU32::new(60).unwrap(),
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        }],
    }];
    let students = vec![
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
    ];
    let slot_groupings = vec![
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 0,
                interrogation: 0,
                slot: 2,
            }]),
        },
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 0,
                interrogation: 0,
                slot: 3,
            }]),
        },
    ];
    let grouping_incompats = SlotGroupingIncompatList::from([SlotGroupingIncompat {
        groupings: BTreeSet::from([0, 1]),
    }]);

    let expected_result = ValidatedData {
        general: general.clone(),
        subjects: subjects.clone(),
        incompatibilities: incompatibilities.clone(),
        students: students.clone(),
        slot_groupings: slot_groupings.clone(),
        slot_grouping_incompats: grouping_incompats.clone(),
    };

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Ok(expected_result)
    );
}

#[test]
fn invalid_students_per_interrogation() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(0, 0).unwrap(),
            }],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![],
            not_assigned: BTreeSet::new(),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SubjectWithInvalidStudentsPerInterrogationRange(
            0,
            NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(1).unwrap()
        ))
    );
}

#[test]
fn subject_slot_overlaps_next_day() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 1).unwrap(),
            }],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![],
            not_assigned: BTreeSet::new(),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SubjectWithSlotOverlappingNextDay(0, 0, 0))
    );
}

#[test]
fn incompatibility_slot_overlaps_next_day() {
    let general = GeneralData {
        teacher_count: 0,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };
    let subjects = SubjectList::new();
    let incompatibilities = vec![Incompatibility {
        slots: vec![Slot {
            duration: NonZeroU32::new(60).unwrap(),
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 1).unwrap(),
            },
        }],
    }];
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();
    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::IncompatibilityWithSlotOverlappingNextDay(0, 0))
    );
}

#[test]
fn invalid_teacher_number() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 1,
            slots: vec![SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 0).unwrap(),
            }],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![],
            not_assigned: BTreeSet::new(),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SubjectWithInvalidTeacher(0, 0, 1))
    );
}

#[test]
fn invalid_incompatibility_number() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = SubjectList::new();
    let incompatibilities = vec![Incompatibility {
        slots: vec![Slot {
            duration: NonZeroU32::new(60).unwrap(),
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 0).unwrap(),
            },
        }],
    }];
    let students = vec![Student {
        incompatibilities: BTreeSet::from([1]),
    }];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::StudentWithInvalidIncompatibility(0, 1))
    );
}

#[test]
fn slot_ref_has_invalid_subject() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![
        Subject {
            students_per_interrogation: NonZeroUsize::new(2).unwrap()
                ..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                }],
            }],
            groups: GroupsDesc {
                assigned_to_group: vec![],
                not_assigned: BTreeSet::new(),
            },
        },
        Subject {
            students_per_interrogation: NonZeroUsize::new(2).unwrap()
                ..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                }],
            }],
            groups: GroupsDesc {
                assigned_to_group: vec![],
                not_assigned: BTreeSet::new(),
            },
        },
    ];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = vec![SlotGrouping {
        slots: BTreeSet::from([
            SlotRef {
                subject: 1,
                interrogation: 0,
                slot: 0,
            },
            SlotRef {
                subject: 2,
                interrogation: 0,
                slot: 0,
            },
        ]),
    }];
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SlotGroupingWithInvalidSubject(
            0,
            SlotRef {
                subject: 2,
                interrogation: 0,
                slot: 0,
            }
        ))
    );
}

#[test]
fn slot_ref_has_invalid_interrogation() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![
        Subject {
            students_per_interrogation: NonZeroUsize::new(2).unwrap()
                ..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                }],
            }],
            groups: GroupsDesc {
                assigned_to_group: vec![],
                not_assigned: BTreeSet::new(),
            },
        },
        Subject {
            students_per_interrogation: NonZeroUsize::new(2).unwrap()
                ..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                }],
            }],
            groups: GroupsDesc {
                assigned_to_group: vec![],
                not_assigned: BTreeSet::new(),
            },
        },
    ];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = vec![SlotGrouping {
        slots: BTreeSet::from([
            SlotRef {
                subject: 1,
                interrogation: 1,
                slot: 0,
            },
            SlotRef {
                subject: 0,
                interrogation: 0,
                slot: 0,
            },
        ]),
    }];
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SlotGroupingWithInvalidInterrogation(
            0,
            SlotRef {
                subject: 1,
                interrogation: 1,
                slot: 0,
            }
        ))
    );
}

#[test]
fn slot_ref_has_invalid_slot() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![
        Subject {
            students_per_interrogation: NonZeroUsize::new(2).unwrap()
                ..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                }],
            }],
            groups: GroupsDesc {
                assigned_to_group: vec![],
                not_assigned: BTreeSet::new(),
            },
        },
        Subject {
            students_per_interrogation: NonZeroUsize::new(2).unwrap()
                ..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                }],
            }],
            groups: GroupsDesc {
                assigned_to_group: vec![],
                not_assigned: BTreeSet::new(),
            },
        },
    ];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = vec![SlotGrouping {
        slots: BTreeSet::from([
            SlotRef {
                subject: 1,
                interrogation: 0,
                slot: 0,
            },
            SlotRef {
                subject: 0,
                interrogation: 0,
                slot: 1,
            },
        ]),
    }];
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SlotGroupingWithInvalidSlot(
            0,
            SlotRef {
                subject: 0,
                interrogation: 0,
                slot: 1,
            }
        ))
    );
}

#[test]
fn grouping_incompact_invalid_ref() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![
        Subject {
            students_per_interrogation: NonZeroUsize::new(2).unwrap()
                ..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                }],
            }],
            groups: GroupsDesc {
                assigned_to_group: vec![],
                not_assigned: BTreeSet::new(),
            },
        },
        Subject {
            students_per_interrogation: NonZeroUsize::new(2).unwrap()
                ..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                }],
            }],
            groups: GroupsDesc {
                assigned_to_group: vec![],
                not_assigned: BTreeSet::new(),
            },
        },
    ];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = vec![
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 1,
                interrogation: 0,
                slot: 0,
            }]),
        },
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 0,
                interrogation: 0,
                slot: 0,
            }]),
        },
    ];
    let grouping_incompats = vec![SlotGroupingIncompat {
        groupings: BTreeSet::from([0, 2]),
    }];

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SlotGroupingIncompatWithInvalidSlotGrouping(0, 2))
    );
}

#[test]
fn invalid_interrogations_per_week() {
    let general = GeneralData {
        teacher_count: 0,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: Some(10..8),
    };

    let subjects = SubjectList::new();
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();

    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SlotGeneralDataWithInvalidInterrogationsPerWeek(
            10..8
        ))
    );
}

#[test]
fn duplicated_groups() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            ],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
            ],
            not_assigned: BTreeSet::new(),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SubjectWithDuplicatedStudentInGroups(0, 0, 0, 2))
    );
}

#[test]
fn duplicated_student() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            ],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 7, 8]),
                    can_be_extended: false,
                },
            ],
            not_assigned: BTreeSet::new(),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SubjectWithDuplicatedStudentInGroups(0, 3, 1, 2))
    );
}

#[test]
fn duplicated_student_not_assigned() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            ],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
            ],
            not_assigned: BTreeSet::from([3]),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SubjectWithDuplicatedStudentInGroupsAndUnassigned(
            0, 3, 1
        ))
    );
}

#[test]
fn invalid_student_in_group() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            ],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![
                GroupDesc {
                    students: BTreeSet::from([1, 2, 3]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([4, 5, 6]),
                    can_be_extended: false,
                },
            ],
            not_assigned: BTreeSet::from([0]),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SubjectWithInvalidAssignedStudent(0, 1, 6))
    );
}

#[test]
fn invalid_student_not_assigned() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            ],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![GroupDesc {
                students: BTreeSet::from([0, 1, 2]),
                can_be_extended: false,
            }],
            not_assigned: BTreeSet::from([4, 7]),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SubjectWithInvalidNotAssignedStudent(0, 7))
    );
}

#[test]
fn empty_group() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            ],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([]),
                    can_be_extended: false,
                },
            ],
            not_assigned: BTreeSet::new(),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SubjectWithEmptyGroup(0, 2))
    );
}

#[test]
fn extensible_empty_group() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            ],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([]),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::new(),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SubjectWithEmptyGroup(0, 2))
    );
}

#[test]
fn group_too_large() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            ],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([2, 3, 4, 5]),
                    can_be_extended: false,
                },
            ],
            not_assigned: BTreeSet::new(),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SubjectWithTooLargeAssignedGroup(
            0,
            1,
            NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap()
        ))
    );
}

#[test]
fn non_extensible_too_small_group() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            ],
        }],
        groups: GroupsDesc {
            assigned_to_group: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([6]),
                    can_be_extended: false,
                },
            ],
            not_assigned: BTreeSet::new(),
        },
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatList::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SubjectWithTooSmallNonExtensibleGroup(
            0,
            2,
            NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap()
        ))
    );
}
