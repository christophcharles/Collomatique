use super::*;

#[test]
fn trivial_validated_data() {
    let general = GeneralData {
        teacher_count: 0,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = SubjectList::new();
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();

    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::from([]),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([5, 6, 7, 8]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![Incompatibility {
        slots: vec![SlotWithDuration {
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
                slot: 2,
            }]),
        },
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 0,
                slot: 3,
            }]),
        },
    ];
    let grouping_incompats = SlotGroupingIncompatSet::from([SlotGroupingIncompat {
        groupings: BTreeSet::from([0, 1]),
        max_count: NonZeroUsize::new(1).unwrap(),
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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(1).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![SlotWithTeacher {
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(0, 0).unwrap(),
            },
        }],
        groups: GroupsDesc {
            prefilled_groups: vec![],
            not_assigned: BTreeSet::new(),
        },
        ..Subject::default()
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SubjectWithInvalidStudentsPerSlotRange(
            0,
            NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(1).unwrap()
        ))
    );
}

#[test]
fn subject_slot_overlaps_next_day() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![SlotWithTeacher {
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 1).unwrap(),
            },
        }],
        groups: GroupsDesc {
            prefilled_groups: vec![],
            not_assigned: BTreeSet::new(),
        },
        ..Subject::default()
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SubjectWithSlotOverlappingNextDay(0, 0))
    );
}

#[test]
fn incompatibility_slot_overlaps_next_day() {
    let general = GeneralData {
        teacher_count: 0,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };
    let subjects = SubjectList::new();
    let incompatibilities = vec![Incompatibility {
        slots: vec![SlotWithDuration {
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
    let grouping_incompats = SlotGroupingIncompatSet::new();
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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![SlotWithTeacher {
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 0).unwrap(),
            },
        }],
        groups: GroupsDesc {
            prefilled_groups: vec![],
            not_assigned: BTreeSet::new(),
        },
        ..Subject::default()
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        max_interrogations_per_day: None,
    };

    let subjects = SubjectList::new();
    let incompatibilities = vec![Incompatibility {
        slots: vec![SlotWithDuration {
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = vec![SlotGrouping {
        slots: BTreeSet::from([
            SlotRef {
                subject: 1,
                slot: 0,
            },
            SlotRef {
                subject: 2,
                slot: 0,
            },
        ]),
    }];
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
                slot: 0,
            }
        ))
    );
}

#[test]
fn slot_ref_has_invalid_slot() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = vec![SlotGrouping {
        slots: BTreeSet::from([
            SlotRef {
                subject: 1,
                slot: 0,
            },
            SlotRef {
                subject: 0,
                slot: 1,
            },
        ]),
    }];
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
                slot: 1,
            }
        ))
    );
}

#[test]
fn slot_grouping_overlap() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = vec![
        SlotGrouping {
            slots: BTreeSet::from([
                SlotRef {
                    subject: 1,
                    slot: 0,
                },
                SlotRef {
                    subject: 0,
                    slot: 1,
                },
            ]),
        },
        SlotGrouping {
            slots: BTreeSet::from([
                SlotRef {
                    subject: 0,
                    slot: 0,
                },
                SlotRef {
                    subject: 0,
                    slot: 1,
                },
            ]),
        },
    ];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SlotGroupingOverlap(
            0,
            1,
            SlotRef {
                subject: 0,
                slot: 1,
            }
        ))
    );
}

#[test]
fn grouping_incompact_invalid_ref() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            slots: vec![SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = vec![
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 1,
                slot: 0,
            }]),
        },
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 0,
                slot: 0,
            }]),
        },
    ];
    let grouping_incompats = BTreeSet::from([SlotGroupingIncompat {
        groupings: BTreeSet::from([0, 2]),
        max_count: NonZeroUsize::new(1).unwrap(),
    }]);

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
        max_interrogations_per_day: None,
    };

    let subjects = SubjectList::new();
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();

    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
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
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
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
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
            ],
            not_assigned: BTreeSet::from([3]),
        },
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
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
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![GroupDesc {
                students: BTreeSet::from([0, 1, 2]),
                can_be_extended: false,
            }],
            not_assigned: BTreeSet::from([4, 7]),
        },
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
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
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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

#[test]
fn extensible_empty_group() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
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
            not_assigned: BTreeSet::from([6, 7]),
        },
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        None
    );
}

#[test]
fn group_too_large() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
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
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
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
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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

#[test]
fn too_few_groups() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5, 6, 7, 8]),
        },
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        Some(Error::SubjectWithTooFewGroups(
            0,
            NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap()
        ))
    );
}

#[test]
fn too_many_groups() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5, 6, 7, 8]),
        },
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        Some(Error::SubjectWithTooManyGroups(
            0,
            NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap()
        ))
    );
}

#[test]
fn no_full_period() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

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
        Some(Error::SubjectWithPeriodicityTooBig(2, 1))
    );
}

#[test]
fn group_in_slot_variables() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let group_in_slot_variables = ilp_translator.build_group_in_slot_variables();

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        Variable::GroupInSlot { subject: 0, slot: 0, group: 0 },
        Variable::GroupInSlot { subject: 0, slot: 0, group: 1 },
        Variable::GroupInSlot { subject: 0, slot: 0, group: 2 },
        Variable::GroupInSlot { subject: 0, slot: 0, group: 3 },
        Variable::GroupInSlot { subject: 0, slot: 1, group: 0 },
        Variable::GroupInSlot { subject: 0, slot: 1, group: 1 },
        Variable::GroupInSlot { subject: 0, slot: 1, group: 2 },
        Variable::GroupInSlot { subject: 0, slot: 1, group: 3 },
        Variable::GroupInSlot { subject: 0, slot: 2, group: 0 },
        Variable::GroupInSlot { subject: 0, slot: 2, group: 1 },
        Variable::GroupInSlot { subject: 0, slot: 2, group: 2 },
        Variable::GroupInSlot { subject: 0, slot: 2, group: 3 },
        Variable::GroupInSlot { subject: 0, slot: 3, group: 0 },
        Variable::GroupInSlot { subject: 0, slot: 3, group: 1 },
        Variable::GroupInSlot { subject: 0, slot: 3, group: 2 },
        Variable::GroupInSlot { subject: 0, slot: 3, group: 3 },
        Variable::GroupInSlot { subject: 0, slot: 4, group: 0 },
        Variable::GroupInSlot { subject: 0, slot: 4, group: 1 },
        Variable::GroupInSlot { subject: 0, slot: 4, group: 2 },
        Variable::GroupInSlot { subject: 0, slot: 4, group: 3 },
        Variable::GroupInSlot { subject: 0, slot: 5, group: 0 },
        Variable::GroupInSlot { subject: 0, slot: 5, group: 1 },
        Variable::GroupInSlot { subject: 0, slot: 5, group: 2 },
        Variable::GroupInSlot { subject: 0, slot: 5, group: 3 },
        Variable::GroupInSlot { subject: 0, slot: 6, group: 0 },
        Variable::GroupInSlot { subject: 0, slot: 6, group: 1 },
        Variable::GroupInSlot { subject: 0, slot: 6, group: 2 },
        Variable::GroupInSlot { subject: 0, slot: 6, group: 3 },
        Variable::GroupInSlot { subject: 0, slot: 7, group: 0 },
        Variable::GroupInSlot { subject: 0, slot: 7, group: 1 },
        Variable::GroupInSlot { subject: 0, slot: 7, group: 2 },
        Variable::GroupInSlot { subject: 0, slot: 7, group: 3 },
    ]);

    assert_eq!(group_in_slot_variables, expected_result);
}

#[test]
fn dynamic_group_assignment_variables() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let dynamic_group_assignment_variables =
        ilp_translator.build_dynamic_group_assignment_variables();

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 2, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 2, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 2, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 2, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 2, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 2, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 3, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 3, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 3, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 3, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 3, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 3, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 2, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 2, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 2, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 2, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 2, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 2, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 3, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 3, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 3, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 3, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 3, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 3, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 2, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 2, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 2, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 2, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 2, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 2, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 3, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 3, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 3, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 3, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 3, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 3, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 2, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 2, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 2, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 2, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 2, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 2, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 3, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 3, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 3, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 3, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 3, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 3, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 2, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 2, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 2, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 2, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 2, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 2, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 3, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 3, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 3, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 3, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 3, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 3, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 2, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 2, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 2, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 2, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 2, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 2, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 3, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 3, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 3, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 3, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 3, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 5, group: 3, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 2, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 2, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 2, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 2, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 2, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 2, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 3, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 3, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 3, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 3, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 3, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 3, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 2, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 2, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 2, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 2, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 2, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 2, student: 11 },

        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 3, student: 6 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 3, student: 7 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 3, student: 8 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 3, student: 9 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 3, student: 10 },
        Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 3, student: 11 },
    ]);

    assert_eq!(dynamic_group_assignment_variables, expected_result);
}

#[test]
fn student_in_group_variables() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let student_in_group_variables = ilp_translator.build_student_in_group_variables();

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        Variable::StudentInGroup { subject: 0, student: 6, group: 2 },
        Variable::StudentInGroup { subject: 0, student: 7, group: 2 },
        Variable::StudentInGroup { subject: 0, student: 8, group: 2 },
        Variable::StudentInGroup { subject: 0, student: 9, group: 2 },
        Variable::StudentInGroup { subject: 0, student: 10, group: 2 },
        Variable::StudentInGroup { subject: 0, student: 11, group: 2 },
       
        Variable::StudentInGroup { subject: 0, student: 6, group: 3 },
        Variable::StudentInGroup { subject: 0, student: 7, group: 3 },
        Variable::StudentInGroup { subject: 0, student: 8, group: 3 },
        Variable::StudentInGroup { subject: 0, student: 9, group: 3 },
        Variable::StudentInGroup { subject: 0, student: 10, group: 3 },
        Variable::StudentInGroup { subject: 0, student: 11, group: 3 }, 
    ]);

    assert_eq!(student_in_group_variables, expected_result);
}

#[test]
fn with_student_not_in_last_period_variables() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(3).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([6, 7, 8]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([9, 10, 11]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([0, 1, 2, 3, 4, 5]),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let student_not_in_last_period = ilp_translator.build_student_not_in_last_period_variables();

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        Variable::StudentNotInLastPeriod { subject: 0, student: 0 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 1 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 2 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 3 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 4 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 5 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 6 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 7 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 8 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 9 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 10 },
        Variable::StudentNotInLastPeriod { subject: 0, student: 11 },

        Variable::StudentNotInLastPeriod { subject: 1, student: 0 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 1 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 2 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 3 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 4 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 5 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 6 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 7 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 8 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 9 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 10 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 11 },
    ]);

    assert_eq!(student_not_in_last_period, expected_result);
}

