use super::*;

#[test]
fn trivial_validated_data() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 0,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = SubjectList::new();
    let incompatibility_groups = IncompatibilityGroupList::new();
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();

    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let expected_result = ValidatedData {
        general: general.clone(),
        subjects: subjects.clone(),
        incompatibilities: incompatibilities.clone(),
        incompatibility_groups: incompatibility_groups.clone(),
        students: students.clone(),
        slot_groupings: slot_groupings.clone(),
        slot_grouping_incompats: grouping_incompats.clone(),
    };

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = vec![IncompatibilityGroup {
        slots: BTreeSet::from([SlotWithDuration {
            duration: NonZeroU32::new(60).unwrap(),
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        }]),
    }];
    let incompatibilities = vec![Incompatibility {
        groups: BTreeSet::from([0]),
        max_count: 0,
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
        incompatibility_groups: incompatibility_groups.clone(),
        incompatibilities: incompatibilities.clone(),
        students: students.clone(),
        slot_groupings: slot_groupings.clone(),
        slot_grouping_incompats: grouping_incompats.clone(),
    };

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
            cost: 0,
        }],
        groups: GroupsDesc {
            prefilled_groups: vec![],
            not_assigned: BTreeSet::new(),
        },
        ..Subject::default()
    }];
    let incompatibility_groups = IncompatibilityGroupList::new();
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
            cost: 0,
        }],
        groups: GroupsDesc {
            prefilled_groups: vec![],
            not_assigned: BTreeSet::new(),
        },
        ..Subject::default()
    }];
    let incompatibility_groups = IncompatibilityGroupList::new();
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 0,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };
    let subjects = SubjectList::new();
    let incompatibility_groups = vec![IncompatibilityGroup {
        slots: BTreeSet::from([SlotWithDuration {
            duration: NonZeroU32::new(60).unwrap(),
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 1).unwrap(),
            },
        }]),
    }];
    let incompatibilities = vec![Incompatibility {
        groups: BTreeSet::from([0]),
        max_count: 0,
    }];
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();
    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibility_groups,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::IncompatibilityGroupWithSlotOverlappingNextDay(
            0,
            SlotWithDuration {
                duration: NonZeroU32::new(60).unwrap(),
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(23, 1).unwrap(),
                },
            }
        ))
    );
}

#[test]
fn invalid_teacher_number() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
            cost: 0,
        }],
        groups: GroupsDesc {
            prefilled_groups: vec![],
            not_assigned: BTreeSet::new(),
        },
        ..Subject::default()
    }];
    let incompatibility_groups = IncompatibilityGroupList::new();
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = SubjectList::new();
    let incompatibility_groups = vec![IncompatibilityGroup {
        slots: BTreeSet::from([SlotWithDuration {
            duration: NonZeroU32::new(60).unwrap(),
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 0).unwrap(),
            },
        }]),
    }];
    let incompatibilities = vec![Incompatibility {
        groups: BTreeSet::from([0]),
        max_count: 0,
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
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
                cost: 0,
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
    ];
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
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
                cost: 0,
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
    ];
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                    cost: 0,
                },
                SlotWithTeacher {
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                    cost: 0,
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
                cost: 0,
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
    ];
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
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
                cost: 0,
            }],
            groups: GroupsDesc {
                prefilled_groups: vec![],
                not_assigned: BTreeSet::new(),
            },
            ..Subject::default()
        },
    ];
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 0,
        week_count: NonZeroU32::new(1).unwrap(),
        interrogations_per_week: Some(10..8),
        max_interrogations_per_day: None,
    };

    let subjects = SubjectList::new();
    let incompatibility_groups = IncompatibilityGroupList::new();
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();

    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
            incompatibility_groups,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        )
        .err(),
        Some(Error::SubjectWithPeriodicityTooBig(0, 2, 1))
    );
}

#[test]
fn group_in_slot_variables() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
fn group_on_slot_selection_variables() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(4).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        balancing_requirements: BalancingRequirements {
            constraints: BalancingConstraints::Strict,
            slot_selections: vec![
                BalancingSlotSelection {
                    slot_groups: vec![
                        BalancingSlotGroup {
                            slots: BTreeSet::from([0, 2]),
                            count: 1,
                        },
                        BalancingSlotGroup {
                            slots: BTreeSet::from([4, 6]),
                            count: 1,
                        },
                    ],
                },
                BalancingSlotSelection {
                    slot_groups: vec![
                        BalancingSlotGroup {
                            slots: BTreeSet::from([1, 3]),
                            count: 1,
                        },
                        BalancingSlotGroup {
                            slots: BTreeSet::from([5, 7]),
                            count: 1,
                        },
                    ],
                },
            ],
        },
        slots: vec![
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 3,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 3,
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
                    students: BTreeSet::from([]),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
    }];
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let group_on_slot_selection_variables =
        ilp_translator.build_group_on_slot_selection_variables();

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 },
        Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 },
        Variable::GroupOnSlotSelection { subject: 0, slot_selection: 1, group: 0 },
        Variable::GroupOnSlotSelection { subject: 0, slot_selection: 1, group: 1 },
    ]);

    assert_eq!(group_on_slot_selection_variables, expected_result);
}

