//! Deterministic builder for a fully populated document
//!
//! The document is built through the checked op path (no raw `InnerData`
//! literals, no unchecked ids), so it is guaranteed valid. Every
//! serialized section is non-trivially populated; the asserts at the end
//! guard against the script silently degrading as the model evolves.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    AssignmentOp, BalancingOp, ColloscopeOp, Data, ExportConfigOp, GroupListOp, IncompatOp, NewId,
    NonEmptyRangeInclusive, Op, PairingOp, PeriodOp, PersonWithContact, SettingsOp, SlotOp,
    SlotPairingOp, StudentOp, Subject, SubjectInterrogationParameters, SubjectOp,
    SubjectParameters, SubjectPeriodicity, TeacherOp, WeekOp, WeekPatternOp,
    balancing::{Balancing, BalancingOptions},
    export_config,
    group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup},
    ids::{PeriodId, WeekId},
    incompats::Incompatibility,
    pairings::{PairingRule, RulePart},
    settings::Limits,
    slot_pairings::{SlotPairingRule, SlotRulePart},
    slots::Slot,
    soft_param::SoftParam,
    students::Student,
    subjects::WeekBlock,
    teachers::Teacher,
    week_patterns::WeekPattern,
    weeks::WeekDesc,
};
use collomatique_time::{
    NonZeroMinutes, SlotStart, SlotWithDuration, WeekStart, Weekday, WholeMinuteTime,
};

fn non_empty(text: &str) -> non_empty_string::NonEmptyString {
    non_empty_string::NonEmptyString::new(text.to_string()).expect("Text should be non-empty")
}

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("Value should be non-zero")
}

fn ner<T: Ord + Clone>(range: std::ops::RangeInclusive<T>) -> NonEmptyRangeInclusive<T> {
    NonEmptyRangeInclusive::new(range).expect("Range should be non-empty")
}

fn person(
    surname: &str,
    firstname: &str,
    tel: Option<&str>,
    email: Option<&str>,
) -> PersonWithContact {
    PersonWithContact {
        surname: surname.to_string(),
        firstname: firstname.to_string(),
        tel: tel.map(non_empty),
        email: email.map(non_empty),
    }
}

fn slot_start(weekday: chrono::Weekday, hour: u32, minute: u32) -> SlotStart {
    SlotStart {
        weekday: Weekday(weekday),
        start_time: WholeMinuteTime::new(
            chrono::NaiveTime::from_hms_opt(hour, minute, 0).expect("Time should be valid"),
        )
        .expect("Time should be on a whole minute"),
    }
}

fn incompat_slot(
    weekday: chrono::Weekday,
    hour: u32,
    minute: u32,
    duration: u32,
) -> SlotWithDuration {
    SlotWithDuration::new(
        slot_start(weekday, hour, minute),
        NonZeroMinutes::new(duration).expect("Duration should be non-zero"),
    )
    .expect("Slot should not cross midnight")
}

fn apply(state: &mut AppState<Data, String>, op: Op, desc: &str) -> Option<NewId> {
    state
        .apply(op, desc.to_string())
        .unwrap_or_else(|e| panic!("build_rich_data op `{desc}` failed: {e}"))
}

macro_rules! apply_new_id {
    ($state:expr, $op:expr, $desc:expr, $variant:ident) => {{
        let Some(NewId::$variant(id)) = apply($state, $op, $desc) else {
            panic!("op `{}` should return a {} id", $desc, stringify!($variant));
        };
        id
    }};
}

/// Creates a period (front, or after `after`) carrying `weeks`, one spliced in
/// at a time via the `WeekOp` family — periods are created empty.
fn add_period(
    state: &mut AppState<Data, String>,
    after: Option<PeriodId>,
    weeks: Vec<WeekDesc>,
) -> PeriodId {
    let period = match after {
        None => apply_new_id!(
            state,
            Op::Period(PeriodOp::AddFront),
            "add period",
            PeriodId
        ),
        Some(a) => apply_new_id!(
            state,
            Op::Period(PeriodOp::AddAfter(a)),
            "add period",
            PeriodId
        ),
    };
    let mut prev: Option<WeekId> = None;
    for desc in weeks {
        let op = match prev {
            None => WeekOp::AddFront(period, desc),
            Some(w) => WeekOp::AddAfter(w, desc),
        };
        prev = Some(apply_new_id!(state, Op::Week(op), "add week", WeekId));
    }
    period
}