#[test]
fn without_student_not_in_last_period_variables() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([6, 7, 8]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([9, 10, 11]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([0, 1, 2, 3, 4, 5]),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let student_not_in_last_period_variables =
        ilp_translator.build_student_not_in_last_period_variables();

    let expected_result = BTreeSet::new();

    assert_eq!(student_not_in_last_period_variables, expected_result);
}

#[test]
fn mixed_case_for_student_not_in_last_period_variables() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(3).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(3).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([6, 7, 8]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([9, 10, 11]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([0, 1, 2, 3, 4, 5]),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let student_not_in_last_period_variables =
        ilp_translator.build_student_not_in_last_period_variables();

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        Variable::StudentNotInLastPeriod { subject: 1, student: 0 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 1 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 2 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 3 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 4 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 5 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 6 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 7 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 8 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 9 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 10 },
        Variable::StudentNotInLastPeriod { subject: 1, student: 11 },
    ]);

    assert_eq!(student_not_in_last_period_variables, expected_result);
}

#[test]
fn periodicity_variables_for_strict_period() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let exact_periodicity_variables = ilp_translator.build_periodicity_variables();

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        Variable::Periodicity { subject: 0, student: 0, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 0, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 1, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 1, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 2, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 2, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 3, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 3, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 4, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 4, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 5, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 5, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 6, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 6, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 7, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 7, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 8, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 8, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 9, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 9, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 10, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 10, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 11, week_modulo: 0},
        Variable::Periodicity { subject: 0, student: 11, week_modulo: 1, },
    ]);

    assert_eq!(exact_periodicity_variables, expected_result);
}

#[test]
fn periodicity_variables_for_loose_unfinished_period() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(3).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let exact_periodicity_variables = ilp_translator.build_periodicity_variables();

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        Variable::Periodicity { subject: 0, student: 0, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 0, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 1, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 1, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 2, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 2, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 3, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 3, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 4, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 4, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 5, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 5, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 6, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 6, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 7, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 7, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 8, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 8, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 9, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 9, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 10, week_modulo: 0 },
        Variable::Periodicity { subject: 0, student: 10, week_modulo: 1 },
        Variable::Periodicity { subject: 0, student: 11, week_modulo: 0},
        Variable::Periodicity { subject: 0, student: 11, week_modulo: 1, },
    ]);

    assert_eq!(exact_periodicity_variables, expected_result);
}

#[test]
fn without_periodicity_variables_for_loose_complete_period() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4, 5]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let exact_periodicity_variables = ilp_translator.build_periodicity_variables();

    let expected_result = BTreeSet::new();

    assert_eq!(exact_periodicity_variables, expected_result);
}

#[test]
fn use_grouping() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
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
    let slot_groupings = vec![
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 0,
                slot: 2,
            }]),
        },
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 0,
                slot: 3,
            }]),
        },
    ];
    let grouping_incompats = SlotGroupingIncompatSet::from([SlotGroupingIncompat {
        groupings: BTreeSet::from([0, 1]),
        max_count: NonZeroUsize::new(1).unwrap(),
    }]);

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let use_grouping_variables = ilp_translator.build_use_grouping_variables();

    let expected_result = BTreeSet::from([Variable::UseGrouping(0), Variable::UseGrouping(1)]);

    assert_eq!(use_grouping_variables, expected_result);
}

#[test]
fn at_most_max_groups_per_slot_constraints() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(2).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Thursday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Thursday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                ],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let at_most_max_groups_per_slot_constraints =
        ilp_translator.build_at_most_max_groups_per_slot_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 2 });
    #[rustfmt::skip]
    let gis_0_3 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 3 });

    #[rustfmt::skip]
    let gis_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });
    #[rustfmt::skip]
    let gis_1_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 2 });
    #[rustfmt::skip]
    let gis_1_3 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 3 });

    #[rustfmt::skip]
    let gis_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_2_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 1 });
    #[rustfmt::skip]
    let gis_2_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 2 });
    #[rustfmt::skip]
    let gis_2_3 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 3 });

    #[rustfmt::skip]
    let gis_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 0 });
    #[rustfmt::skip]
    let gis_3_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 1 });
    #[rustfmt::skip]
    let gis_3_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 2 });
    #[rustfmt::skip]
    let gis_3_3 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 3 });

    #[rustfmt::skip]
    let gis_4_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 0 });
    #[rustfmt::skip]
    let gis_4_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 1 });
    #[rustfmt::skip]
    let gis_4_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 2 });
    #[rustfmt::skip]
    let gis_4_3 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 3 });

    #[rustfmt::skip]
    let gis_5_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 0 });
    #[rustfmt::skip]
    let gis_5_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 1 });
    #[rustfmt::skip]
    let gis_5_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 2 });
    #[rustfmt::skip]
    let gis_5_3 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 3 });

    #[rustfmt::skip]
    let gis_6_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 0 });
    #[rustfmt::skip]
    let gis_6_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 1 });
    #[rustfmt::skip]
    let gis_6_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 2 });
    #[rustfmt::skip]
    let gis_6_3 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 3 });

    #[rustfmt::skip]
    let gis_7_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 0 });
    #[rustfmt::skip]
    let gis_7_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 1 });
    #[rustfmt::skip]
    let gis_7_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 2 });
    #[rustfmt::skip]
    let gis_7_3 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 3 });

    #[rustfmt::skip]
    let gis_1_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_1_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_1_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 1 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (gis_0_0 + gis_0_1 + gis_0_2 + gis_0_3).leq(&Expr::constant(1)),
        (gis_1_0 + gis_1_1 + gis_1_2 + gis_1_3).leq(&Expr::constant(1)),
        (gis_2_0 + gis_2_1 + gis_2_2 + gis_2_3).leq(&Expr::constant(1)),
        (gis_3_0 + gis_3_1 + gis_3_2 + gis_3_3).leq(&Expr::constant(1)),
        (gis_4_0 + gis_4_1 + gis_4_2 + gis_4_3).leq(&Expr::constant(1)),
        (gis_5_0 + gis_5_1 + gis_5_2 + gis_5_3).leq(&Expr::constant(1)),
        (gis_6_0 + gis_6_1 + gis_6_2 + gis_6_3).leq(&Expr::constant(1)),
        (gis_7_0 + gis_7_1 + gis_7_2 + gis_7_3).leq(&Expr::constant(1)),
        (gis_1_0_0 + gis_1_0_1).leq(&Expr::constant(2)),
        (gis_1_1_0 + gis_1_1_1).leq(&Expr::constant(2)),
    ]);

    assert_eq!(at_most_max_groups_per_slot_constraints, expected_result);
}

#[test]
fn at_most_one_interrogation_per_time_unit_constraints() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([0, 1, 2]),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let at_most_one_interrogation_per_time_unit_constraints =
        ilp_translator.build_at_most_one_interrogation_per_time_unit_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_1_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 0 });

    #[rustfmt::skip]
    let dga_0_0_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_0_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_1_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_1_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_1_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 5 });

    #[rustfmt::skip]
    let dga_1_0_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_0_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_0_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 2 });
    #[rustfmt::skip]
    let dga_1_1_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_1_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_1_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &dga_1_0_1_0).leq(&Expr::constant(1)),
        (&gis_0_0_0 + &dga_1_0_1_1).leq(&Expr::constant(1)),
        (&gis_0_0_0 + &dga_1_0_1_2).leq(&Expr::constant(1)),

        (&gis_0_1_0 + &dga_1_1_1_0).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &dga_1_1_1_1).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &dga_1_1_1_2).leq(&Expr::constant(1)),

        (&gis_1_0_0 + &dga_0_0_1_3).leq(&Expr::constant(1)),
        (&gis_1_0_0 + &dga_0_0_1_4).leq(&Expr::constant(1)),
        (&gis_1_0_0 + &dga_0_0_1_5).leq(&Expr::constant(1)),

        (&gis_1_1_0 + &dga_0_1_1_3).leq(&Expr::constant(1)),
        (&gis_1_1_0 + &dga_0_1_1_4).leq(&Expr::constant(1)),
        (&gis_1_1_0 + &dga_0_1_1_5).leq(&Expr::constant(1)),
    ]);

    assert_eq!(
        at_most_one_interrogation_per_time_unit_constraints,
        expected_result
    );
}

#[test]
fn one_interrogation_per_period() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(4).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 3,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 3,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([0, 1, 2]),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let one_interrogation_per_period_contraints =
        ilp_translator.build_one_interrogation_per_period_contraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_0_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 0 });
    #[rustfmt::skip]
    let gis_1_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_1_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_1_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 3, group: 0 });

    #[rustfmt::skip]
    let dga_0_0_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_0_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_1_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_1_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_1_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_2_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_2_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_2_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_3_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_3_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_3_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 1, student: 5 });

    #[rustfmt::skip]
    let dga_1_0_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_0_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_0_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 2 });
    #[rustfmt::skip]
    let dga_1_1_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_1_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_1_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 2 });
    #[rustfmt::skip]
    let dga_1_2_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 2, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_2_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 2, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_2_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 2, group: 1, student: 2 });
    #[rustfmt::skip]
    let dga_1_3_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 3, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_3_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 3, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_3_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 3, group: 1, student: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0).eq(&Expr::constant(1)),
        (&dga_0_0_1_3 + &dga_0_1_1_3).eq(&Expr::constant(1)),
        (&dga_0_0_1_4 + &dga_0_1_1_4).eq(&Expr::constant(1)),
        (&dga_0_0_1_5 + &dga_0_1_1_5).eq(&Expr::constant(1)),

        (&gis_0_2_0 + &gis_0_3_0).eq(&Expr::constant(1)),
        (&dga_0_2_1_3 + &dga_0_3_1_3).eq(&Expr::constant(1)),
        (&dga_0_2_1_4 + &dga_0_3_1_4).eq(&Expr::constant(1)),
        (&dga_0_2_1_5 + &dga_0_3_1_5).eq(&Expr::constant(1)),

        (&gis_1_0_0 + &gis_1_1_0).eq(&Expr::constant(1)),
        (&dga_1_0_1_0 + &dga_1_1_1_0).eq(&Expr::constant(1)),
        (&dga_1_0_1_1 + &dga_1_1_1_1).eq(&Expr::constant(1)),
        (&dga_1_0_1_2 + &dga_1_1_1_2).eq(&Expr::constant(1)),

        (&gis_1_2_0 + &gis_1_3_0).eq(&Expr::constant(1)),
        (&dga_1_2_1_0 + &dga_1_3_1_0).eq(&Expr::constant(1)),
        (&dga_1_2_1_1 + &dga_1_3_1_1).eq(&Expr::constant(1)),
        (&dga_1_2_1_2 + &dga_1_3_1_2).eq(&Expr::constant(1)),
    ]);

    assert_eq!(one_interrogation_per_period_contraints, expected_result);
}

