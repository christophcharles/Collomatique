use collomatique::gen::colloscope::*;
use collomatique::gen::time;

use std::collections::BTreeSet;
use std::num::{NonZeroU32, NonZeroUsize};

fn main() {
    let general = GeneralData {
        teacher_count: 5,
        week_count: NonZeroU32::new(2).unwrap(),
        interrogations_per_week: Some(2..3),
    };

    let subjects = vec![
        Subject {
            students_per_slot: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(1).unwrap(),
            period_is_strict: false,
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
                        students: BTreeSet::from([7, 8]),
                        can_be_extended: true,
                    },
                    GroupDesc {
                        students: BTreeSet::from([10, 11]),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([6, 9]),
            },
        },
        Subject {
            students_per_slot: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
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
                        students: BTreeSet::from([7, 8]),
                        can_be_extended: true,
                    },
                    GroupDesc {
                        students: BTreeSet::from([10, 11]),
                        can_be_extended: true,
                    },
                ],
                not_assigned: BTreeSet::from([6, 9]),
            },
        },
        Subject {
            students_per_slot: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
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
            students_per_slot: NonZeroUsize::new(2).unwrap()..=NonZeroUsize::new(3).unwrap(),
            period: NonZeroU32::new(2).unwrap(),
            period_is_strict: false,
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

    println!("{}", problem);

    let mut sa_optimizer = collomatique::ilp::optimizers::sa::Optimizer::new(&problem);

    let mut random_gen = collomatique::ilp::random::DefaultRndGen::new();

    sa_optimizer.set_init_config(problem.random_config(&mut random_gen));
    sa_optimizer.set_max_steps(Some(1000));

    use collomatique::ilp::solvers::backtracking::heuristics::Knuth2000;
    let solver = collomatique::ilp::solvers::backtracking::Solver::new(Knuth2000::default());
    let iterator = sa_optimizer.iterate(solver, &mut random_gen);

    for (i, (sol, cost)) in iterator.enumerate() {
        println!(
            "{}: {} - {:?}",
            i,
            cost,
            ilp_translator.read_solution(sol.as_ref())
        );
    }
}