/// Builds a document where every serialized section is non-trivially populated
pub fn build_rich_data() -> Data {
    let mut state = AppState::<_, String>::new(Data::new());

    // Periods: 3 weeks + 4 weeks (7 weeks total), with annotations and
    // non-interrogation weeks on both periods
    apply(
        &mut state,
        Op::Period(PeriodOp::ChangeStartDate(Some(WeekStart::round_from(
            chrono::NaiveDate::from_ymd_opt(2026, 8, 31).expect("Date should be valid"),
        )))),
        "start date",
    );
    let period1 = add_period(
        &mut state,
        None,
        vec![
            WeekDesc {
                interrogations: true,
                annotation: Some(non_empty("Rentrée")),
            },
            WeekDesc::new(true),
            WeekDesc::new(false),
        ],
    );
    let period2 = add_period(
        &mut state,
        Some(period1),
        vec![
            WeekDesc::new(true),
            WeekDesc::new(true),
            WeekDesc {
                interrogations: false,
                annotation: Some(non_empty("Vacances")),
            },
            WeekDesc::new(true),
        ],
    );

    // Students: with and without contact info, one excluded from a period
    let student1 = apply_new_id!(
        &mut state,
        Op::Student(StudentOp::Add(Student {
            desc: person(
                "Potter",
                "Harry",
                Some("0601020304"),
                Some("harry@poudlard.fr")
            ),
            excluded_periods: BTreeSet::new(),
        })),
        "student 1",
        StudentId
    );
    let student2 = apply_new_id!(
        &mut state,
        Op::Student(StudentOp::Add(Student {
            desc: person("Granger", "Hermione", None, Some("hermione@poudlard.fr")),
            excluded_periods: BTreeSet::from([period2]),
        })),
        "student 2",
        StudentId
    );
    let student3 = apply_new_id!(
        &mut state,
        Op::Student(StudentOp::Add(Student {
            desc: person("Weasley", "Ron", None, None),
            excluded_periods: BTreeSet::new(),
        })),
        "student 3",
        StudentId
    );
    let student4 = apply_new_id!(
        &mut state,
        Op::Student(StudentOp::Add(Student {
            desc: person("Lovegood", "Luna", None, None),
            excluded_periods: BTreeSet::new(),
        })),
        "student 4",
        StudentId
    );

    // Subjects: the four interrogation-bearing ones cover the four
    // periodicity variants; one has an excluded period; one has no
    // interrogation parameters at all
    let subject_maths = apply_new_id!(
        &mut state,
        Op::Subject(SubjectOp::AddAfter(
            None,
            Subject {
                parameters: SubjectParameters {
                    name: "Mathématiques".to_string(),
                    interrogation_parameters: Some(SubjectInterrogationParameters {
                        students_per_group: ner(nz(2)..=nz(3)),
                        groups_per_interrogation: ner(nz(1)..=nz(1)),
                        duration: NonZeroMinutes::new(60).expect("Duration should be non-zero"),
                        take_duration_into_account: true,
                        periodicity: SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                            weeks_per_block: nz(2),
                            minimum_week_separation: nz(1),
                        },
                    }),
                },
                excluded_periods: BTreeSet::new(),
            },
        )),
        "subject maths",
        SubjectId
    );
    let subject_physics = apply_new_id!(
        &mut state,
        Op::Subject(SubjectOp::AddAfter(
            Some(subject_maths),
            Subject {
                parameters: SubjectParameters {
                    name: "Physique-Chimie".to_string(),
                    interrogation_parameters: Some(SubjectInterrogationParameters {
                        students_per_group: ner(nz(1)..=nz(3)),
                        groups_per_interrogation: ner(nz(1)..=nz(2)),
                        duration: NonZeroMinutes::new(30).expect("Duration should be non-zero"),
                        take_duration_into_account: false,
                        periodicity: SubjectPeriodicity::ExactlyPeriodic {
                            periodicity_in_weeks: nz(2),
                        },
                    }),
                },
                excluded_periods: BTreeSet::from([period2]),
            },
        )),
        "subject physics",
        SubjectId
    );
    let subject_english = apply_new_id!(
        &mut state,
        Op::Subject(SubjectOp::AddAfter(
            Some(subject_physics),
            Subject {
                parameters: SubjectParameters {
                    name: "Anglais".to_string(),
                    interrogation_parameters: Some(SubjectInterrogationParameters {
                        students_per_group: ner(nz(2)..=nz(2)),
                        groups_per_interrogation: ner(nz(1)..=nz(1)),
                        duration: NonZeroMinutes::new(30).expect("Duration should be non-zero"),
                        take_duration_into_account: true,
                        periodicity: SubjectPeriodicity::AmountInYear {
                            interrogation_count_in_year: ner(1..=3),
                            minimum_week_separation: 1,
                        },
                    }),
                },
                excluded_periods: BTreeSet::new(),
            },
        )),
        "subject english",
        SubjectId
    );
    let subject_philosophy = apply_new_id!(
        &mut state,
        Op::Subject(SubjectOp::AddAfter(
            Some(subject_english),
            Subject {
                parameters: SubjectParameters {
                    name: "Philosophie".to_string(),
                    interrogation_parameters: Some(SubjectInterrogationParameters {
                        students_per_group: ner(nz(1)..=nz(2)),
                        groups_per_interrogation: ner(nz(1)..=nz(1)),
                        duration: NonZeroMinutes::new(60).expect("Duration should be non-zero"),
                        take_duration_into_account: true,
                        periodicity: SubjectPeriodicity::AmountForEveryArbitraryBlock {
                            blocks: vec![WeekBlock {
                                delay_in_weeks: 0,
                                size_in_weeks: nz(3),
                                interrogation_count_in_block: ner(1..=2),
                            }],
                            minimum_week_separation: 1,
                        },
                    }),
                },
                excluded_periods: BTreeSet::new(),
            },
        )),
        "subject philosophy",
        SubjectId
    );
    let _subject_sport = apply_new_id!(
        &mut state,
        Op::Subject(SubjectOp::AddAfter(
            Some(subject_philosophy),
            Subject {
                parameters: SubjectParameters {
                    name: "Sport".to_string(),
                    interrogation_parameters: None,
                },
                excluded_periods: BTreeSet::new(),
            },
        )),
        "subject sport",
        SubjectId
    );

    // Teachers: one teaches two subjects
    let teacher1 = apply_new_id!(
        &mut state,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: person("Rogue", "Severus", Some("0605060708"), None),
            subjects: BTreeSet::from([subject_maths, subject_physics]),
        })),
        "teacher 1",
        TeacherId
    );
    let teacher2 = apply_new_id!(
        &mut state,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: person("McGonagall", "Minerva", None, Some("minerva@poudlard.fr")),
            subjects: BTreeSet::from([subject_maths]),
        })),
        "teacher 2",
        TeacherId
    );
    let teacher3 = apply_new_id!(
        &mut state,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: person("Lupin", "Remus", None, None),
            subjects: BTreeSet::from([subject_english, subject_philosophy]),
        })),
        "teacher 3",
        TeacherId
    );

    // Assignments, set wholesale as rows (SetRow). student4 lands on english,
    // and is deliberately kept off the period-1 maths row.
    for (period, subject, students, desc) in [
        (
            period1,
            subject_maths,
            BTreeSet::from([student1, student2, student3]),
            "maths on period 1",
        ),
        (
            period1,
            subject_physics,
            BTreeSet::from([student1]),
            "physics on period 1",
        ),
        (
            period1,
            subject_english,
            BTreeSet::from([student4]),
            "english on period 1",
        ),
        (
            period2,
            subject_maths,
            BTreeSet::from([student1]),
            "maths on period 2",
        ),
    ] {
        apply(
            &mut state,
            Op::Assignment(AssignmentOp::SetRow(period, subject, students)),
            desc,
        );
    }

    // Week patterns over the 7 weeks of the two periods. Snapshot the week ids
    // in walk order so the fortnight pattern can exclude the even-index weeks.
    let week_ids: Vec<WeekId> = state
        .get_data()
        .get_inner_data()
        .params
        .walk_weeks()
        .map(|(_period_id, week_id, _week)| week_id)
        .collect();
    let pattern_fortnight = apply_new_id!(
        &mut state,
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "Quinzaine A".to_string(),
            // Excludes global weeks 1, 3, 5 (the `false` bits of the old
            // positional `[true, false, true, false, true, false, true]`).
            excluded_weeks: BTreeSet::from([week_ids[1], week_ids[3], week_ids[5]]),
        })),
        "week pattern fortnight",
        WeekPatternId
    );
    let pattern_all = apply_new_id!(
        &mut state,
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "Toutes les semaines".to_string(),
            excluded_weeks: BTreeSet::new(),
        })),
        "week pattern all",
        WeekPatternId
    );

    // Slots on three subjects, with and without week pattern, with
    // positive, zero and negative costs
    let slot_maths1 = apply_new_id!(
        &mut state,
        Op::Slot(SlotOp::AddAfter(
            None,
            Slot {
                subject_id: subject_maths,
                teacher_id: teacher1,
                start_time: slot_start(chrono::Weekday::Mon, 14, 0),
                extra_info: "Salle 101".to_string(),
                week_pattern: None,
                cost: 0,
            },
        )),
        "slot maths 1",
        SlotId
    );
    let slot_maths2 = apply_new_id!(
        &mut state,
        Op::Slot(SlotOp::AddAfter(
            Some(slot_maths1),
            Slot {
                subject_id: subject_maths,
                teacher_id: teacher2,
                start_time: slot_start(chrono::Weekday::Tue, 17, 0),
                extra_info: String::new(),
                week_pattern: Some(pattern_fortnight),
                cost: 2,
            },
        )),
        "slot maths 2",
        SlotId
    );
    let slot_physics = apply_new_id!(
        &mut state,
        Op::Slot(SlotOp::AddAfter(
            None,
            Slot {
                subject_id: subject_physics,
                teacher_id: teacher1,
                start_time: slot_start(chrono::Weekday::Wed, 15, 30),
                extra_info: "Labo".to_string(),
                week_pattern: Some(pattern_all),
                cost: -1,
            },
        )),
        "slot physics",
        SlotId
    );
    let _slot_english = apply_new_id!(
        &mut state,
        Op::Slot(SlotOp::AddAfter(
            None,
            Slot {
                subject_id: subject_english,
                teacher_id: teacher3,
                start_time: slot_start(chrono::Weekday::Fri, 8, 0),
                extra_info: String::new(),
                week_pattern: None,
                cost: 0,
            },
        )),
        "slot english",
        SlotId
    );

    // An incompatibility with several slots and a week pattern
    apply(
        &mut state,
        Op::Incompat(IncompatOp::Add(Incompatibility {
            subject_id: subject_maths,
            name: "Option latin".to_string(),
            slots: vec![
                incompat_slot(chrono::Weekday::Mon, 8, 0, 60),
                incompat_slot(chrono::Weekday::Thu, 10, 0, 90),
            ],
            minimum_free_slots: nz(1),
            week_pattern_id: Some(pattern_all),
        })),
        "incompat",
    );

    // Group lists: a prefilled one (mixed named/unnamed groups) and an
    // automatic one (with an excluded student), both associated to subjects
    let maths_params = GroupListParameters {
        name: "Groupes de maths".to_string(),
        students_per_group: ner(nz(2)..=nz(3)),
        group_names: vec![Some(non_empty("Gryffondor")), None],
    };
    let group_list_maths = apply_new_id!(
        &mut state,
        Op::GroupList(GroupListOp::Add(
            GroupList::new(maths_params.clone(), GroupListFilling::default()).unwrap(),
        )),
        "group list maths",
        GroupListId
    );
    apply(
        &mut state,
        Op::GroupList(GroupListOp::Update(
            group_list_maths,
            GroupList::new(
                maths_params,
                GroupListFilling::Prefilled {
                    groups: vec![
                        PrefilledGroup {
                            students: BTreeSet::from([student1, student2]),
                        },
                        PrefilledGroup {
                            students: BTreeSet::from([student3]),
                        },
                    ],
                },
            )
            .unwrap(),
        )),
        "prefill group list maths",
    );
    let physics_params = GroupListParameters {
        name: "Groupes de physique".to_string(),
        students_per_group: ner(nz(1)..=nz(2)),
        group_names: vec![None, Some(non_empty("Binôme B")), None],
    };
    let group_list_physics = apply_new_id!(
        &mut state,
        Op::GroupList(GroupListOp::Add(
            GroupList::new(physics_params.clone(), GroupListFilling::default()).unwrap(),
        )),
        "group list physics",
        GroupListId
    );
    apply(
        &mut state,
        Op::GroupList(GroupListOp::Update(
            group_list_physics,
            GroupList::new(
                physics_params,
                GroupListFilling::Automatic {
                    excluded_students: BTreeSet::from([student4]),
                },
            )
            .unwrap(),
        )),
        "exclusions group list physics",
    );
    apply(
        &mut state,
        Op::GroupList(GroupListOp::AssignToSubject(
            period1,
            subject_maths,
            Some(group_list_maths),
        )),
        "associate maths period 1",
    );
    apply(
        &mut state,
        Op::GroupList(GroupListOp::AssignToSubject(
            period2,
            subject_maths,
            Some(group_list_maths),
        )),
        "associate maths period 2",
    );
    apply(
        &mut state,
        Op::GroupList(GroupListOp::AssignToSubject(
            period1,
            subject_physics,
            Some(group_list_physics),
        )),
        "associate physics period 1",
    );

    // Settings: global limits plus a per-student override
    apply(
        &mut state,
        Op::Settings(SettingsOp::SetGlobal(Limits {
            interrogations_per_week_min: Some(SoftParam {
                soft: false,
                value: 1,
            }),
            interrogations_per_week_max: Some(SoftParam {
                soft: true,
                value: 4,
            }),
            max_interrogations_per_day: Some(SoftParam {
                soft: false,
                value: nz(2),
            }),
        })),
        "global settings",
    );
    apply(
        &mut state,
        Op::Settings(SettingsOp::SetStudent(
            student1,
            Some(Limits {
                interrogations_per_week_min: None,
                interrogations_per_week_max: Some(SoftParam {
                    soft: true,
                    value: 3,
                }),
                max_interrogations_per_day: None,
            }),
        )),
        "per-student settings",
    );

    // Balancing: global options plus a per-subject override
    apply(
        &mut state,
        Op::Balancing(BalancingOp::Update(Balancing {
            global: BalancingOptions {
                teacher_rotation: Some(SoftParam {
                    soft: true,
                    value: (),
                }),
                slot_rotation: None,
                avoid_twice_in_a_row: true,
                year_teacher_rotation: false,
                period_teacher_rotation: true,
            },
            subjects: BTreeMap::from([(
                subject_maths,
                BalancingOptions {
                    teacher_rotation: None,
                    slot_rotation: Some(SoftParam {
                        soft: false,
                        value: (),
                    }),
                    avoid_twice_in_a_row: false,
                    year_teacher_rotation: true,
                    period_teacher_rotation: false,
                },
            )])
            .into(),
        })),
        "balancing",
    );

    // A pairing rule and a slot pairing rule
    apply(
        &mut state,
        Op::Pairing(PairingOp::Add(
            PairingRule::new(
                RulePart {
                    subject_id: subject_maths,
                    should_have: true,
                },
                RulePart {
                    subject_id: subject_english,
                    should_have: false,
                },
                BTreeSet::from([period2]),
                true,
            )
            .expect("distinct subjects"),
        )),
        "pairing rule",
    );
    apply(
        &mut state,
        Op::SlotPairing(SlotPairingOp::Add(
            SlotPairingRule::new(
                SlotRulePart {
                    slot_id: slot_maths1,
                    should_have: true,
                },
                SlotRulePart {
                    slot_id: slot_maths2,
                    should_have: true,
                },
                BTreeSet::new(),
                false,
            )
            .expect("distinct slots"),
        )),
        "slot pairing rule",
    );

    // Colloscope: fill the automatic group list and a few interrogations
    let week0_p1 = state
        .get_data()
        .get_inner_data()
        .params
        .weeks
        .week_id_at(period1, 0)
        .expect("period1 has a first week");
    let week1_p1 = state
        .get_data()
        .get_inner_data()
        .params
        .weeks
        .week_id_at(period1, 1)
        .expect("period1 has a second week");
    apply(
        &mut state,
        Op::Colloscope(ColloscopeOp::SetGroupList(
            group_list_physics,
            BTreeMap::from([(student1, 0), (student2, 1), (student3, 2)]),
        )),
        "colloscope group list",
    );
    apply(
        &mut state,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot_maths1,
            week0_p1,
            BTreeSet::from([0]),
        )),
        "interrogation maths 1",
    );
    apply(
        &mut state,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot_maths2,
            week0_p1,
            BTreeSet::from([1]),
        )),
        "interrogation maths 2",
    );
    apply(
        &mut state,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot_physics,
            week1_p1,
            BTreeSet::from([0, 2]),
        )),
        "interrogation physics",
    );

    // Export configuration: several op variants, values away from defaults
    apply(
        &mut state,
        Op::ExportConfig(ExportConfigOp::UpdateGlobalConfig(
            export_config::GlobalConfig {
                background_color: export_config::Color {
                    red: 240,
                    green: 240,
                    blue: 255,
                },
                stripes_color_enabled: false,
                stripes_color: export_config::Color {
                    red: 200,
                    green: 200,
                    blue: 200,
                },
            },
        )),
        "export global config",
    );
    apply(
        &mut state,
        Op::ExportConfig(ExportConfigOp::UpdateColloscopeConfig(
            export_config::ColloscopeConfig {
                sheet_name: "Colloscope 2026".to_string(),
                extra_info_column_enabled: true,
                extra_info_column_name: "Salle".to_string(),
                teacher_email_enabled: true,
                teacher_email: "Courriel".to_string(),
                teacher_tel_enabled: false,
                teacher_tel: String::new(),
                orientation: export_config::PageOrientation::Landscape,
                display_week_dates: true,
                display_annotations: true,
                no_interrogation_color: export_config::Color {
                    red: 128,
                    green: 128,
                    blue: 128,
                },
                annotation_color_enabled: true,
                annotation_color: export_config::Color {
                    red: 255,
                    green: 255,
                    blue: 0,
                },
                extra_colors: BTreeMap::from([(
                    "Vacances".to_string(),
                    export_config::Color {
                        red: 0,
                        green: 128,
                        blue: 0,
                    },
                )]),
            },
        )),
        "export colloscope config",
    );
    apply(
        &mut state,
        Op::ExportConfig(ExportConfigOp::UpdateAllGroupsConfig(
            export_config::PerStudentGroupsConfig {
                sheet_name: "Groupes".to_string(),
                orientation: Some(export_config::PageOrientation::Portrait),
                show_emails: true,
                show_tel: false,
            },
        )),
        "export all groups config",
    );
    apply(
        &mut state,
        Op::ExportConfig(ExportConfigOp::UpdatePerGroupListConfig(
            export_config::PerGroupListConfig {
                orientation: export_config::PageOrientation::Landscape,
                show_emails: false,
                show_tel: true,
                center_vertically: true,
            },
        )),
        "export per group list config",
    );
    apply(
        &mut state,
        Op::ExportConfig(ExportConfigOp::UpdateColloscopeEnabled(false)),
        "export colloscope enabled",
    );
    apply(
        &mut state,
        Op::ExportConfig(ExportConfigOp::UpdatePrefilledGroupsEnabled(true)),
        "export prefilled groups enabled",
    );

    let data = state.get_data().clone();
    check_all_sections_populated(&data);
    data
}