#[test]
fn one_interrogation_per_period_with_incomplete_period() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(3).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([0, 1, 2]),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let one_interrogation_per_period_contraints =
        ilp_translator.build_one_interrogation_per_period_contraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_1_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_1_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 2, group: 0 });

    #[rustfmt::skip]
    let dga_0_0_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_0_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_1_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_1_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_1_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_2_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_2_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_2_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 5 });

    #[rustfmt::skip]
    let dga_1_0_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_0_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_0_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 2 });
    #[rustfmt::skip]
    let dga_1_1_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_1_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_1_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 2 });
    #[rustfmt::skip]
    let dga_1_2_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 2, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_2_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 2, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_2_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 2, group: 1, student: 2 });

    #[rustfmt::skip]
    let snilp_0_0 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 0, student: 0 });
    #[rustfmt::skip]
    let snilp_0_1 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 0, student: 1 });
    #[rustfmt::skip]
    let snilp_0_2 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 0, student: 2 });
    #[rustfmt::skip]
    let snilp_0_3 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 0, student: 3 });
    #[rustfmt::skip]
    let snilp_0_4 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 0, student: 4 });
    #[rustfmt::skip]
    let snilp_0_5 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 0, student: 5 });

    #[rustfmt::skip]
    let snilp_1_0 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 0 });
    #[rustfmt::skip]
    let snilp_1_1 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 1 });
    #[rustfmt::skip]
    let snilp_1_2 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 2 });
    #[rustfmt::skip]
    let snilp_1_3 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 3 });
    #[rustfmt::skip]
    let snilp_1_4 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 4 });
    #[rustfmt::skip]
    let snilp_1_5 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 5 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0).eq(&Expr::constant(1)),
        (&dga_0_0_1_3 + &dga_0_1_1_3).eq(&Expr::constant(1)),
        (&dga_0_0_1_4 + &dga_0_1_1_4).eq(&Expr::constant(1)),
        (&dga_0_0_1_5 + &dga_0_1_1_5).eq(&Expr::constant(1)),

        (&gis_0_2_0 + &snilp_0_0).eq(&Expr::constant(1)),
        (&gis_0_2_0 + &snilp_0_1).eq(&Expr::constant(1)),
        (&gis_0_2_0 + &snilp_0_2).eq(&Expr::constant(1)),
        (&dga_0_2_1_3 + &snilp_0_3).eq(&Expr::constant(1)),
        (&dga_0_2_1_4 + &snilp_0_4).eq(&Expr::constant(1)),
        (&dga_0_2_1_5 + &snilp_0_5).eq(&Expr::constant(1)),

        (&gis_1_0_0 + &gis_1_1_0).eq(&Expr::constant(1)),
        (&dga_1_0_1_0 + &dga_1_1_1_0).eq(&Expr::constant(1)),
        (&dga_1_0_1_1 + &dga_1_1_1_1).eq(&Expr::constant(1)),
        (&dga_1_0_1_2 + &dga_1_1_1_2).eq(&Expr::constant(1)),

        (&gis_1_2_0 + &snilp_1_3).eq(&Expr::constant(1)),
        (&gis_1_2_0 + &snilp_1_4).eq(&Expr::constant(1)),
        (&gis_1_2_0 + &snilp_1_5).eq(&Expr::constant(1)),
        (&dga_1_2_1_0 + &snilp_1_0).eq(&Expr::constant(1)),
        (&dga_1_2_1_1 + &snilp_1_1).eq(&Expr::constant(1)),
        (&dga_1_2_1_2 + &snilp_1_2).eq(&Expr::constant(1)),
    ]);

    assert_eq!(one_interrogation_per_period_contraints, expected_result);
}

#[test]
fn students_per_group_count() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::from([4, 5]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let students_per_group_count_constraints =
        ilp_translator.build_students_per_group_count_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let sig_0_6_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 6, group: 1 });
    #[rustfmt::skip]
    let sig_0_7_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 7, group: 1 });
    #[rustfmt::skip]
    let sig_0_8_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 8, group: 1 });
    #[rustfmt::skip]
    let sig_0_9_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 9, group: 1 });
    #[rustfmt::skip]
    let sig_0_10_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 10, group: 1 });
    #[rustfmt::skip]
    let sig_0_11_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 11, group: 1 });

    #[rustfmt::skip]
    let sig_0_6_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 6, group: 2 });
    #[rustfmt::skip]
    let sig_0_7_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 7, group: 2 });
    #[rustfmt::skip]
    let sig_0_8_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 8, group: 2 });
    #[rustfmt::skip]
    let sig_0_9_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 9, group: 2 });
    #[rustfmt::skip]
    let sig_0_10_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 10, group: 2 });
    #[rustfmt::skip]
    let sig_0_11_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 11, group: 2 });

    #[rustfmt::skip]
    let sig_0_6_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 6, group: 3 });
    #[rustfmt::skip]
    let sig_0_7_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 7, group: 3 });
    #[rustfmt::skip]
    let sig_0_8_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 8, group: 3 });
    #[rustfmt::skip]
    let sig_0_9_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 9, group: 3 });
    #[rustfmt::skip]
    let sig_0_10_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 10, group: 3 });
    #[rustfmt::skip]
    let sig_0_11_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 11, group: 3 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&sig_0_6_1 + &sig_0_7_1 + &sig_0_8_1 + &sig_0_9_1 + &sig_0_10_1 + &sig_0_11_1).leq(&Expr::constant(2)),
        (&sig_0_6_1 + &sig_0_7_1 + &sig_0_8_1 + &sig_0_9_1 + &sig_0_10_1 + &sig_0_11_1).geq(&Expr::constant(1)),

        (&sig_0_6_2 + &sig_0_7_2 + &sig_0_8_2 + &sig_0_9_2 + &sig_0_10_2 + &sig_0_11_2).leq(&Expr::constant(1)),

        (&sig_0_6_3 + &sig_0_7_3 + &sig_0_8_3 + &sig_0_9_3 + &sig_0_10_3 + &sig_0_11_3).leq(&Expr::constant(3)),
        (&sig_0_6_3 + &sig_0_7_3 + &sig_0_8_3 + &sig_0_9_3 + &sig_0_10_3 + &sig_0_11_3).geq(&Expr::constant(2)),
    ]);

    assert_eq!(students_per_group_count_constraints, expected_result);
}

#[test]
fn student_in_single_group() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::from([4, 5]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([6, 7, 8, 9, 10, 11]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let student_in_single_group_constraints =
        ilp_translator.build_student_in_single_group_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let sig_0_6_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 6, group: 1 });
    #[rustfmt::skip]
    let sig_0_7_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 7, group: 1 });
    #[rustfmt::skip]
    let sig_0_8_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 8, group: 1 });
    #[rustfmt::skip]
    let sig_0_9_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 9, group: 1 });
    #[rustfmt::skip]
    let sig_0_10_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 10, group: 1 });
    #[rustfmt::skip]
    let sig_0_11_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 11, group: 1 });

    #[rustfmt::skip]
    let sig_0_6_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 6, group: 2 });
    #[rustfmt::skip]
    let sig_0_7_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 7, group: 2 });
    #[rustfmt::skip]
    let sig_0_8_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 8, group: 2 });
    #[rustfmt::skip]
    let sig_0_9_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 9, group: 2 });
    #[rustfmt::skip]
    let sig_0_10_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 10, group: 2 });
    #[rustfmt::skip]
    let sig_0_11_2 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 11, group: 2 });

    #[rustfmt::skip]
    let sig_0_6_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 6, group: 3 });
    #[rustfmt::skip]
    let sig_0_7_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 7, group: 3 });
    #[rustfmt::skip]
    let sig_0_8_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 8, group: 3 });
    #[rustfmt::skip]
    let sig_0_9_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 9, group: 3 });
    #[rustfmt::skip]
    let sig_0_10_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 10, group: 3 });
    #[rustfmt::skip]
    let sig_0_11_3 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 11, group: 3 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&sig_0_6_1 + &sig_0_6_2 + &sig_0_6_3).eq(&Expr::constant(1)),
        (&sig_0_7_1 + &sig_0_7_2 + &sig_0_7_3).eq(&Expr::constant(1)),
        (&sig_0_8_1 + &sig_0_8_2 + &sig_0_8_3).eq(&Expr::constant(1)),
        (&sig_0_9_1 + &sig_0_9_2 + &sig_0_9_3).eq(&Expr::constant(1)),
        (&sig_0_10_1 + &sig_0_10_2 + &sig_0_10_3).eq(&Expr::constant(1)),
        (&sig_0_11_1 + &sig_0_11_2 + &sig_0_11_3).eq(&Expr::constant(1)),
    ]);

    assert_eq!(student_in_single_group_constraints, expected_result);
}

#[test]
fn dynamic_groups_student_in_group_inequalities() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let dynamic_groups_student_in_group_constraints =
        ilp_translator.build_dynamic_groups_student_in_group_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let sig_0_3_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 3, group: 1 });
    #[rustfmt::skip]
    let sig_0_4_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 4, group: 1 });
    #[rustfmt::skip]
    let sig_0_5_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 5, group: 1 });

    #[rustfmt::skip]
    let dga_0_0_1_3 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_0_1_4 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_1_5 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_1_1_3 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_1_1_4 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_1_1_5 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 5 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        dga_0_0_1_3.leq(&sig_0_3_1),
        dga_0_1_1_3.leq(&sig_0_3_1),

        dga_0_0_1_4.leq(&sig_0_4_1),
        dga_0_1_1_4.leq(&sig_0_4_1),

        dga_0_0_1_5.leq(&sig_0_5_1),
        dga_0_1_1_5.leq(&sig_0_5_1),
    ]);

    assert_eq!(dynamic_groups_student_in_group_constraints, expected_result);
}

#[test]
fn dynamic_groups_group_in_slot_inequalities() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let dynamic_groups_group_in_slot_constraints =
        ilp_translator.build_dynamic_groups_group_in_slot_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });

    #[rustfmt::skip]
    let dga_0_0_1_3 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_0_1_4 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_1_5 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_1_1_3 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_1_1_4 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_1_1_5 = Expr::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 5 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        dga_0_0_1_3.leq(&gis_0_0_1),
        dga_0_0_1_4.leq(&gis_0_0_1),
        dga_0_0_1_5.leq(&gis_0_0_1),

        dga_0_1_1_3.leq(&gis_0_1_1),
        dga_0_1_1_4.leq(&gis_0_1_1),
        dga_0_1_1_5.leq(&gis_0_1_1),
    ]);

    assert_eq!(dynamic_groups_group_in_slot_constraints, expected_result);
}