#[test]
fn default_group_on_slot_selection_variables() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(4).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 3,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 3,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
    ];
    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        balancing_requirements: BalancingRequirements::default_from_slots(&slots),
        slots,
        groups: GroupsDesc {
            prefilled_groups: vec![
                GroupDesc {
                    students: BTreeSet::from([0, 1, 2]),
                    can_be_extended: false,
                },
                GroupDesc {
                    students: BTreeSet::from([]),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
    }];
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let group_on_slot_selection_variables =
        ilp_translator.build_group_on_slot_selection_variables();

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 },
        Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 },
    ]);

    assert_eq!(group_on_slot_selection_variables, expected_result);
}

#[test]
fn dynamic_group_assignment_variables() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
fn use_grouping() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
fn incomapt_group_for_student_variables() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::from([
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
            ]),
        },
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
            ]),
        },
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
            ]),
        },
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(14, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(120).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(14, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(120).unwrap(),
                },
            ]),
        },
    ]);
    let incompatibilities = IncompatibilityList::from([
        Incompatibility {
            groups: BTreeSet::from([0, 1]),
            max_count: 1,
        },
        Incompatibility {
            groups: BTreeSet::from([2]),
            max_count: 0,
        },
        Incompatibility {
            groups: BTreeSet::from([2, 3]),
            max_count: 0,
        },
    ]);
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
        Student {
            incompatibilities: BTreeSet::from([1]),
        },
        Student {
            incompatibilities: BTreeSet::from([2]),
        },
        Student {
            incompatibilities: BTreeSet::from([0, 1]),
        },
        Student {
            incompatibilities: BTreeSet::from([0, 1, 2]),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibility_groups,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let incompat_group_for_student_variables =
        ilp_translator.build_incompat_group_for_student_variables();

    let expected_result = BTreeSet::from([
        Variable::IncompatGroupForStudent {
            incompat_group: 0,
            student: 1,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 0,
            student: 4,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 0,
            student: 5,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 1,
            student: 1,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 1,
            student: 4,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 1,
            student: 5,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 2,
            student: 2,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 2,
            student: 3,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 2,
            student: 4,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 2,
            student: 5,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 3,
            student: 3,
        },
        Variable::IncompatGroupForStudent {
            incompat_group: 3,
            student: 5,
        },
    ]);

    assert_eq!(incompat_group_for_student_variables, expected_result);
}

#[test]
fn at_most_max_groups_per_slot_constraints() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Thursday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
            period_is_strict: false,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let one_interrogation_per_period_contraints =
        ilp_translator.build_one_interrogation_per_period_constraints();

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

        (&gis_0_1_0 + &gis_0_2_0).eq(&Expr::constant(1)),
        (&dga_0_1_1_3 + &dga_0_2_1_3).eq(&Expr::constant(1)),
        (&dga_0_1_1_4 + &dga_0_2_1_4).eq(&Expr::constant(1)),
        (&dga_0_1_1_5 + &dga_0_2_1_5).eq(&Expr::constant(1)),

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
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(5).unwrap(),
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
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 3,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 4,
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
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 2,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 3,
                        weekday: time::Weekday::Tuesday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 4,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let one_interrogation_per_period_contraints =
        ilp_translator.build_one_interrogation_per_period_constraints();

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
    let gis_1_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_1_1_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 1, group: 0 });
    #[rustfmt::skip]
    let gis_1_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_1_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 3, group: 0 });
    #[rustfmt::skip]
    let gis_1_4_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 1, slot: 4, group: 0 });

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
    let dga_0_4_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_4_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_4_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 1, student: 5 });

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
    let dga_1_4_1_0 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 4, group: 1, student: 0 });
    #[rustfmt::skip]
    let dga_1_4_1_1 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 4, group: 1, student: 1 });
    #[rustfmt::skip]
    let dga_1_4_1_2 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 1, slot: 4, group: 1, student: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0).eq(&Expr::constant(1)),
        (&dga_0_0_1_3 + &dga_0_1_1_3).eq(&Expr::constant(1)),
        (&dga_0_0_1_4 + &dga_0_1_1_4).eq(&Expr::constant(1)),
        (&dga_0_0_1_5 + &dga_0_1_1_5).eq(&Expr::constant(1)),

        (&gis_0_1_0 + &gis_0_2_0).eq(&Expr::constant(1)),
        (&dga_0_1_1_3 + &dga_0_2_1_3).eq(&Expr::constant(1)),
        (&dga_0_1_1_4 + &dga_0_2_1_4).eq(&Expr::constant(1)),
        (&dga_0_1_1_5 + &dga_0_2_1_5).eq(&Expr::constant(1)),

        (&gis_0_2_0 + &gis_0_3_0).eq(&Expr::constant(1)),
        (&dga_0_2_1_3 + &dga_0_3_1_3).eq(&Expr::constant(1)),
        (&dga_0_2_1_4 + &dga_0_3_1_4).eq(&Expr::constant(1)),
        (&dga_0_2_1_5 + &dga_0_3_1_5).eq(&Expr::constant(1)),

        (&gis_0_3_0 + &gis_0_4_0).eq(&Expr::constant(1)),
        (&dga_0_3_1_3 + &dga_0_4_1_3).eq(&Expr::constant(1)),
        (&dga_0_3_1_4 + &dga_0_4_1_4).eq(&Expr::constant(1)),
        (&dga_0_3_1_5 + &dga_0_4_1_5).eq(&Expr::constant(1)),

        (&gis_1_0_0 + &gis_1_1_0).eq(&Expr::constant(1)),
        (&dga_1_0_1_0 + &dga_1_1_1_0).eq(&Expr::constant(1)),
        (&dga_1_0_1_1 + &dga_1_1_1_1).eq(&Expr::constant(1)),
        (&dga_1_0_1_2 + &dga_1_1_1_2).eq(&Expr::constant(1)),

        (&gis_1_2_0 + &gis_1_3_0).eq(&Expr::constant(1)),
        (&dga_1_2_1_0 + &dga_1_3_1_0).eq(&Expr::constant(1)),
        (&dga_1_2_1_1 + &dga_1_3_1_1).eq(&Expr::constant(1)),
        (&dga_1_2_1_2 + &dga_1_3_1_2).eq(&Expr::constant(1)),

        (&gis_1_3_0 + &gis_1_4_0).eq(&Expr::constant(1)),
        (&dga_1_3_1_0 + &dga_1_4_1_0).eq(&Expr::constant(1)),
        (&dga_1_3_1_1 + &dga_1_4_1_1).eq(&Expr::constant(1)),
        (&dga_1_3_1_2 + &dga_1_4_1_2).eq(&Expr::constant(1)),
    ]);

    assert_eq!(one_interrogation_per_period_contraints, expected_result);
}

