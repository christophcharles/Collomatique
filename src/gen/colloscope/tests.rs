use super::*;

#[test]
fn trivial_validated_data() {
    let general = GeneralData {
        teacher_count: 0,
        week_count: NonZeroU32::new(1).unwrap(),
    };

    let subjects = SubjectList::new();
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();

    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = GroupingIncompatSet::new();

    let expected_result = ValidatedData {
        general: general.clone(),
        subjects: subjects.clone(),
        incompatibilities: incompatibilities.clone(),
        students: students.clone(),
        slot_groupings: slot_groupings.clone(),
        grouping_incompats: grouping_incompats.clone(),
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
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 0).unwrap(),
            }],
        }],
    }];
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
        subjects: BTreeSet::from([0]),
        incompatibilities: BTreeSet::from([0]),
    }];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = GroupingIncompatSet::new();

    let expected_result = ValidatedData {
        general: general.clone(),
        subjects: subjects.clone(),
        incompatibilities: incompatibilities.clone(),
        students: students.clone(),
        slot_groupings: slot_groupings.clone(),
        grouping_incompats: grouping_incompats.clone(),
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
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(1).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(0, 0).unwrap(),
            }],
        }],
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = GroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::InvalidStudentsPerInterrogationRange)
    );
}

#[test]
fn slot_overlaps_next_day() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 1).unwrap(),
            }],
        }],
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = GroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SlotOverlapsNextDay)
    );

    let general = GeneralData {
        teacher_count: 0,
        week_count: NonZeroU32::new(1).unwrap(),
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
    let grouping_incompats = GroupingIncompatSet::new();
    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::SlotOverlapsNextDay)
    );
}

#[test]
fn invalid_teacher_number() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 1,
            slots: vec![SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 0).unwrap(),
            }],
        }],
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = StudentList::new();
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = GroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::InvalidTeacherNumber)
    );
}

#[test]
fn invalid_subject_number() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
    };

    let subjects = vec![Subject {
        students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
        period: NonZeroU32::new(2).unwrap(),
        duration: NonZeroU32::new(60).unwrap(),
        interrogations: vec![Interrogation {
            teacher: 0,
            slots: vec![SlotStart {
                week: 0,
                weekday: time::Weekday::Monday,
                start_time: time::Time::from_hm(23, 0).unwrap(),
            }],
        }],
    }];
    let incompatibilities = IncompatibilityList::new();
    let students = vec![Student {
        subjects: BTreeSet::from([1]),
        incompatibilities: BTreeSet::new(),
    }];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = GroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::InvalidSubjectNumber)
    );
}

#[test]
fn invalid_incompatibility_number() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
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
        subjects: BTreeSet::new(),
        incompatibilities: BTreeSet::from([1]),
    }];
    let slot_groupings = SlotGroupingList::new();
    let grouping_incompats = GroupingIncompatSet::new();

    assert_eq!(
        ValidatedData::new(
            general,
            subjects,
            incompatibilities,
            students,
            slot_groupings,
            grouping_incompats
        ),
        Err(Error::InvalidIncompatibilityNumber)
    );
}

#[test]
fn slot_ref_has_invalid_subject() {
    let general = GeneralData {
        teacher_count: 1,
        week_count: NonZeroU32::new(1).unwrap(),
    };

    let subjects = vec![
        Subject {
            students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                }],
            }],
        },
        Subject {
            students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                }],
            }],
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
    let grouping_incompats = GroupingIncompatSet::new();

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
    };

    let subjects = vec![
        Subject {
            students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                }],
            }],
        },
        Subject {
            students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                }],
            }],
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
    let grouping_incompats = GroupingIncompatSet::new();

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
    };

    let subjects = vec![
        Subject {
            students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Monday,
                    start_time: time::Time::from_hm(17, 0).unwrap(),
                }],
            }],
        },
        Subject {
            students_per_interrogation: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            duration: NonZeroU32::new(60).unwrap(),
            interrogations: vec![Interrogation {
                teacher: 0,
                slots: vec![SlotStart {
                    week: 0,
                    weekday: time::Weekday::Tuesday,
                    start_time: time::Time::from_hm(8, 0).unwrap(),
                }],
            }],
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
    let grouping_incompats = GroupingIncompatSet::new();

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