#[test]
fn one_periodicity_choice_per_student() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(3).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(3).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(3).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let one_periodicity_choice_per_student_constraints =
        ilp_translator.build_one_periodicity_choice_per_student_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let p_0_0_0 = Expr::var(Variable::Periodicity { subject: 0, student: 0, week_modulo: 0 });
    #[rustfmt::skip]
    let p_0_1_0 = Expr::var(Variable::Periodicity { subject: 0, student: 1, week_modulo: 0 });
    #[rustfmt::skip]
    let p_0_2_0 = Expr::var(Variable::Periodicity { subject: 0, student: 2, week_modulo: 0 });
    #[rustfmt::skip]
    let p_0_3_0 = Expr::var(Variable::Periodicity { subject: 0, student: 3, week_modulo: 0 });
    #[rustfmt::skip]
    let p_0_4_0 = Expr::var(Variable::Periodicity { subject: 0, student: 4, week_modulo: 0 });
    #[rustfmt::skip]
    let p_0_5_0 = Expr::var(Variable::Periodicity { subject: 0, student: 5, week_modulo: 0 });

    #[rustfmt::skip]
    let p_0_0_1 = Expr::var(Variable::Periodicity { subject: 0, student: 0, week_modulo: 1 });
    #[rustfmt::skip]
    let p_0_1_1 = Expr::var(Variable::Periodicity { subject: 0, student: 1, week_modulo: 1 });
    #[rustfmt::skip]
    let p_0_2_1 = Expr::var(Variable::Periodicity { subject: 0, student: 2, week_modulo: 1 });
    #[rustfmt::skip]
    let p_0_3_1 = Expr::var(Variable::Periodicity { subject: 0, student: 3, week_modulo: 1 });
    #[rustfmt::skip]
    let p_0_4_1 = Expr::var(Variable::Periodicity { subject: 0, student: 4, week_modulo: 1 });
    #[rustfmt::skip]
    let p_0_5_1 = Expr::var(Variable::Periodicity { subject: 0, student: 5, week_modulo: 1 });

    #[rustfmt::skip]
    let p_0_0_2 = Expr::var(Variable::Periodicity { subject: 0, student: 0, week_modulo: 2 });
    #[rustfmt::skip]
    let p_0_1_2 = Expr::var(Variable::Periodicity { subject: 0, student: 1, week_modulo: 2 });
    #[rustfmt::skip]
    let p_0_2_2 = Expr::var(Variable::Periodicity { subject: 0, student: 2, week_modulo: 2 });
    #[rustfmt::skip]
    let p_0_3_2 = Expr::var(Variable::Periodicity { subject: 0, student: 3, week_modulo: 2 });
    #[rustfmt::skip]
    let p_0_4_2 = Expr::var(Variable::Periodicity { subject: 0, student: 4, week_modulo: 2 });
    #[rustfmt::skip]
    let p_0_5_2 = Expr::var(Variable::Periodicity { subject: 0, student: 5, week_modulo: 2 });

    #[rustfmt::skip]
    let p_1_0_0 = Expr::var(Variable::Periodicity { subject: 1, student: 0, week_modulo: 0 });
    #[rustfmt::skip]
    let p_1_1_0 = Expr::var(Variable::Periodicity { subject: 1, student: 1, week_modulo: 0 });
    #[rustfmt::skip]
    let p_1_2_0 = Expr::var(Variable::Periodicity { subject: 1, student: 2, week_modulo: 0 });
    #[rustfmt::skip]
    let p_1_3_0 = Expr::var(Variable::Periodicity { subject: 1, student: 3, week_modulo: 0 });
    #[rustfmt::skip]
    let p_1_4_0 = Expr::var(Variable::Periodicity { subject: 1, student: 4, week_modulo: 0 });
    #[rustfmt::skip]
    let p_1_5_0 = Expr::var(Variable::Periodicity { subject: 1, student: 5, week_modulo: 0 });

    #[rustfmt::skip]
    let p_1_0_1 = Expr::var(Variable::Periodicity { subject: 1, student: 0, week_modulo: 1 });
    #[rustfmt::skip]
    let p_1_1_1 = Expr::var(Variable::Periodicity { subject: 1, student: 1, week_modulo: 1 });
    #[rustfmt::skip]
    let p_1_2_1 = Expr::var(Variable::Periodicity { subject: 1, student: 2, week_modulo: 1 });
    #[rustfmt::skip]
    let p_1_3_1 = Expr::var(Variable::Periodicity { subject: 1, student: 3, week_modulo: 1 });
    #[rustfmt::skip]
    let p_1_4_1 = Expr::var(Variable::Periodicity { subject: 1, student: 4, week_modulo: 1 });
    #[rustfmt::skip]
    let p_1_5_1 = Expr::var(Variable::Periodicity { subject: 1, student: 5, week_modulo: 1 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&p_0_0_0 + &p_0_0_1 + &p_0_0_2).eq(&Expr::constant(1)),
        (&p_0_1_0 + &p_0_1_1 + &p_0_1_2).eq(&Expr::constant(1)),
        (&p_0_2_0 + &p_0_2_1 + &p_0_2_2).eq(&Expr::constant(1)),
        (&p_0_3_0 + &p_0_3_1 + &p_0_3_2).eq(&Expr::constant(1)),
        (&p_0_4_0 + &p_0_4_1 + &p_0_4_2).eq(&Expr::constant(1)),
        (&p_0_5_0 + &p_0_5_1 + &p_0_5_2).eq(&Expr::constant(1)),

        (&p_1_0_0 + &p_1_0_1).eq(&Expr::constant(1)),
        (&p_1_1_0 + &p_1_1_1).eq(&Expr::constant(1)),
        (&p_1_2_0 + &p_1_2_1).eq(&Expr::constant(1)),
        (&p_1_3_0 + &p_1_3_1).eq(&Expr::constant(1)),
        (&p_1_4_0 + &p_1_4_1).eq(&Expr::constant(1)),
        (&p_1_5_0 + &p_1_5_1).eq(&Expr::constant(1)),
    ]);

    assert_eq!(
        one_periodicity_choice_per_student_constraints,
        expected_result
    );
}

#[test]
fn periodicity_inequalities() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(3).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(3).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(3).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let periodicity_constraints = ilp_translator.build_periodicity_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let p_0_0_0 = Expr::var(Variable::Periodicity { subject: 0, student: 0, week_modulo: 0 });
    #[rustfmt::skip]
    let p_0_1_0 = Expr::var(Variable::Periodicity { subject: 0, student: 1, week_modulo: 0 });
    #[rustfmt::skip]
    let p_0_2_0 = Expr::var(Variable::Periodicity { subject: 0, student: 2, week_modulo: 0 });
    #[rustfmt::skip]
    let p_0_3_0 = Expr::var(Variable::Periodicity { subject: 0, student: 3, week_modulo: 0 });
    #[rustfmt::skip]
    let p_0_4_0 = Expr::var(Variable::Periodicity { subject: 0, student: 4, week_modulo: 0 });
    #[rustfmt::skip]
    let p_0_5_0 = Expr::var(Variable::Periodicity { subject: 0, student: 5, week_modulo: 0 });

    #[rustfmt::skip]
    let p_0_0_1 = Expr::var(Variable::Periodicity { subject: 0, student: 0, week_modulo: 1 });
    #[rustfmt::skip]
    let p_0_1_1 = Expr::var(Variable::Periodicity { subject: 0, student: 1, week_modulo: 1 });
    #[rustfmt::skip]
    let p_0_2_1 = Expr::var(Variable::Periodicity { subject: 0, student: 2, week_modulo: 1 });
    #[rustfmt::skip]
    let p_0_3_1 = Expr::var(Variable::Periodicity { subject: 0, student: 3, week_modulo: 1 });
    #[rustfmt::skip]
    let p_0_4_1 = Expr::var(Variable::Periodicity { subject: 0, student: 4, week_modulo: 1 });
    #[rustfmt::skip]
    let p_0_5_1 = Expr::var(Variable::Periodicity { subject: 0, student: 5, week_modulo: 1 });

    #[rustfmt::skip]
    let p_0_0_2 = Expr::var(Variable::Periodicity { subject: 0, student: 0, week_modulo: 2 });
    #[rustfmt::skip]
    let p_0_1_2 = Expr::var(Variable::Periodicity { subject: 0, student: 1, week_modulo: 2 });
    #[rustfmt::skip]
    let p_0_2_2 = Expr::var(Variable::Periodicity { subject: 0, student: 2, week_modulo: 2 });
    #[rustfmt::skip]
    let p_0_3_2 = Expr::var(Variable::Periodicity { subject: 0, student: 3, week_modulo: 2 });
    #[rustfmt::skip]
    let p_0_4_2 = Expr::var(Variable::Periodicity { subject: 0, student: 4, week_modulo: 2 });
    #[rustfmt::skip]
    let p_0_5_2 = Expr::var(Variable::Periodicity { subject: 0, student: 5, week_modulo: 2 });

    #[rustfmt::skip]
    let p_1_0_0 = Expr::var(Variable::Periodicity { subject: 1, student: 0, week_modulo: 0 });
    #[rustfmt::skip]
    let p_1_1_0 = Expr::var(Variable::Periodicity { subject: 1, student: 1, week_modulo: 0 });
    #[rustfmt::skip]
    let p_1_2_0 = Expr::var(Variable::Periodicity { subject: 1, student: 2, week_modulo: 0 });
    #[rustfmt::skip]
    let p_1_3_0 = Expr::var(Variable::Periodicity { subject: 1, student: 3, week_modulo: 0 });
    #[rustfmt::skip]
    let p_1_4_0 = Expr::var(Variable::Periodicity { subject: 1, student: 4, week_modulo: 0 });
    #[rustfmt::skip]
    let p_1_5_0 = Expr::var(Variable::Periodicity { subject: 1, student: 5, week_modulo: 0 });

    #[rustfmt::skip]
    let p_1_0_1 = Expr::var(Variable::Periodicity { subject: 1, student: 0, week_modulo: 1 });
    #[rustfmt::skip]
    let p_1_1_1 = Expr::var(Variable::Periodicity { subject: 1, student: 1, week_modulo: 1 });
    #[rustfmt::skip]
    let p_1_2_1 = Expr::var(Variable::Periodicity { subject: 1, student: 2, week_modulo: 1 });
    #[rustfmt::skip]
    let p_1_3_1 = Expr::var(Variable::Periodicity { subject: 1, student: 3, week_modulo: 1 });
    #[rustfmt::skip]
    let p_1_4_1 = Expr::var(Variable::Periodicity { subject: 1, student: 4, week_modulo: 1 });
    #[rustfmt::skip]
    let p_1_5_1 = Expr::var(Variable::Periodicity { subject: 1, student: 5, week_modulo: 1 });

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let dga_0_0_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_0_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_1_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_1_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_1_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_2_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_2_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_2_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 5 });

    #[rustfmt::skip]
    let gis_1_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_1_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 2, group: 0 });
    #[rustfmt::skip]
    let dga_1_0_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_1_0_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_1_0_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_1_1_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_1_1_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_1_1_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_1_2_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 2, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_1_2_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 2, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_1_2_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 2, group: 1, student: 5 });
    #[rustfmt::skip]
    let snilp_1_0 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 0 });
    #[rustfmt::skip]
    let snilp_1_1 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 1 });
    #[rustfmt::skip]
    let snilp_1_2 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 2 });
    #[rustfmt::skip]
    let snilp_1_3 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 3 });
    #[rustfmt::skip]
    let snilp_1_4 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 4 });
    #[rustfmt::skip]
    let snilp_1_5 = Expr::<Variable>::var(Variable::StudentNotInLastPeriod { subject: 1, student: 5 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        gis_0_0_0.leq(&p_0_0_0),
        gis_0_0_0.leq(&p_0_1_0),
        gis_0_0_0.leq(&p_0_2_0),

        gis_0_1_0.leq(&p_0_0_1),
        gis_0_1_0.leq(&p_0_1_1),
        gis_0_1_0.leq(&p_0_2_1),

        gis_0_2_0.leq(&p_0_0_2),
        gis_0_2_0.leq(&p_0_1_2),
        gis_0_2_0.leq(&p_0_2_2),

        dga_0_0_1_3.leq(&p_0_3_0),
        dga_0_0_1_4.leq(&p_0_4_0),
        dga_0_0_1_5.leq(&p_0_5_0),

        dga_0_1_1_3.leq(&p_0_3_1),
        dga_0_1_1_4.leq(&p_0_4_1),
        dga_0_1_1_5.leq(&p_0_5_1),

        dga_0_2_1_3.leq(&p_0_3_2),
        dga_0_2_1_4.leq(&p_0_4_2),
        dga_0_2_1_5.leq(&p_0_5_2),

        gis_1_0_0.leq(&p_1_0_0),
        gis_1_0_0.leq(&p_1_1_0),
        gis_1_0_0.leq(&p_1_2_0),

        gis_1_1_0.leq(&p_1_0_1),
        gis_1_1_0.leq(&p_1_1_1),
        gis_1_1_0.leq(&p_1_2_1),

        gis_1_2_0.leq(&p_1_0_0),
        gis_1_2_0.leq(&p_1_1_0),
        gis_1_2_0.leq(&p_1_2_0),

        snilp_1_0.leq(&p_1_0_1),
        snilp_1_1.leq(&p_1_1_1),
        snilp_1_2.leq(&p_1_2_1),

        dga_1_0_1_3.leq(&p_1_3_0),
        dga_1_0_1_4.leq(&p_1_4_0),
        dga_1_0_1_5.leq(&p_1_5_0),

        dga_1_1_1_3.leq(&p_1_3_1),
        dga_1_1_1_4.leq(&p_1_4_1),
        dga_1_1_1_5.leq(&p_1_5_1),

        dga_1_2_1_3.leq(&p_1_3_0),
        dga_1_2_1_4.leq(&p_1_4_0),
        dga_1_2_1_5.leq(&p_1_5_0),

        snilp_1_3.leq(&p_1_3_1),
        snilp_1_4.leq(&p_1_4_1),
        snilp_1_5.leq(&p_1_5_1),
    ]);

    assert_eq!(periodicity_constraints, expected_result);
}