#[test]
fn students_per_group_count() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
fn interrogations_per_week() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Friday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots1 = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(17, 0).unwrap(),
            },
        },
    ];
    let slots2 = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(17, 0).unwrap(),
            },
        },
    ];
    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            balancing_requirements: BalancingRequirements::default_from_slots(&slots1),
            slots: slots1,
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
            balancing_requirements: BalancingRequirements::default_from_slots(&slots2),
            slots: slots2,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
    let goss_0_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_0_1_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 1, group: 0 });
    #[rustfmt::skip]
    let goss_1_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 1, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_1_1_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 1, slot_selection: 1, group: 0 });

    #[rustfmt::skip]
    let goss_0_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_0_1_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 1, group: 1 });
    #[rustfmt::skip]
    let goss_1_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 1, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_1_1_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 1, slot_selection: 1, group: 1 });

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

        // Auto Group On Week Selection
        gis_0_0_0.leq(&goss_0_0_0),
        gis_0_0_0.geq(&goss_0_0_0),

        gis_0_1_0.leq(&goss_0_1_0),
        gis_0_1_0.geq(&goss_0_1_0),

        gis_1_0_0.leq(&goss_1_0_0),
        gis_1_0_0.geq(&goss_1_0_0),

        gis_1_1_0.leq(&goss_1_1_0),
        gis_1_1_0.geq(&goss_1_1_0),

        gis_0_0_1.leq(&goss_0_0_1),
        gis_0_0_1.geq(&goss_0_0_1),

        gis_0_1_1.leq(&goss_0_1_1),
        gis_0_1_1.geq(&goss_0_1_1),

        gis_1_0_1.leq(&goss_1_0_1),
        gis_1_0_1.geq(&goss_1_0_1),

        gis_1_1_1.leq(&goss_1_1_1),
        gis_1_1_1.geq(&goss_1_1_1),

        (&goss_0_0_0 + &goss_0_1_0).eq(&Expr::constant(1)),
        (&goss_1_0_0 + &goss_1_1_0).eq(&Expr::constant(1)),
        (&goss_0_0_1 + &goss_0_1_1).eq(&Expr::constant(1)),
        (&goss_1_0_1 + &goss_1_1_1).eq(&Expr::constant(1)),
    ]);

    assert_eq!(constraints, expected_result);
}

#[test]
fn colloscope_with_dynamic_groups() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots1 = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(17, 0).unwrap(),
            },
        },
    ];
    let slots2 = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(17, 0).unwrap(),
            },
        },
    ];
    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            balancing_requirements: BalancingRequirements::default_from_slots(&slots1),
            slots: slots1,
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
            balancing_requirements: BalancingRequirements::default_from_slots(&slots2),
            slots: slots2,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
    let goss_0_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_0_1_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 1, group: 0 });
    #[rustfmt::skip]
    let goss_1_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 1, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_1_1_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 1, slot_selection: 1, group: 0 });

    #[rustfmt::skip]
    let goss_0_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_0_1_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 1, group: 1 });
    #[rustfmt::skip]
    let goss_1_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 1, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_1_1_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 1, slot_selection: 1, group: 1 });

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

        // Auto Group On Week Selection
        gis_0_0_0.leq(&goss_0_0_0),
        gis_0_0_0.geq(&goss_0_0_0),

        gis_0_1_0.leq(&goss_0_1_0),
        gis_0_1_0.geq(&goss_0_1_0),

        gis_1_0_0.leq(&goss_1_0_0),
        gis_1_0_0.geq(&goss_1_0_0),

        gis_1_1_0.leq(&goss_1_1_0),
        gis_1_1_0.geq(&goss_1_1_0),

        gis_0_0_1.leq(&goss_0_0_1),
        gis_0_0_1.geq(&goss_0_0_1),

        gis_0_1_1.leq(&goss_0_1_1),
        gis_0_1_1.geq(&goss_0_1_1),

        gis_1_0_1.leq(&goss_1_0_1),
        gis_1_0_1.geq(&goss_1_0_1),

        gis_1_1_1.leq(&goss_1_1_1),
        gis_1_1_1.geq(&goss_1_1_1),

        (&goss_0_0_0 + &goss_0_1_0).eq(&Expr::constant(1)),
        (&goss_1_0_0 + &goss_1_1_0).eq(&Expr::constant(1)),
        (&goss_0_0_1 + &goss_0_1_1).eq(&Expr::constant(1)),
        (&goss_1_0_1 + &goss_1_1_1).eq(&Expr::constant(1)),
    ]);

    assert_eq!(constraints, expected_result);
}