/// Guards against the builder silently degrading: every serialized
/// section must be non-trivially populated
fn check_all_sections_populated(data: &Data) {
    let inner = data.get_inner_data();
    let params = &inner.params;

    assert!(params.periods.first_week.is_some());
    assert!(params.periods.period_count() >= 2);
    assert!(
        params
            .walk_weeks()
            .any(|(_id, _week_id, week)| week.annotation.is_some())
    );

    assert!(params.students.student_map.len() >= 4);
    assert!(
        params
            .students
            .student_map
            .values()
            .any(|student| !student.excluded_periods.is_empty())
    );

    assert!(params.subjects.ordered_subject_list.len() >= 5);
    assert!(
        params
            .subjects
            .ordered_subject_list
            .iter()
            .any(|(_id, subject)| subject.parameters.interrogation_parameters.is_none())
    );
    assert!(
        params
            .subjects
            .ordered_subject_list
            .iter()
            .any(|(_id, subject)| !subject.excluded_periods.is_empty())
    );

    assert!(
        params
            .teachers
            .teacher_map
            .values()
            .any(|teacher| teacher.subjects.len() >= 2)
    );

    assert!(
        params
            .assignments
            .map
            .values()
            .any(|students| !students.is_empty())
    );

    assert!(params.week_patterns.week_pattern_map.len() >= 2);

    assert!(
        params
            .slots
            .subjects_with_slots()
            .filter(|subject_id| {
                params
                    .slots
                    .slot_count_for_subject(*subject_id)
                    .expect("subject comes from subjects_with_slots")
                    > 0
            })
            .count()
            >= 2
    );

    assert!(!params.incompats.incompat_map.is_empty());

    assert!(
        params
            .group_lists
            .group_list_map
            .values()
            .any(|group_list| group_list.filling().is_prefilled())
    );
    assert!(
        params
            .group_lists
            .group_list_map
            .values()
            .any(|group_list| !group_list.filling().is_prefilled())
    );
    assert!(!params.group_lists.subjects_associations.is_empty());

    assert!(!params.settings.students.is_empty());
    assert!(!params.balancing.subjects.is_empty());
    assert!(!params.pairings.pairing_rule_map.is_empty());
    assert!(!params.slot_pairings.slot_pairing_rule_map.is_empty());

    assert!(!inner.colloscope.is_empty());
    assert!(!inner.colloscope.are_group_lists_empty());

    assert!(inner.export_config != export_config::ExportConfig::default());
}