#[test]
fn interrogations_per_week() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: Some(1..3),
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([0, 1, 2]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(8).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            is_tutorial: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Friday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Friday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([0, 1, 2]),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let interrogations_per_week_constraints =
        ilp_translator.build_interrogations_per_week_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_1_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 0 });

    #[rustfmt::skip]
    let dga_0_0_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_0_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_1_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_1_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_1_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 5 });

    #[rustfmt::skip]
    let dga_1_0_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_0_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_0_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 2 });
    #[rustfmt::skip]
    let dga_1_1_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_1_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_1_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &dga_1_0_1_0).leq(&Expr::constant(2)),
        (&gis_0_0_0 + &dga_1_0_1_0).geq(&Expr::constant(1)),
        (&gis_0_0_0 + &dga_1_0_1_1).leq(&Expr::constant(2)),
        (&gis_0_0_0 + &dga_1_0_1_1).geq(&Expr::constant(1)),
        (&gis_0_0_0 + &dga_1_0_1_2).leq(&Expr::constant(2)),
        (&gis_0_0_0 + &dga_1_0_1_2).geq(&Expr::constant(1)),

        (&gis_0_1_0 + &dga_1_1_1_0).leq(&Expr::constant(2)),
        (&gis_0_1_0 + &dga_1_1_1_0).geq(&Expr::constant(1)),
        (&gis_0_1_0 + &dga_1_1_1_1).leq(&Expr::constant(2)),
        (&gis_0_1_0 + &dga_1_1_1_1).geq(&Expr::constant(1)),
        (&gis_0_1_0 + &dga_1_1_1_2).leq(&Expr::constant(2)),
        (&gis_0_1_0 + &dga_1_1_1_2).geq(&Expr::constant(1)),

        (&dga_0_0_1_3 + &gis_1_0_0).leq(&Expr::constant(2)),
        (&dga_0_0_1_3 + &gis_1_0_0).geq(&Expr::constant(1)),
        (&dga_0_0_1_4 + &gis_1_0_0).leq(&Expr::constant(2)),
        (&dga_0_0_1_4 + &gis_1_0_0).geq(&Expr::constant(1)),
        (&dga_0_0_1_5 + &gis_1_0_0).leq(&Expr::constant(2)),
        (&dga_0_0_1_5 + &gis_1_0_0).geq(&Expr::constant(1)),

        (&dga_0_1_1_3 + &gis_1_1_0).leq(&Expr::constant(2)),
        (&dga_0_1_1_3 + &gis_1_1_0).geq(&Expr::constant(1)),
        (&dga_0_1_1_4 + &gis_1_1_0).leq(&Expr::constant(2)),
        (&dga_0_1_1_4 + &gis_1_1_0).geq(&Expr::constant(1)),
        (&dga_0_1_1_5 + &gis_1_1_0).leq(&Expr::constant(2)),
        (&dga_0_1_1_5 + &gis_1_1_0).geq(&Expr::constant(1)),
    ]);

    assert_eq!(interrogations_per_week_constraints, expected_result);
}

#[test]
fn grouping_inequalities() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
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
    let slot_groupings = vec![
        SlotGrouping {
            slots: BTreeSet::from([
                SlotRef {
                    subject: 0,
                    slot: 1,
                },
                SlotRef {
                    subject: 0,
                    slot: 2,
                },
            ]),
        },
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 0,
                slot: 3,
            }]),
        },
    ];
    let grouping_incompats = SlotGroupingIncompatSet::from([SlotGroupingIncompat {
        groupings: BTreeSet::from([0, 1]),
        max_count: NonZeroUsize::new(1).unwrap(),
    }]);

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let grouping_constraints = ilp_translator.build_grouping_constraints();

    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_0_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 0 });

    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });
    #[rustfmt::skip]
    let gis_0_2_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 1 });
    #[rustfmt::skip]
    let gis_0_3_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 1 });

    #[rustfmt::skip]
    let ug_0 = Expr::<Variable>::var(Variable::UseGrouping(0));
    #[rustfmt::skip]
    let ug_1 = Expr::<Variable>::var(Variable::UseGrouping(1));

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        gis_0_1_0.leq(&ug_0),
        gis_0_1_1.leq(&ug_0),
        gis_0_2_0.leq(&ug_0),
        gis_0_2_1.leq(&ug_0),

        gis_0_3_0.leq(&ug_1),
        gis_0_3_1.leq(&ug_1),
    ]);

    assert_eq!(grouping_constraints, expected_result);
}

#[test]
fn grouping_incompats() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
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
    let slot_groupings = vec![
        SlotGrouping {
            slots: BTreeSet::from([
                SlotRef {
                    subject: 0,
                    slot: 1,
                },
                SlotRef {
                    subject: 0,
                    slot: 2,
                },
            ]),
        },
        SlotGrouping {
            slots: BTreeSet::from([SlotRef {
                subject: 0,
                slot: 3,
            }]),
        },
    ];
    let grouping_incompats = SlotGroupingIncompatSet::from([SlotGroupingIncompat {
        groupings: BTreeSet::from([0, 1]),
        max_count: NonZeroUsize::new(1).unwrap(),
    }]);

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let grouping_incompats_constraints = ilp_translator.build_grouping_incompats_constraints();

    #[rustfmt::skip]
    let ug_0 = Expr::<Variable>::var(Variable::UseGrouping(0));
    #[rustfmt::skip]
    let ug_1 = Expr::<Variable>::var(Variable::UseGrouping(1));

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&ug_0 + &ug_1).leq(&Expr::constant(1)),
    ]);

    assert_eq!(grouping_incompats_constraints, expected_result);
}

#[test]
fn students_incompats() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
    }];
    let incompatibilities = IncompatibilityList::from([
        Incompatibility {
            slots: vec![
                SlotWithDuration {
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Wednesday,
                        start_time: time::Time::from_hm(11, 0).unwrap(),
                    },
                    duration: NonZeroU32::new(120).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Thursday,
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
            ],
        },
        Incompatibility {
            slots: vec![SlotWithDuration {
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(11, 0).unwrap(),
                },
                duration: NonZeroU32::new(120).unwrap(),
            }],
        },
    ]);
    let students = vec![
        Student {
            incompatibilities: BTreeSet::from([0, 1]),
        },
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
        Student {
            incompatibilities: BTreeSet::from([1]),
        },
        Student {
            incompatibilities: BTreeSet::from([0, 1]),
        },
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
        Student {
            incompatibilities: BTreeSet::from([1]),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let students_incompats_constraints = ilp_translator.build_students_incompats_constraints();

    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });

    #[rustfmt::skip]
    let dga_0_2_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_2_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 4 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        gis_0_2_0.eq(&Expr::constant(0)),
        dga_0_2_1_3.eq(&Expr::constant(0)),
        dga_0_2_1_4.eq(&Expr::constant(0)),
    ]);

    assert_eq!(students_incompats_constraints, expected_result);
}

#[test]
fn slot_overlaps() {
    let slot1 = SlotWithDuration {
        start: SlotStart {
            week: 0,
            weekday: time::Weekday::Monday,
            start_time: time::Time::from_hm(12, 0).unwrap(),
        },
        duration: NonZeroU32::new(60).unwrap(),
    };

    let slot2 = SlotWithDuration {
        start: SlotStart {
            week: 0,
            weekday: time::Weekday::Monday,
            start_time: time::Time::from_hm(11, 30).unwrap(),
        },
        duration: NonZeroU32::new(60).unwrap(),
    };

    let slot3 = SlotWithDuration {
        start: SlotStart {
            week: 0,
            weekday: time::Weekday::Monday,
            start_time: time::Time::from_hm(12, 30).unwrap(),
        },
        duration: NonZeroU32::new(60).unwrap(),
    };

    let slot4 = SlotWithDuration {
        start: SlotStart {
            week: 0,
            weekday: time::Weekday::Monday,
            start_time: time::Time::from_hm(11, 0).unwrap(),
        },
        duration: NonZeroU32::new(60).unwrap(),
    };

    let slot5 = SlotWithDuration {
        start: SlotStart {
            week: 0,
            weekday: time::Weekday::Monday,
            start_time: time::Time::from_hm(13, 0).unwrap(),
        },
        duration: NonZeroU32::new(60).unwrap(),
    };

    let slot6 = SlotWithDuration {
        start: SlotStart {
            week: 0,
            weekday: time::Weekday::Tuesday,
            start_time: time::Time::from_hm(12, 0).unwrap(),
        },
        duration: NonZeroU32::new(60).unwrap(),
    };

    let slot7 = SlotWithDuration {
        start: SlotStart {
            week: 1,
            weekday: time::Weekday::Monday,
            start_time: time::Time::from_hm(12, 0).unwrap(),
        },
        duration: NonZeroU32::new(60).unwrap(),
    };

    assert_eq!(slot1.overlap_with(&slot2), true);
    assert_eq!(slot1.overlap_with(&slot3), true);
    assert_eq!(slot1.overlap_with(&slot4), false);
    assert_eq!(slot1.overlap_with(&slot5), false);
    assert_eq!(slot1.overlap_with(&slot6), false);
    assert_eq!(slot1.overlap_with(&slot7), false);
}