#[test]
fn at_most_one_interrogation_per_empty_group() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
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
                    cost: 0,
                    teacher: 0,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Monday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
                    cost: 0,
                    teacher: 1,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Friday,
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Wednesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Wednesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
    ];

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            constraints: BalancingConstraints::OverallOnly,
            slot_selections: BalancingRequirements::balance_teachers_from_slots(&slots),
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
    let goss_0_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_0_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_0_0_2 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0 + &gis_0_3_0).leq(&(2*&goss_0_0_0)),
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0 + &gis_0_3_0).geq(&(1*&goss_0_0_0)),

        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1 + &gis_0_3_1).leq(&(2*&goss_0_0_1)),
        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1 + &gis_0_3_1).geq(&(1*&goss_0_0_1)),

        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2 + &gis_0_3_2).leq(&(2*&goss_0_0_2)),
        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2 + &gis_0_3_2).geq(&(1*&goss_0_0_2)),

        (&gis_0_4_0 + &gis_0_5_0).leq(&(1*&goss_0_0_0)),
        (&gis_0_4_0 + &gis_0_5_0).geq(&(0*&goss_0_0_0)),

        (&gis_0_4_1 + &gis_0_5_1).leq(&(1*&goss_0_0_1)),
        (&gis_0_4_1 + &gis_0_5_1).geq(&(0*&goss_0_0_1)),

        (&gis_0_4_2 + &gis_0_5_2).leq(&(1*&goss_0_0_2)),
        (&gis_0_4_2 + &gis_0_5_2).geq(&(0*&goss_0_0_2)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn balancing_timeslots() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Wednesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Wednesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
    ];

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            constraints: BalancingConstraints::OverallOnly,
            slot_selections: BalancingRequirements::balance_timeslots_from_slots(&slots),
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
    let goss_0_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_0_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_0_0_2 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_2_0).leq(&(1*&goss_0_0_0)),
        (&gis_0_0_0 + &gis_0_2_0).geq(&(0*&goss_0_0_0)),

        (&gis_0_0_1 + &gis_0_2_1).leq(&(1*&goss_0_0_1)),
        (&gis_0_0_1 + &gis_0_2_1).geq(&(0*&goss_0_0_1)),

        (&gis_0_0_2 + &gis_0_2_2).leq(&(1*&goss_0_0_2)),
        (&gis_0_0_2 + &gis_0_2_2).geq(&(0*&goss_0_0_2)),

        (&gis_0_1_0 + &gis_0_3_0).leq(&(1*&goss_0_0_0)),
        (&gis_0_1_0 + &gis_0_3_0).geq(&(0*&goss_0_0_0)),

        (&gis_0_1_1 + &gis_0_3_1).leq(&(1*&goss_0_0_1)),
        (&gis_0_1_1 + &gis_0_3_1).geq(&(0*&goss_0_0_1)),

        (&gis_0_1_2 + &gis_0_3_2).leq(&(1*&goss_0_0_2)),
        (&gis_0_1_2 + &gis_0_3_2).geq(&(0*&goss_0_0_2)),

        (&gis_0_4_0 + &gis_0_5_0).leq(&(1*&goss_0_0_0)),
        (&gis_0_4_0 + &gis_0_5_0).geq(&(0*&goss_0_0_0)),

        (&gis_0_4_1 + &gis_0_5_1).leq(&(1*&goss_0_0_1)),
        (&gis_0_4_1 + &gis_0_5_1).geq(&(0*&goss_0_0_1)),

        (&gis_0_4_2 + &gis_0_5_2).leq(&(1*&goss_0_0_2)),
        (&gis_0_4_2 + &gis_0_5_2).geq(&(0*&goss_0_0_2)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn balancing_timeslots_2() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
    ];

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            constraints: BalancingConstraints::OverallOnly,
            slot_selections: BalancingRequirements::balance_timeslots_from_slots(&slots),
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
    let goss_0_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_0_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_0_0_2 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_2_0 + &gis_0_4_0 + &gis_0_5_0).leq(&(2*&goss_0_0_0)),
        (&gis_0_0_0 + &gis_0_2_0 + &gis_0_4_0 + &gis_0_5_0).geq(&(1*&goss_0_0_0)),

        (&gis_0_0_1 + &gis_0_2_1 + &gis_0_4_1 + &gis_0_5_1).leq(&(2*&goss_0_0_1)),
        (&gis_0_0_1 + &gis_0_2_1 + &gis_0_4_1 + &gis_0_5_1).geq(&(1*&goss_0_0_1)),

        (&gis_0_0_2 + &gis_0_2_2 + &gis_0_4_2 + &gis_0_5_2).leq(&(2*&goss_0_0_2)),
        (&gis_0_0_2 + &gis_0_2_2 + &gis_0_4_2 + &gis_0_5_2).geq(&(1*&goss_0_0_2)),

        (&gis_0_1_0 + &gis_0_3_0).leq(&(1*&goss_0_0_0)),
        (&gis_0_1_0 + &gis_0_3_0).geq(&(0*&goss_0_0_0)),

        (&gis_0_1_1 + &gis_0_3_1).leq(&(1*&goss_0_0_1)),
        (&gis_0_1_1 + &gis_0_3_1).geq(&(0*&goss_0_0_1)),

        (&gis_0_1_2 + &gis_0_3_2).leq(&(1*&goss_0_0_2)),
        (&gis_0_1_2 + &gis_0_3_2).geq(&(0*&goss_0_0_2)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn balancing_teachers_and_timeslots() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
    ];

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            constraints: BalancingConstraints::OverallOnly,
            slot_selections: BalancingRequirements::balance_teachers_and_timeslots_from_slots(
                &slots,
            ),
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
    let goss_0_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_0_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_0_0_2 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_2_0).leq(&(1*&goss_0_0_0)),
        (&gis_0_0_0 + &gis_0_2_0).geq(&(0*&goss_0_0_0)),

        (&gis_0_0_1 + &gis_0_2_1).leq(&(1*&goss_0_0_1)),
        (&gis_0_0_1 + &gis_0_2_1).geq(&(0*&goss_0_0_1)),

        (&gis_0_0_2 + &gis_0_2_2).leq(&(1*&goss_0_0_2)),
        (&gis_0_0_2 + &gis_0_2_2).geq(&(0*&goss_0_0_2)),

        (&gis_0_1_0 + &gis_0_3_0).leq(&(1*&goss_0_0_0)),
        (&gis_0_1_0 + &gis_0_3_0).geq(&(0*&goss_0_0_0)),

        (&gis_0_1_1 + &gis_0_3_1).leq(&(1*&goss_0_0_1)),
        (&gis_0_1_1 + &gis_0_3_1).geq(&(0*&goss_0_0_1)),

        (&gis_0_1_2 + &gis_0_3_2).leq(&(1*&goss_0_0_2)),
        (&gis_0_1_2 + &gis_0_3_2).geq(&(0*&goss_0_0_2)),

        (&gis_0_4_0 + &gis_0_5_0).leq(&(1*&goss_0_0_0)),
        (&gis_0_4_0 + &gis_0_5_0).geq(&(0*&goss_0_0_0)),

        (&gis_0_4_1 + &gis_0_5_1).leq(&(1*&goss_0_0_1)),
        (&gis_0_4_1 + &gis_0_5_1).geq(&(0*&goss_0_0_1)),

        (&gis_0_4_2 + &gis_0_5_2).leq(&(1*&goss_0_0_2)),
        (&gis_0_4_2 + &gis_0_5_2).geq(&(0*&goss_0_0_2)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn no_balancing() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
    ];
    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements::default_from_slots(&slots),
        duration: NonZeroU32::new(60).unwrap(),
        slots,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(4).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 3,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 3,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 3,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 3,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
    ];

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            constraints: BalancingConstraints::OverallOnly,
            slot_selections: BalancingRequirements::balance_timeslots_from_slots(&slots),
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
    let goss_0_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_0_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_0_0_2 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0 + &gis_0_3_0 + &gis_0_8_0 + &gis_0_9_0 + &gis_0_a_0 + &gis_0_b_0).eq(&(2*&goss_0_0_0)),
        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1 + &gis_0_3_1 + &gis_0_8_1 + &gis_0_9_1 + &gis_0_a_1 + &gis_0_b_1).eq(&(2*&goss_0_0_1)),
        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2 + &gis_0_3_2 + &gis_0_8_2 + &gis_0_9_2 + &gis_0_a_2 + &gis_0_b_2).eq(&(2*&goss_0_0_2)),

        (&gis_0_4_0 + &gis_0_5_0 + &gis_0_6_0 + &gis_0_7_0 + &gis_0_c_0 + &gis_0_d_0 + &gis_0_e_0 + &gis_0_f_0).eq(&(2*&goss_0_0_0)),
        (&gis_0_4_1 + &gis_0_5_1 + &gis_0_6_1 + &gis_0_7_1 + &gis_0_c_1 + &gis_0_d_1 + &gis_0_e_1 + &gis_0_f_1).eq(&(2*&goss_0_0_1)),
        (&gis_0_4_2 + &gis_0_5_2 + &gis_0_6_2 + &gis_0_7_2 + &gis_0_c_2 + &gis_0_d_2 + &gis_0_e_2 + &gis_0_f_2).eq(&(2*&goss_0_0_2)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn balancing_timeslots_with_ghost_group_2() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(3).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots = vec![
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 0,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
        SlotWithTeacher {
            cost: 0,
            teacher: 1,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
        },
    ];

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(1).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            constraints: BalancingConstraints::OverallOnly,
            slot_selections: BalancingRequirements::balance_timeslots_from_slots(&slots),
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
    let goss_0_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_0_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_0_0_2 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0 + &gis_0_6_0 + &gis_0_7_0 + &gis_0_8_0).geq(&(1*&goss_0_0_0)),
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0 + &gis_0_6_0 + &gis_0_7_0 + &gis_0_8_0).leq(&(2*&goss_0_0_0)),

        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1 + &gis_0_6_1 + &gis_0_7_1 + &gis_0_8_1).geq(&(1*&goss_0_0_1)),
        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1 + &gis_0_6_1 + &gis_0_7_1 + &gis_0_8_1).leq(&(2*&goss_0_0_1)),

        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2 + &gis_0_6_2 + &gis_0_7_2 + &gis_0_8_2).geq(&(1*&goss_0_0_2)),
        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2 + &gis_0_6_2 + &gis_0_7_2 + &gis_0_8_2).leq(&(2*&goss_0_0_2)),

        (&gis_0_3_0 + &gis_0_4_0 + &gis_0_5_0 + &gis_0_9_0 + &gis_0_a_0 + &gis_0_b_0).geq(&(1*&goss_0_0_0)),
        (&gis_0_3_0 + &gis_0_4_0 + &gis_0_5_0 + &gis_0_9_0 + &gis_0_a_0 + &gis_0_b_0).leq(&(2*&goss_0_0_0)),

        (&gis_0_3_1 + &gis_0_4_1 + &gis_0_5_1 + &gis_0_9_1 + &gis_0_a_1 + &gis_0_b_1).geq(&(1*&goss_0_0_1)),
        (&gis_0_3_1 + &gis_0_4_1 + &gis_0_5_1 + &gis_0_9_1 + &gis_0_a_1 + &gis_0_b_1).leq(&(2*&goss_0_0_1)),

        (&gis_0_3_2 + &gis_0_4_2 + &gis_0_5_2 + &gis_0_9_2 + &gis_0_a_2 + &gis_0_b_2).geq(&(1*&goss_0_0_2)),
        (&gis_0_3_2 + &gis_0_4_2 + &gis_0_5_2 + &gis_0_9_2 + &gis_0_a_2 + &gis_0_b_2).leq(&(2*&goss_0_0_2)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn balancing_timeslots_with_partial_last_period() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(3).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let slots = vec![
        SlotWithTeacher {
            teacher: 0,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
            cost: 0,
        },
        SlotWithTeacher {
            teacher: 0,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
            cost: 0,
        },
        SlotWithTeacher {
            teacher: 0,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
            cost: 0,
        },
        SlotWithTeacher {
            teacher: 1,
            start: SlotStart {
                week: 0,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
            cost: 0,
        },
        SlotWithTeacher {
            teacher: 1,
            start: SlotStart {
                week: 1,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
            cost: 0,
        },
        SlotWithTeacher {
            teacher: 1,
            start: SlotStart {
                week: 2,
                weekday: time::Weekday::Tuesday,
                start_time: time::Time::from_hm(8, 0).unwrap(),
            },
            cost: 0,
        },
    ];

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: false,
        balancing_requirements: BalancingRequirements {
            constraints: BalancingConstraints::OverallOnly,
            slot_selections: BalancingRequirements::balance_timeslots_from_slots(&slots),
        },
        duration: NonZeroU32::new(60).unwrap(),
        slots,
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
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
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
    let goss_0_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_0_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_0_0_2 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 2 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0).geq(&(0*&goss_0_0_0)),
        (&gis_0_0_0 + &gis_0_1_0 + &gis_0_2_0).leq(&(1*&goss_0_0_0)),

        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1).geq(&(0*&goss_0_0_1)),
        (&gis_0_0_1 + &gis_0_1_1 + &gis_0_2_1).leq(&(1*&goss_0_0_1)),

        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2).geq(&(0*&goss_0_0_2)),
        (&gis_0_0_2 + &gis_0_1_2 + &gis_0_2_2).leq(&(1*&goss_0_0_2)),

        (&gis_0_3_0 + &gis_0_4_0 + &gis_0_5_0).geq(&(0*&goss_0_0_0)),
        (&gis_0_3_0 + &gis_0_4_0 + &gis_0_5_0).leq(&(1*&goss_0_0_0)),

        (&gis_0_3_1 + &gis_0_4_1 + &gis_0_5_1).geq(&(0*&goss_0_0_1)),
        (&gis_0_3_1 + &gis_0_4_1 + &gis_0_5_1).leq(&(1*&goss_0_0_1)),

        (&gis_0_3_2 + &gis_0_4_2 + &gis_0_5_2).geq(&(0*&goss_0_0_2)),
        (&gis_0_3_2 + &gis_0_4_2 + &gis_0_5_2).leq(&(1*&goss_0_0_2)),
    ]);

    assert_eq!(balancing_constraints, expected_result);
}

