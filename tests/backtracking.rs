#[test]
fn restricted_interrogations_per_week() {
    use collomatique::gen::colloscope::*;
    use collomatique::gen::time;

    use std::collections::BTreeSet;
    use std::num::{NonZeroU32, NonZeroUsize};

    let general = GeneralData {
        teacher_count: 5,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: Some(2..3),
    };

    let subjects = vec![
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(1).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
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
                        students: BTreeSet::from([6, 7, 8]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([9, 10, 11]),
                        can_be_extended: false,
                    },
                ],
                not_assigned: BTreeSet::new(),
            },
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 2,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Wednesday,
                        start_time: time::Time::from_hm(14, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 2,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Wednesday,
                        start_time: time::Time::from_hm(15, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 2,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Wednesday,
                        start_time: time::Time::from_hm(14, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 2,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Wednesday,
                        start_time: time::Time::from_hm(15, 0).unwrap(),
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
                        students: BTreeSet::from([6, 7, 8]),
                        can_be_extended: false,
                    },
                    GroupDesc {
                        students: BTreeSet::from([9, 10, 11]),
                        can_be_extended: false,
                    },
                ],
                not_assigned: BTreeSet::new(),
            },
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 3,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Thursday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 3,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Thursday,
                        start_time: time::Time::from_hm(18, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 3,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Thursday,
                        start_time: time::Time::from_hm(17, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 3,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Thursday,
                        start_time: time::Time::from_hm(18, 0).unwrap(),
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
                        students: BTreeSet::from([6, 10, 11]),
                        can_be_extended: false,
                    },
                ],
                not_assigned: BTreeSet::new(),
            },
        },
        Subject {
            students_per_group: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            max_groups_per_slot: NonZeroUsize::new(1).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
            is_tutorial: false,
            duration: NonZeroU32::new(60).unwrap(),
            slots: vec![
                SlotWithTeacher {
                    teacher: 4,
                    start: SlotStart {
                        week: 0,
                        weekday: time::Weekday::Wednesday,
                        start_time: time::Time::from_hm(14, 0).unwrap(),
                    },
                },
                SlotWithTeacher {
                    teacher: 4,
                    start: SlotStart {
                        week: 1,
                        weekday: time::Weekday::Wednesday,
                        start_time: time::Time::from_hm(14, 0).unwrap(),
                    },
                },
            ],
            groups: GroupsDesc {
                prefilled_groups: vec![GroupDesc {
                    students: BTreeSet::from([7, 8, 9]),
                    can_be_extended: false,
                }],
                not_assigned: BTreeSet::new(),
            },
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
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
        Student {
            incompatibilities: BTreeSet::new(),
        },
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

    use collomatique::ilp::solvers::backtracking::heuristics::Knuth2000;
    let solver = collomatique::ilp::solvers::backtracking::Solver::new(Knuth2000::default());

    use collomatique::gen::colloscope::Variable;
    // Through testing this one caused issue when trying to put GiS_2_0_2 to 1
    /* GiS_0_0_0: 0, GiS_0_0_1: 0, GiS_0_0_2: 1, GiS_0_0_3: 0, GiS_0_1_0: 0, GiS_0_1_1: 1, GiS_0_1_2: 0,
    GiS_0_1_3: 0, GiS_0_2_0: 0, GiS_0_2_1: 0, GiS_0_2_2: 0, GiS_0_2_3: 1, GiS_0_3_0: 1, GiS_0_3_1: 0,
    GiS_0_3_2: 0, GiS_0_3_3: 0, GiS_0_4_0: 0, GiS_0_4_1: 0, GiS_0_4_2: 1, GiS_0_4_3: 0, GiS_0_5_0: 0,
    GiS_0_5_1: 1, GiS_0_5_2: 0, GiS_0_5_3: 0, GiS_0_6_0: 1, GiS_0_6_1: 0, GiS_0_6_2: 0, GiS_0_6_3: 0,
    GiS_0_7_0: 0, GiS_0_7_1: 0, GiS_0_7_2: 0, GiS_0_7_3: 1, GiS_1_0_0: 0, GiS_1_0_1: 0, GiS_1_0_2: 0,
    GiS_1_0_3: 1, GiS_1_1_0: 0, GiS_1_1_1: 0, GiS_1_1_2: 1, GiS_1_1_3: 0, GiS_1_2_0: 1, GiS_1_2_1: 0,
    GiS_1_2_2: 0, GiS_1_2_3: 0, GiS_1_3_0: 0, GiS_1_3_1: 1, GiS_1_3_2: 0, GiS_1_3_3: 0, GiS_2_0_0: 1,
    GiS_2_0_1: 0, GiS_2_0_2: 0, GiS_2_1_0: 0, GiS_2_1_1: 1, GiS_2_1_2: 0, GiS_2_2_0: 0, GiS_2_2_1: 0,
    GiS_2_2_2: 0, GiS_2_3_0: 0, GiS_2_3_1: 0, GiS_2_3_2: 1, GiS_3_0_0: 0, GiS_3_1_0: 1 */
    let config = problem
        .config_from([
            &Variable::GroupInSlot {
                subject: 0,
                slot: 0,
                group: 2,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 1,
                group: 1,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 2,
                group: 3,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 3,
                group: 0,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 4,
                group: 2,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 5,
                group: 1,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 6,
                group: 0,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 7,
                group: 3,
            },
            &Variable::GroupInSlot {
                subject: 1,
                slot: 0,
                group: 3,
            },
            &Variable::GroupInSlot {
                subject: 1,
                slot: 1,
                group: 2,
            },
            &Variable::GroupInSlot {
                subject: 1,
                slot: 2,
                group: 0,
            },
            &Variable::GroupInSlot {
                subject: 1,
                slot: 3,
                group: 1,
            },
            &Variable::GroupInSlot {
                subject: 2,
                slot: 0,
                group: 0,
            },
            &Variable::GroupInSlot {
                subject: 2,
                slot: 1,
                group: 1,
            },
            &Variable::GroupInSlot {
                subject: 2,
                slot: 3,
                group: 2,
            },
            &Variable::GroupInSlot {
                subject: 3,
                slot: 1,
                group: 0,
            },
        ])
        .unwrap();
    let origin_config = config.into_feasable().unwrap();

    let mut config2 = origin_config.inner().clone();
    config2
        .set(
            &Variable::GroupInSlot {
                subject: 2,
                slot: 0,
                group: 2,
            },
            true,
        )
        .unwrap();

    // Knuth heuristic should give this :
    /* GiS_0_0_0: 0, GiS_0_0_1: 0, GiS_0_0_2: 1, GiS_0_0_3: 0, GiS_0_1_0: 0, GiS_0_1_1: 1, GiS_0_1_2: 0,
    GiS_0_1_3: 0, GiS_0_2_0: 0, GiS_0_2_1: 0, GiS_0_2_2: 0, GiS_0_2_3: 1, GiS_0_3_0: 1, GiS_0_3_1: 0,
    GiS_0_3_2: 0, GiS_0_3_3: 0, GiS_0_4_0: 0, GiS_0_4_1: 0, GiS_0_4_2: 1, GiS_0_4_3: 0, GiS_0_5_0: 0,
    GiS_0_5_1: 1, GiS_0_5_2: 0, GiS_0_5_3: 0, GiS_0_6_0: 1, GiS_0_6_1: 0, GiS_0_6_2: 0, GiS_0_6_3: 0,
    GiS_0_7_0: 0, GiS_0_7_1: 0, GiS_0_7_2: 0, GiS_0_7_3: 1, GiS_1_0_0: 1, GiS_1_0_1: 0, GiS_1_0_2: 0,
    GiS_1_0_3: 0, GiS_1_1_0: 0, GiS_1_1_1: 1, GiS_1_1_2: 0, GiS_1_1_3: 0, GiS_1_2_0: 0, GiS_1_2_1: 0,
    GiS_1_2_2: 0, GiS_1_2_3: 1, GiS_1_3_0: 0, GiS_1_3_1: 0, GiS_1_3_2: 1, GiS_1_3_3: 0, GiS_2_0_0: 0,
    GiS_2_0_1: 0, GiS_2_0_2: 1, GiS_2_1_0: 0, GiS_2_1_1: 0, GiS_2_1_2: 0, GiS_2_2_0: 0, GiS_2_2_1: 1,
    GiS_2_2_2: 0, GiS_2_3_0: 1, GiS_2_3_1: 0, GiS_2_3_2: 0, GiS_3_0_0: 1, GiS_3_1_0: 0 */
    // I'm not quite sure to what extent this is dependent on the precise implementation but anyway,
    // this test should avoid regression
    let config3 = problem
        .config_from([
            &Variable::GroupInSlot {
                subject: 0,
                slot: 0,
                group: 2,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 1,
                group: 1,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 2,
                group: 3,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 3,
                group: 0,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 4,
                group: 2,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 5,
                group: 1,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 6,
                group: 0,
            },
            &Variable::GroupInSlot {
                subject: 0,
                slot: 7,
                group: 3,
            },
            &Variable::GroupInSlot {
                subject: 1,
                slot: 0,
                group: 0,
            },
            &Variable::GroupInSlot {
                subject: 1,
                slot: 1,
                group: 1,
            },
            &Variable::GroupInSlot {
                subject: 1,
                slot: 2,
                group: 3,
            },
            &Variable::GroupInSlot {
                subject: 1,
                slot: 3,
                group: 2,
            },
            &Variable::GroupInSlot {
                subject: 2,
                slot: 0,
                group: 2,
            },
            &Variable::GroupInSlot {
                subject: 2,
                slot: 2,
                group: 1,
            },
            &Variable::GroupInSlot {
                subject: 2,
                slot: 3,
                group: 0,
            },
            &Variable::GroupInSlot {
                subject: 3,
                slot: 0,
                group: 0,
            },
        ])
        .unwrap();
    let expected_result = config3.into_feasable().unwrap();

    use collomatique::ilp::solvers::FeasabilitySolver;

    let result = solver.restore_feasability_with_origin(&config2, Some(&origin_config));

    assert_eq!(result.expect("Should find a solution"), expected_result);
}