#[test]
fn simple_colloscope() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                ],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                ],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
    ];
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let problem = ilp_translator.problem();
    let constraints = problem.get_constraints().clone();

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });

    #[rustfmt::skip]
    let gis_1_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_1_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_1_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 1 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0).eq(&Expr::constant(1)),
        (&gis_0_0_1 + &gis_0_1_1).eq(&Expr::constant(1)),
        (&gis_1_0_0 + &gis_1_1_0).eq(&Expr::constant(1)),
        (&gis_1_0_1 + &gis_1_1_1).eq(&Expr::constant(1)),

        (&gis_0_0_0 + &gis_1_0_0).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &gis_1_1_0).leq(&Expr::constant(1)),
        (&gis_0_0_1 + &gis_1_0_1).leq(&Expr::constant(1)),
        (&gis_0_1_1 + &gis_1_1_1).leq(&Expr::constant(1)),

        (&gis_0_0_0 + &gis_0_0_1).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &gis_0_1_1).leq(&Expr::constant(1)),
        (&gis_1_0_0 + &gis_1_0_1).leq(&Expr::constant(1)),
        (&gis_1_1_0 + &gis_1_1_1).leq(&Expr::constant(1)),
    ]);

    assert_eq!(constraints, expected_result);
}

#[test]
fn colloscope_with_dynamic_groups() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1]),
                        can_be_extended: true,
                    },
                    GroupDesc {
                        students: BTreeSet::from([3]),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([2, 4, 5]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
    ];
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
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let problem = ilp_translator.problem_builder().build(); // Avoid problem() as it simplifies constraints
    let constraints = problem.get_constraints().clone();

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });

    #[rustfmt::skip]
    let gis_1_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_1_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_1_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 1 });

    #[rustfmt::skip]
    let sig_0_2_0 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 2, group: 0 });
    #[rustfmt::skip]
    let sig_0_4_0 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 4, group: 0 });
    #[rustfmt::skip]
    let sig_0_5_0 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 5, group: 0 });
    #[rustfmt::skip]
    let sig_0_2_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 2, group: 1 });
    #[rustfmt::skip]
    let sig_0_4_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 4, group: 1 });
    #[rustfmt::skip]
    let sig_0_5_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 0, student: 5, group: 1 });

    #[rustfmt::skip]
    let sig_1_3_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 1, student: 3, group: 1 });
    #[rustfmt::skip]
    let sig_1_4_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 1, student: 4, group: 1 });
    #[rustfmt::skip]
    let sig_1_5_1 = Expr::<Variable>::var(Variable::StudentInGroup { subject: 1, student: 5, group: 1 });

    #[rustfmt::skip]
    let dga_0_0_0_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 0, student: 2 });
    #[rustfmt::skip]
    let dga_0_0_0_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 0, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_0_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 0, student: 5 });
    #[rustfmt::skip]
    let dga_0_1_0_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 0, student: 2 });
    #[rustfmt::skip]
    let dga_0_1_0_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 0, student: 4 });
    #[rustfmt::skip]
    let dga_0_1_0_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 0, student: 5 });

    #[rustfmt::skip]
    let dga_0_0_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 2 });
    #[rustfmt::skip]
    let dga_0_0_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_1_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 2 });
    #[rustfmt::skip]
    let dga_0_1_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_1_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 5 });

    #[rustfmt::skip]
    let dga_1_0_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_1_0_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_1_0_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_1_1_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_1_1_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_1_1_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 5 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        // One interrogation per period for groups with fixed students
        (&gis_0_0_0 + &gis_0_1_0).eq(&Expr::constant(1)),
        (&gis_0_0_1 + &gis_0_1_1).eq(&Expr::constant(1)),
        (&gis_1_0_0 + &gis_1_1_0).eq(&Expr::constant(1)),

        // One interrogation per period for dynamic students
        (&dga_0_0_0_2 + &dga_0_0_1_2 + &dga_0_1_0_2 + &dga_0_1_1_2).eq(&Expr::constant(1)),
        (&dga_0_0_0_4 + &dga_0_0_1_4 + &dga_0_1_0_4 + &dga_0_1_1_4).eq(&Expr::constant(1)),
        (&dga_0_0_0_5 + &dga_0_0_1_5 + &dga_0_1_0_5 + &dga_0_1_1_5).eq(&Expr::constant(1)),

        (&dga_1_0_1_3 + &dga_1_1_1_3).eq(&Expr::constant(1)),
        (&dga_1_0_1_4 + &dga_1_1_1_4).eq(&Expr::constant(1)),
        (&dga_1_0_1_5 + &dga_1_1_1_5).eq(&Expr::constant(1)),

        // At most one interrogation per period for groups without fixed students
        (&gis_1_0_1 + &gis_1_1_1).leq(&Expr::constant(1)),

        // One interrogation at one time maximum for a given student
        (&gis_0_0_0 + &gis_1_0_0).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &gis_1_1_0).leq(&Expr::constant(1)),
        (&dga_0_0_0_2 + &dga_0_0_1_2 + &gis_1_0_0).leq(&Expr::constant(1)),
        (&dga_0_1_0_2 + &dga_0_1_1_2 + &gis_1_1_0).leq(&Expr::constant(1)),
        (&gis_0_0_1 + &dga_1_0_1_3).leq(&Expr::constant(1)),
        (&gis_0_1_1 + &dga_1_1_1_3).leq(&Expr::constant(1)),
        (&dga_0_0_0_4 + &dga_0_0_1_4 + &dga_1_0_1_4).leq(&Expr::constant(1)),
        (&dga_0_1_0_4 + &dga_0_1_1_4 + &dga_1_1_1_4).leq(&Expr::constant(1)),
        (&dga_0_0_0_5 + &dga_0_0_1_5 + &dga_1_0_1_5).leq(&Expr::constant(1)),
        (&dga_0_1_0_5 + &dga_0_1_1_5 + &dga_1_1_1_5).leq(&Expr::constant(1)),

        // One group per interrogation maximum
        (&gis_0_0_0 + &gis_0_0_1).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &gis_0_1_1).leq(&Expr::constant(1)),
        (&gis_1_0_0 + &gis_1_0_1).leq(&Expr::constant(1)),
        (&gis_1_1_0 + &gis_1_1_1).leq(&Expr::constant(1)),

        // Dynamic students in exactly one group
        (&sig_0_2_0 + &sig_0_2_1).eq(&Expr::constant(1)),
        (&sig_0_4_0 + &sig_0_4_1).eq(&Expr::constant(1)),
        (&sig_0_5_0 + &sig_0_5_1).eq(&Expr::constant(1)),

        sig_1_3_1.eq(&Expr::constant(1)),
        sig_1_4_1.eq(&Expr::constant(1)),
        sig_1_5_1.eq(&Expr::constant(1)),

        // Number of students per group (for dynamic students) 
        (&sig_0_2_0 + &sig_0_4_0 + &sig_0_5_0).leq(&Expr::constant(1)),
        (&sig_0_2_1 + &sig_0_4_1 + &sig_0_5_1).leq(&Expr::constant(2)),
        (&sig_0_2_1 + &sig_0_4_1 + &sig_0_5_1).geq(&Expr::constant(1)),

        (&sig_1_3_1 + &sig_1_4_1 + &sig_1_5_1).leq(&Expr::constant(3)),
        (&sig_1_3_1 + &sig_1_4_1 + &sig_1_5_1).geq(&Expr::constant(2)),

        // Dynamic group assignement only if group in correct slot
        dga_0_0_0_2.leq(&gis_0_0_0),
        dga_0_0_0_4.leq(&gis_0_0_0),
        dga_0_0_0_5.leq(&gis_0_0_0),
        dga_0_1_0_2.leq(&gis_0_1_0),
        dga_0_1_0_4.leq(&gis_0_1_0),
        dga_0_1_0_5.leq(&gis_0_1_0),

        dga_0_0_1_2.leq(&gis_0_0_1),
        dga_0_0_1_4.leq(&gis_0_0_1),
        dga_0_0_1_5.leq(&gis_0_0_1),
        dga_0_1_1_2.leq(&gis_0_1_1),
        dga_0_1_1_4.leq(&gis_0_1_1),
        dga_0_1_1_5.leq(&gis_0_1_1),

        dga_1_0_1_3.leq(&gis_1_0_1),
        dga_1_0_1_4.leq(&gis_1_0_1),
        dga_1_0_1_5.leq(&gis_1_0_1),
        dga_1_1_1_3.leq(&gis_1_1_1),
        dga_1_1_1_4.leq(&gis_1_1_1),
        dga_1_1_1_5.leq(&gis_1_1_1),

        // Dynamic group assignement only if student in correct group
        dga_0_0_0_2.leq(&sig_0_2_0),
        dga_0_0_0_4.leq(&sig_0_4_0),
        dga_0_0_0_5.leq(&sig_0_5_0),
        dga_0_1_0_2.leq(&sig_0_2_0),
        dga_0_1_0_4.leq(&sig_0_4_0),
        dga_0_1_0_5.leq(&sig_0_5_0),

        dga_0_0_1_2.leq(&sig_0_2_1),
        dga_0_0_1_4.leq(&sig_0_4_1),
        dga_0_0_1_5.leq(&sig_0_5_1),
        dga_0_1_1_2.leq(&sig_0_2_1),
        dga_0_1_1_4.leq(&sig_0_4_1),
        dga_0_1_1_5.leq(&sig_0_5_1),

        dga_1_0_1_3.leq(&sig_1_3_1),
        dga_1_0_1_4.leq(&sig_1_4_1),
        dga_1_0_1_5.leq(&sig_1_5_1),
        dga_1_1_1_3.leq(&sig_1_3_1),
        dga_1_1_1_4.leq(&sig_1_4_1),
        dga_1_1_1_5.leq(&sig_1_5_1),
    ]);

    assert_eq!(constraints, expected_result);
}

#[test]
fn at_most_one_interrogation_per_empty_group() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let at_most_one_interrogation_per_period_for_empty_groups_contraints =
        ilp_translator.build_at_most_one_interrogation_per_period_for_empty_groups_contraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_1 + &gis_0_1_1).leq(&Expr::constant(1)),
    ]);

    assert_eq!(
        at_most_one_interrogation_per_period_for_empty_groups_contraints,
        expected_result
    );
}