#[test]
fn student_incompat_max_count_constraints() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
                cost: 0,
            },
            SlotWithTeacher {
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
                cost: 0,
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
    let incompatibility_groups = IncompatibilityGroupList::from([
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
            ]),
        },
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
            ]),
        },
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
            ]),
        },
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(14, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(120).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(14, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(120).unwrap(),
                },
            ]),
        },
    ]);
    let incompatibilities = IncompatibilityList::from([
        Incompatibility {
            groups: BTreeSet::from([0, 1]),
            max_count: 1,
        },
        Incompatibility {
            groups: BTreeSet::from([2]),
            max_count: 0,
        },
        Incompatibility {
            groups: BTreeSet::from([2, 3]),
            max_count: 0,
        },
    ]);
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
        Student {
            incompatibilities: BTreeSet::from([1]),
        },
        Student {
            incompatibilities: BTreeSet::from([2]),
        },
        Student {
            incompatibilities: BTreeSet::from([0, 1]),
        },
        Student {
            incompatibilities: BTreeSet::from([0, 1, 2]),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibility_groups,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let student_incompat_max_count_constraints =
        ilp_translator.build_student_incompat_max_count_constraints();

    #[rustfmt::skip]
    let igfs_0_1 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 0, student: 1 });
    #[rustfmt::skip]
    let igfs_1_1 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 1, student: 1 });
    #[rustfmt::skip]
    let igfs_2_2 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 2, student: 2 });
    #[rustfmt::skip]
    let igfs_2_3 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 2, student: 3 });
    #[rustfmt::skip]
    let igfs_3_3 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 3, student: 3 });
    #[rustfmt::skip]
    let igfs_0_4 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 0, student: 4 });
    #[rustfmt::skip]
    let igfs_1_4 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 1, student: 4 });
    #[rustfmt::skip]
    let igfs_2_4 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 2, student: 4 });
    #[rustfmt::skip]
    let igfs_0_5 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 0, student: 5 });
    #[rustfmt::skip]
    let igfs_1_5 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 1, student: 5 });
    #[rustfmt::skip]
    let igfs_2_5 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 2, student: 5 });
    #[rustfmt::skip]
    let igfs_3_5 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 3, student: 5 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        (&igfs_0_1 + &igfs_1_1).leq(&Expr::constant(1)),
        (&igfs_0_4 + &igfs_1_4).leq(&Expr::constant(1)),
        (&igfs_0_5 + &igfs_1_5).leq(&Expr::constant(1)),

        igfs_2_2.leq(&Expr::constant(0)),
        igfs_2_4.leq(&Expr::constant(0)),
        igfs_2_5.leq(&Expr::constant(0)),

        (&igfs_2_3 + &igfs_3_3).leq(&Expr::constant(0)),
        (&igfs_2_5 + &igfs_3_5).leq(&Expr::constant(0)),
    ]);

    assert_eq!(student_incompat_max_count_constraints, expected_result);
}