#[test]
fn max_interrogations_per_day() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: Some(NonZeroU32::new(1).unwrap()),
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([0, 1, 2]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([3, 4, 5]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([0, 1, 2]),
            },
            ..Subject::default()
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(8).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: true,
            is_tutorial: true,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Friday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Friday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![
                    GroupDesc {
                        students: BTreeSet::from([3, 4, 5]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::new(),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([0, 1, 2]),
            },
            ..Subject::default()
        },
    ];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let interrogations_per_week_constraints =
        ilp_translator.build_max_interrogations_per_day_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_1_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 0 });

    #[rustfmt::skip]
    let dga_0_0_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_0_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_1_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_1_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_1_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 1, group: 1, student: 5 });

    #[rustfmt::skip]
    let dga_1_0_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_0_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_0_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 0, group: 1, student: 2 });
    #[rustfmt::skip]
    let dga_1_1_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_1_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_1_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 1, group: 1, student: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &dga_1_0_1_0).leq(&Expr::constant(1)),
        (&gis_0_0_0 + &dga_1_0_1_1).leq(&Expr::constant(1)),
        (&gis_0_0_0 + &dga_1_0_1_2).leq(&Expr::constant(1)),

        (&gis_0_1_0 + &dga_1_1_1_0).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &dga_1_1_1_1).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &dga_1_1_1_2).leq(&Expr::constant(1)),

        (&dga_0_0_1_3 + &gis_1_0_0).leq(&Expr::constant(1)),
        (&dga_0_0_1_4 + &gis_1_0_0).leq(&Expr::constant(1)),
        (&dga_0_0_1_5 + &gis_1_0_0).leq(&Expr::constant(1)),
        
        (&dga_0_1_1_3 + &gis_1_1_0).leq(&Expr::constant(1)),
        (&dga_0_1_1_4 + &gis_1_1_0).leq(&Expr::constant(1)),
        (&dga_0_1_1_5 + &gis_1_1_0).leq(&Expr::constant(1)),
    ]);

    assert_eq!(interrogations_per_week_constraints, expected_result);
}