#[test]
fn incompat_group_for_student_constraints() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
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
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(13, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Wednesday,
                    start_time: time::Time::from_hm(12, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
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
    let incompatibility_groups = IncompatibilityGroupList::from([
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(12, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
            ]),
        },
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Wednesday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Thursday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(13, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Friday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
            ]),
        },
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(8, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Monday,
                    },
                    duration: NonZeroU32::new(60).unwrap(),
                },
            ]),
        },
        IncompatibilityGroup {
            slots: BTreeSet::from([
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(14, 0).unwrap(),
                        week: 0,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(120).unwrap(),
                },
                SlotWithDuration {
                    start: SlotStart {
                        start_time: time::Time::from_hm(14, 0).unwrap(),
                        week: 1,
                        weekday: time::Weekday::Tuesday,
                    },
                    duration: NonZeroU32::new(120).unwrap(),
                },
            ]),
        },
    ]);
    let incompatibilities = IncompatibilityList::from([
        Incompatibility {
            groups: BTreeSet::from([0, 1]),
            max_count: 1,
        },
        Incompatibility {
            groups: BTreeSet::from([2]),
            max_count: 0,
        },
        Incompatibility {
            groups: BTreeSet::from([2, 3]),
            max_count: 0,
        },
    ]);
    let students = vec![
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::from([0]),
        },
        Student {
            incompatibilities: BTreeSet::from([1]),
        },
        Student {
            incompatibilities: BTreeSet::from([2]),
        },
        Student {
            incompatibilities: BTreeSet::from([0, 1]),
        },
        Student {
            incompatibilities: BTreeSet::from([0, 1, 2]),
        },
    ];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = SlotGroupingIncompatSet::new();

    let data = ValidatedData::new(
        general,
        subjects,
        incompatibility_groups,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let incompat_group_for_student_constraints =
        ilp_translator.build_incompat_group_for_student_constraints();

    #[rustfmt::skip]
    let igfs_0_1 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 0, student: 1 });
    #[rustfmt::skip]
    let igfs_1_1 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 1, student: 1 });
    #[rustfmt::skip]
    let igfs_2_2 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 2, student: 2 });
    #[rustfmt::skip]
    let igfs_2_3 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 2, student: 3 });
    #[rustfmt::skip]
    let igfs_0_4 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 0, student: 4 });
    #[rustfmt::skip]
    let igfs_1_4 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 1, student: 4 });
    #[rustfmt::skip]
    let igfs_2_4 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 2, student: 4 });
    #[rustfmt::skip]
    let igfs_0_5 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 0, student: 5 });
    #[rustfmt::skip]
    let igfs_1_5 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 1, student: 5 });
    #[rustfmt::skip]
    let igfs_2_5 = Expr::<Variable>::var(Variable::IncompatGroupForStudent { incompat_group: 2, student: 5 });

    #[rustfmt::skip]
    let gis_0_0_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 0, group: 0 });
    #[rustfmt::skip]
    let gis_0_2_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 2, group: 0 });
    #[rustfmt::skip]
    let gis_0_3_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 3, group: 0 });
    #[rustfmt::skip]
    let gis_0_4_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 4, group: 0 });
    #[rustfmt::skip]
    let gis_0_6_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 6, group: 0 });
    #[rustfmt::skip]
    let gis_0_7_0 = Expr::<Variable>::var(Variable::GroupInSlot { subject: 0, slot: 7, group: 0 });

    #[rustfmt::skip]
    let dga_0_0_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_0_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_0_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 0, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_2_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_2_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 2, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_3_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_3_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 3, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_4_1_3 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 1, student: 3 });
    #[rustfmt::skip]
    let dga_0_4_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_4_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 4, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_6_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_6_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 6, group: 1, student: 5 });
    #[rustfmt::skip]
    let dga_0_7_1_4 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 1, student: 4 });
    #[rustfmt::skip]
    let dga_0_7_1_5 = Expr::<Variable>::var(Variable::DynamicGroupAssignment { subject: 0, slot: 7, group: 1, student: 5 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        gis_0_2_0.leq(&igfs_0_1),
        gis_0_6_0.leq(&igfs_0_1),

        gis_0_3_0.leq(&igfs_1_1),
        gis_0_7_0.leq(&igfs_1_1),

        gis_0_0_0.leq(&igfs_2_2),
        gis_0_4_0.leq(&igfs_2_2),

        dga_0_0_1_3.leq(&igfs_2_3),
        dga_0_4_1_3.leq(&igfs_2_3),

        dga_0_2_1_4.leq(&igfs_0_4),
        dga_0_6_1_4.leq(&igfs_0_4),

        dga_0_3_1_4.leq(&igfs_1_4),
        dga_0_7_1_4.leq(&igfs_1_4),

        dga_0_0_1_4.leq(&igfs_2_4),
        dga_0_4_1_4.leq(&igfs_2_4),

        dga_0_2_1_5.leq(&igfs_0_5),
        dga_0_6_1_5.leq(&igfs_0_5),

        dga_0_3_1_5.leq(&igfs_1_5),
        dga_0_7_1_5.leq(&igfs_1_5),

        dga_0_0_1_5.leq(&igfs_2_5),
        dga_0_4_1_5.leq(&igfs_2_5),
    ]);

    assert_eq!(incompat_group_for_student_constraints, expected_result);
}