#[test]
fn balancing_teachers() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            teachers: true,
            timeslots: false,
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([5, 6, 7, 8]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let balancing_constraints = ilp_translator.build_balancing_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_0_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 0 });
    #[rustfmt::skip]
    let gis_0_4_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 0 });
    #[rustfmt::skip]
    let gis_0_5_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 0 });
    #[rustfmt::skip]
    let gis_0_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });
    #[rustfmt::skip]
    let gis_0_2_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 1 });
    #[rustfmt::skip]
    let gis_0_3_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 1 });
    #[rustfmt::skip]
    let gis_0_4_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 1 });
    #[rustfmt::skip]
    let gis_0_5_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 1 });
    #[rustfmt::skip]
    let gis_0_0_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 2 });
    #[rustfmt::skip]
    let gis_0_1_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 2 });
    #[rustfmt::skip]
    let gis_0_2_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 2 });
    #[rustfmt::skip]
    let gis_0_3_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 2 });
    #[rustfmt::skip]
    let gis_0_4_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 2 });
    #[rustfmt::skip]
    let gis_0_5_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0 + &gis_0_3_0).leq(&Expr::constant(2)),
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0 + &gis_0_3_0).geq(&Expr::constant(1)),

        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1 + &gis_0_3_1).leq(&Expr::constant(2)),
        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1 + &gis_0_3_1).geq(&Expr::constant(1)),

        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2 + &gis_0_3_2).leq(&Expr::constant(2)),
        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2 + &gis_0_3_2).geq(&Expr::constant(1)),

        (&gis_0_4_0 + &gis_0_5_0).leq(&Expr::constant(1)),
        (&gis_0_4_0 + &gis_0_5_0).geq(&Expr::constant(0)),

        (&gis_0_4_1 + &gis_0_5_1).leq(&Expr::constant(1)),
        (&gis_0_4_1 + &gis_0_5_1).geq(&Expr::constant(0)),

        (&gis_0_4_2 + &gis_0_5_2).leq(&Expr::constant(1)),
        (&gis_0_4_2 + &gis_0_5_2).geq(&Expr::constant(0)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn balancing_timeslots() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            teachers: false,
            timeslots: true,
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([5, 6, 7, 8]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let balancing_constraints = ilp_translator.build_balancing_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_0_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 0 });
    #[rustfmt::skip]
    let gis_0_4_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 0 });
    #[rustfmt::skip]
    let gis_0_5_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 0 });
    #[rustfmt::skip]
    let gis_0_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });
    #[rustfmt::skip]
    let gis_0_2_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 1 });
    #[rustfmt::skip]
    let gis_0_3_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 1 });
    #[rustfmt::skip]
    let gis_0_4_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 1 });
    #[rustfmt::skip]
    let gis_0_5_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 1 });
    #[rustfmt::skip]
    let gis_0_0_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 2 });
    #[rustfmt::skip]
    let gis_0_1_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 2 });
    #[rustfmt::skip]
    let gis_0_2_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 2 });
    #[rustfmt::skip]
    let gis_0_3_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 2 });
    #[rustfmt::skip]
    let gis_0_4_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 2 });
    #[rustfmt::skip]
    let gis_0_5_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_2_0).leq(&Expr::constant(1)),
        (&gis_0_0_0 + &gis_0_2_0).geq(&Expr::constant(0)),

        (&gis_0_0_1 + &gis_0_2_1).leq(&Expr::constant(1)),
        (&gis_0_0_1 + &gis_0_2_1).geq(&Expr::constant(0)),

        (&gis_0_0_2 + &gis_0_2_2).leq(&Expr::constant(1)),
        (&gis_0_0_2 + &gis_0_2_2).geq(&Expr::constant(0)),

        (&gis_0_1_0 + &gis_0_3_0).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &gis_0_3_0).geq(&Expr::constant(0)),

        (&gis_0_1_1 + &gis_0_3_1).leq(&Expr::constant(1)),
        (&gis_0_1_1 + &gis_0_3_1).geq(&Expr::constant(0)),

        (&gis_0_1_2 + &gis_0_3_2).leq(&Expr::constant(1)),
        (&gis_0_1_2 + &gis_0_3_2).geq(&Expr::constant(0)),

        (&gis_0_4_0 + &gis_0_5_0).leq(&Expr::constant(1)),
        (&gis_0_4_0 + &gis_0_5_0).geq(&Expr::constant(0)),

        (&gis_0_4_1 + &gis_0_5_1).leq(&Expr::constant(1)),
        (&gis_0_4_1 + &gis_0_5_1).geq(&Expr::constant(0)),

        (&gis_0_4_2 + &gis_0_5_2).leq(&Expr::constant(1)),
        (&gis_0_4_2 + &gis_0_5_2).geq(&Expr::constant(0)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn balancing_timeslots_2() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            teachers: false,
            timeslots: true,
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([5, 6, 7, 8]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let balancing_constraints = ilp_translator.build_balancing_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_0_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 0 });
    #[rustfmt::skip]
    let gis_0_4_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 0 });
    #[rustfmt::skip]
    let gis_0_5_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 0 });
    #[rustfmt::skip]
    let gis_0_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });
    #[rustfmt::skip]
    let gis_0_2_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 1 });
    #[rustfmt::skip]
    let gis_0_3_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 1 });
    #[rustfmt::skip]
    let gis_0_4_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 1 });
    #[rustfmt::skip]
    let gis_0_5_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 1 });
    #[rustfmt::skip]
    let gis_0_0_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 2 });
    #[rustfmt::skip]
    let gis_0_1_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 2 });
    #[rustfmt::skip]
    let gis_0_2_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 2 });
    #[rustfmt::skip]
    let gis_0_3_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 2 });
    #[rustfmt::skip]
    let gis_0_4_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 2 });
    #[rustfmt::skip]
    let gis_0_5_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_2_0 + &gis_0_4_0 + &gis_0_5_0).leq(&Expr::constant(2)),
        (&gis_0_0_0 + &gis_0_2_0 + &gis_0_4_0 + &gis_0_5_0).geq(&Expr::constant(1)),

        (&gis_0_0_1 + &gis_0_2_1 + &gis_0_4_1 + &gis_0_5_1).leq(&Expr::constant(2)),
        (&gis_0_0_1 + &gis_0_2_1 + &gis_0_4_1 + &gis_0_5_1).geq(&Expr::constant(1)),

        (&gis_0_0_2 + &gis_0_2_2 + &gis_0_4_2 + &gis_0_5_2).leq(&Expr::constant(2)),
        (&gis_0_0_2 + &gis_0_2_2 + &gis_0_4_2 + &gis_0_5_2).geq(&Expr::constant(1)),

        (&gis_0_1_0 + &gis_0_3_0).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &gis_0_3_0).geq(&Expr::constant(0)),

        (&gis_0_1_1 + &gis_0_3_1).leq(&Expr::constant(1)),
        (&gis_0_1_1 + &gis_0_3_1).geq(&Expr::constant(0)),

        (&gis_0_1_2 + &gis_0_3_2).leq(&Expr::constant(1)),
        (&gis_0_1_2 + &gis_0_3_2).geq(&Expr::constant(0)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn balancing_teachers_and_timeslots() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            teachers: true,
            timeslots: true,
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([5, 6, 7, 8]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let balancing_constraints = ilp_translator.build_balancing_constraints();

    use crate::ilp::linexpr::Expr;

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_0_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 0 });
    #[rustfmt::skip]
    let gis_0_4_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 0 });
    #[rustfmt::skip]
    let gis_0_5_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 0 });
    #[rustfmt::skip]
    let gis_0_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });
    #[rustfmt::skip]
    let gis_0_2_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 1 });
    #[rustfmt::skip]
    let gis_0_3_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 1 });
    #[rustfmt::skip]
    let gis_0_4_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 1 });
    #[rustfmt::skip]
    let gis_0_5_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 1 });
    #[rustfmt::skip]
    let gis_0_0_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 2 });
    #[rustfmt::skip]
    let gis_0_1_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 2 });
    #[rustfmt::skip]
    let gis_0_2_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 2 });
    #[rustfmt::skip]
    let gis_0_3_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 2 });
    #[rustfmt::skip]
    let gis_0_4_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 2 });
    #[rustfmt::skip]
    let gis_0_5_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_2_0).leq(&Expr::constant(1)),
        (&gis_0_0_0 + &gis_0_2_0).geq(&Expr::constant(0)),

        (&gis_0_0_1 + &gis_0_2_1).leq(&Expr::constant(1)),
        (&gis_0_0_1 + &gis_0_2_1).geq(&Expr::constant(0)),

        (&gis_0_0_2 + &gis_0_2_2).leq(&Expr::constant(1)),
        (&gis_0_0_2 + &gis_0_2_2).geq(&Expr::constant(0)),

        (&gis_0_1_0 + &gis_0_3_0).leq(&Expr::constant(1)),
        (&gis_0_1_0 + &gis_0_3_0).geq(&Expr::constant(0)),

        (&gis_0_1_1 + &gis_0_3_1).leq(&Expr::constant(1)),
        (&gis_0_1_1 + &gis_0_3_1).geq(&Expr::constant(0)),

        (&gis_0_1_2 + &gis_0_3_2).leq(&Expr::constant(1)),
        (&gis_0_1_2 + &gis_0_3_2).geq(&Expr::constant(0)),

        (&gis_0_4_0 + &gis_0_5_0).leq(&Expr::constant(1)),
        (&gis_0_4_0 + &gis_0_5_0).geq(&Expr::constant(0)),

        (&gis_0_4_1 + &gis_0_5_1).leq(&Expr::constant(1)),
        (&gis_0_4_1 + &gis_0_5_1).geq(&Expr::constant(0)),

        (&gis_0_4_2 + &gis_0_5_2).leq(&Expr::constant(1)),
        (&gis_0_4_2 + &gis_0_5_2).geq(&Expr::constant(0)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn no_balancing() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            teachers: false,
            timeslots: false,
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([5, 6, 7, 8]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let balancing_constraints = ilp_translator.build_balancing_constraints();

    let expected_result = BTreeSet::new();

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn balancing_timeslots_with_ghost_group() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(4).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            teachers: false,
            timeslots: true,
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 3,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 3,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 3,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 3,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([5, 6, 7, 8]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let balancing_constraints = ilp_translator.build_balancing_constraints();

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_0_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 0 });
    #[rustfmt::skip]
    let gis_0_4_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 0 });
    #[rustfmt::skip]
    let gis_0_5_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 0 });
    #[rustfmt::skip]
    let gis_0_6_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 0 });
    #[rustfmt::skip]
    let gis_0_7_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 0 });
    #[rustfmt::skip]
    let gis_0_8_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 8, group: 0 });
    #[rustfmt::skip]
    let gis_0_9_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 9, group: 0 });
    #[rustfmt::skip]
    let gis_0_a_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 10, group: 0 });
    #[rustfmt::skip]
    let gis_0_b_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 11, group: 0 });
    #[rustfmt::skip]
    let gis_0_c_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 12, group: 0 });
    #[rustfmt::skip]
    let gis_0_d_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 13, group: 0 });
    #[rustfmt::skip]
    let gis_0_e_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 14, group: 0 });
    #[rustfmt::skip]
    let gis_0_f_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 15, group: 0 });
    #[rustfmt::skip]
    let gis_0_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });
    #[rustfmt::skip]
    let gis_0_2_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 1 });
    #[rustfmt::skip]
    let gis_0_3_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 1 });
    #[rustfmt::skip]
    let gis_0_4_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 1 });
    #[rustfmt::skip]
    let gis_0_5_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 1 });
    #[rustfmt::skip]
    let gis_0_6_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 1 });
    #[rustfmt::skip]
    let gis_0_7_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 1 });
    #[rustfmt::skip]
    let gis_0_8_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 8, group: 1 });
    #[rustfmt::skip]
    let gis_0_9_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 9, group: 1 });
    #[rustfmt::skip]
    let gis_0_a_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 10, group: 1 });
    #[rustfmt::skip]
    let gis_0_b_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 11, group: 1 });
    #[rustfmt::skip]
    let gis_0_c_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 12, group: 1 });
    #[rustfmt::skip]
    let gis_0_d_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 13, group: 1 });
    #[rustfmt::skip]
    let gis_0_e_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 14, group: 1 });
    #[rustfmt::skip]
    let gis_0_f_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 15, group: 1 });
    #[rustfmt::skip]
    let gis_0_0_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 2 });
    #[rustfmt::skip]
    let gis_0_1_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 2 });
    #[rustfmt::skip]
    let gis_0_2_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 2 });
    #[rustfmt::skip]
    let gis_0_3_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 2 });
    #[rustfmt::skip]
    let gis_0_4_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 2 });
    #[rustfmt::skip]
    let gis_0_5_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 2 });
    #[rustfmt::skip]
    let gis_0_6_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 2 });
    #[rustfmt::skip]
    let gis_0_7_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 2 });
    #[rustfmt::skip]
    let gis_0_8_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 8, group: 2 });
    #[rustfmt::skip]
    let gis_0_9_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 9, group: 2 });
    #[rustfmt::skip]
    let gis_0_a_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 10, group: 2 });
    #[rustfmt::skip]
    let gis_0_b_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 11, group: 2 });
    #[rustfmt::skip]
    let gis_0_c_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 12, group: 2 });
    #[rustfmt::skip]
    let gis_0_d_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 13, group: 2 });
    #[rustfmt::skip]
    let gis_0_e_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 14, group: 2 });
    #[rustfmt::skip]
    let gis_0_f_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 15, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0 + &gis_0_3_0 + &gis_0_8_0 + &gis_0_9_0 + &gis_0_a_0 + &gis_0_b_0).eq(&Expr::constant(2)),
        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1 + &gis_0_3_1 + &gis_0_8_1 + &gis_0_9_1 + &gis_0_a_1 + &gis_0_b_1).eq(&Expr::constant(2)),
        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2 + &gis_0_3_2 + &gis_0_8_2 + &gis_0_9_2 + &gis_0_a_2 + &gis_0_b_2).eq(&Expr::constant(2)),

        (&gis_0_4_0 + &gis_0_5_0 + &gis_0_6_0 + &gis_0_7_0 + &gis_0_c_0 + &gis_0_d_0 + &gis_0_e_0 + &gis_0_f_0).eq(&Expr::constant(2)),
        (&gis_0_4_1 + &gis_0_5_1 + &gis_0_6_1 + &gis_0_7_1 + &gis_0_c_1 + &gis_0_d_1 + &gis_0_e_1 + &gis_0_f_1).eq(&Expr::constant(2)),
        (&gis_0_4_2 + &gis_0_5_2 + &gis_0_6_2 + &gis_0_7_2 + &gis_0_c_2 + &gis_0_d_2 + &gis_0_e_2 + &gis_0_f_2).eq(&Expr::constant(2)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn balancing_timeslots_with_ghost_group_2() {
    let general = GeneralData {
        teacher_count: 2,
        week_count: NonZeroU32::new(3).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            teachers: false,
            timeslots: true,
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots: vec![
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                teacher: 1,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
        ],
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([3, 4]),
                    can_be_extended: true,
                },
                GroupDesc {
                    students: BTreeSet::new(),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([5, 6, 7, 8]),
        },
        ..Subject::default()
    }];
    let incompatibilities = vec![];
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
    let slot_groupings = vec![];
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let balancing_constraints = ilp_translator.build_balancing_constraints();

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_0_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 0 });
    #[rustfmt::skip]
    let gis_0_4_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 0 });
    #[rustfmt::skip]
    let gis_0_5_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 0 });
    #[rustfmt::skip]
    let gis_0_6_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 0 });
    #[rustfmt::skip]
    let gis_0_7_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 0 });
    #[rustfmt::skip]
    let gis_0_8_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 8, group: 0 });
    #[rustfmt::skip]
    let gis_0_9_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 9, group: 0 });
    #[rustfmt::skip]
    let gis_0_a_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 10, group: 0 });
    #[rustfmt::skip]
    let gis_0_b_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 11, group: 0 });
    #[rustfmt::skip]
    let gis_0_0_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 1 });
    #[rustfmt::skip]
    let gis_0_1_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 1 });
    #[rustfmt::skip]
    let gis_0_2_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 1 });
    #[rustfmt::skip]
    let gis_0_3_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 1 });
    #[rustfmt::skip]
    let gis_0_4_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 1 });
    #[rustfmt::skip]
    let gis_0_5_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 1 });
    #[rustfmt::skip]
    let gis_0_6_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 1 });
    #[rustfmt::skip]
    let gis_0_7_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 1 });
    #[rustfmt::skip]
    let gis_0_8_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 8, group: 1 });
    #[rustfmt::skip]
    let gis_0_9_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 9, group: 1 });
    #[rustfmt::skip]
    let gis_0_a_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 10, group: 1 });
    #[rustfmt::skip]
    let gis_0_b_1 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 11, group: 1 });
    #[rustfmt::skip]
    let gis_0_0_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 2 });
    #[rustfmt::skip]
    let gis_0_1_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 1, group: 2 });
    #[rustfmt::skip]
    let gis_0_2_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 2 });
    #[rustfmt::skip]
    let gis_0_3_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 2 });
    #[rustfmt::skip]
    let gis_0_4_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 2 });
    #[rustfmt::skip]
    let gis_0_5_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 5, group: 2 });
    #[rustfmt::skip]
    let gis_0_6_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 2 });
    #[rustfmt::skip]
    let gis_0_7_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 2 });
    #[rustfmt::skip]
    let gis_0_8_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 8, group: 2 });
    #[rustfmt::skip]
    let gis_0_9_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 9, group: 2 });
    #[rustfmt::skip]
    let gis_0_a_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 10, group: 2 });
    #[rustfmt::skip]
    let gis_0_b_2 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 11, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0 + &gis_0_6_0 + &gis_0_7_0 + &gis_0_8_0).geq(&Expr::constant(1)),
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0 + &gis_0_6_0 + &gis_0_7_0 + &gis_0_8_0).leq(&Expr::constant(2)),

        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1 + &gis_0_6_1 + &gis_0_7_1 + &gis_0_8_1).geq(&Expr::constant(1)),
        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1 + &gis_0_6_1 + &gis_0_7_1 + &gis_0_8_1).leq(&Expr::constant(2)),
        
        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2 + &gis_0_6_2 + &gis_0_7_2 + &gis_0_8_2).geq(&Expr::constant(1)),
        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2 + &gis_0_6_2 + &gis_0_7_2 + &gis_0_8_2).leq(&Expr::constant(2)),

        (&gis_0_3_0 + &gis_0_4_0 + &gis_0_5_0 + &gis_0_9_0 + &gis_0_a_0 + &gis_0_b_0).geq(&Expr::constant(1)),
        (&gis_0_3_0 + &gis_0_4_0 + &gis_0_5_0 + &gis_0_9_0 + &gis_0_a_0 + &gis_0_b_0).leq(&Expr::constant(2)),

        (&gis_0_3_1 + &gis_0_4_1 + &gis_0_5_1 + &gis_0_9_1 + &gis_0_a_1 + &gis_0_b_1).geq(&Expr::constant(1)),
        (&gis_0_3_1 + &gis_0_4_1 + &gis_0_5_1 + &gis_0_9_1 + &gis_0_a_1 + &gis_0_b_1).leq(&Expr::constant(2)),

        (&gis_0_3_2 + &gis_0_4_2 + &gis_0_5_2 + &gis_0_9_2 + &gis_0_a_2 + &gis_0_b_2).geq(&Expr::constant(1)),
        (&gis_0_3_2 + &gis_0_4_2 + &gis_0_5_2 + &gis_0_9_2 + &gis_0_a_2 + &gis_0_b_2).leq(&Expr::constant(2)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}