#[test]
fn group_on_slot_selection_constraints() {
    let general = GeneralData {
        periodicity_cuts: BTreeSet::new(),
        teacher_count: 2,
        week_count: NonZeroU32::new(4).unwrap(),
        interrogations_per_week: None,
        max_interrogations_per_day: None,
    };

    let subjects = vec![Subject {
        students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
        max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        period_is_strict: true,
        duration: NonZeroU32::new(60).unwrap(),
        balancing_requirements: BalancingRequirements {
            constraints: BalancingConstraints::Strict,
            slot_selections: vec![
                BalancingSlotSelection {
                    slot_groups: vec![
                        BalancingSlotGroup {
                            slots: BTreeSet::from([0, 2]),
                            count: 1,
                        },
                        BalancingSlotGroup {
                            slots: BTreeSet::from([4, 6]),
                            count: 1,
                        },
                    ],
                },
                BalancingSlotSelection {
                    slot_groups: vec![
                        BalancingSlotGroup {
                            slots: BTreeSet::from([1, 3]),
                            count: 1,
                        },
                        BalancingSlotGroup {
                            slots: BTreeSet::from([5, 7]),
                            count: 1,
                        },
                    ],
                },
            ],
        },
        slots: vec![
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 0,
                start: SlotStart {
                    week: 3,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 1,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 2,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                },
            },
            SlotWithTeacher {
                cost: 0,
                teacher: 1,
                start: SlotStart {
                    week: 3,
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
                    students: BTreeSet::from([]),
                    can_be_extended: true,
                },
            ],
            not_assigned: BTreeSet::from([3, 4, 5]),
        },
        ..Subject::default()
    }];
    let incompatibility_groups = IncompatibilityGroupList::new();
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
        incompatibility_groups,
        incompatibilities,
        students,
        slot_groupings,
        grouping_incompats,
    )
    .unwrap();

    let ilp_translator = data.ilp_translator();
    let group_on_slot_selection_constraints =
        ilp_translator.build_group_on_slot_selection_constraints();

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
    let goss_0_0_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 0 });
    #[rustfmt::skip]
    let goss_0_0_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 0, group: 1 });
    #[rustfmt::skip]
    let goss_0_1_0 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 1, group: 0 });
    #[rustfmt::skip]
    let goss_0_1_1 = Expr::<Variable>::var(Variable::GroupOnSlotSelection { subject: 0, slot_selection: 1, group: 1 });

    #[rustfmt::skip]
    let expected_result = BTreeSet::from([
        gis_0_0_0.leq(&goss_0_0_0),
        gis_0_2_0.leq(&goss_0_0_0),
        gis_0_4_0.leq(&goss_0_0_0),
        gis_0_6_0.leq(&goss_0_0_0),
        (&gis_0_0_0 + &gis_0_2_0 + &gis_0_4_0 + &gis_0_6_0).geq(&goss_0_0_0),

        gis_0_0_1.leq(&goss_0_0_1),
        gis_0_2_1.leq(&goss_0_0_1),
        gis_0_4_1.leq(&goss_0_0_1),
        gis_0_6_1.leq(&goss_0_0_1),
        (&gis_0_0_1 + &gis_0_2_1 + &gis_0_4_1 + &gis_0_6_1).geq(&goss_0_0_1),

        gis_0_1_0.leq(&goss_0_1_0),
        gis_0_3_0.leq(&goss_0_1_0),
        gis_0_5_0.leq(&goss_0_1_0),
        gis_0_7_0.leq(&goss_0_1_0),
        (&gis_0_1_0 + &gis_0_3_0 + &gis_0_5_0 + &gis_0_7_0).geq(&goss_0_1_0),

        gis_0_1_1.leq(&goss_0_1_1),
        gis_0_3_1.leq(&goss_0_1_1),
        gis_0_5_1.leq(&goss_0_1_1),
        gis_0_7_1.leq(&goss_0_1_1),
        (&gis_0_1_1 + &gis_0_3_1 + &gis_0_5_1 + &gis_0_7_1).geq(&goss_0_1_1),

        (&goss_0_0_0 + &goss_0_1_0).eq(&Expr::constant(1)),
        (&goss_0_0_1 + &goss_0_1_1).eq(&Expr::constant(1)),
    ]);

    assert_eq!(group_on_slot_selection_constraints, expected_result);
}
