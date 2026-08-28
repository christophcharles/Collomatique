//! Tests for the sparse colloscope surface.
//!
//! The surface (`Colloscope::interrogation` / `iter` / `group_list` / the
//! `set_*` writers) presents the *canonical sparse view* over the dense
//! skeleton: only non-empty cells are rows. These tests pin that semantics, the
//! `WeekId` ↔ positional mapping across more than one period, the writer
//! round-trips (including clearing), and the `is_interrogation_possible` /
//! `is_week_active` possibility predicates.

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    Data, GroupListOp, NewId, NonEmptyRangeInclusive, Op, PeriodOp, SlotOp, StudentOp, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp,
    WeekOp, WeekPatternOp,
    colloscopes::Colloscope,
    group_lists::{GroupList, GroupListFilling, GroupListParameters},
    ids::{Id, PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekId, WeekPatternId},
    slots::Slot,
    students::Student,
    teachers::Teacher,
    week_patterns::WeekPattern,
    weeks::WeekDesc,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

fn interrogation_subject(name: &str) -> Subject {
    Subject {
        parameters: SubjectParameters {
            name: name.into(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: NonEmptyRangeInclusive::new(
                    NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                )
                .expect("statically non-empty"),
                groups_per_interrogation: NonEmptyRangeInclusive::new(
                    NonZeroU32::new(1).unwrap()..=NonZeroU32::new(1).unwrap(),
                )
                .expect("statically non-empty"),
                duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
                },
            }),
        },
        excluded_periods: BTreeSet::new(),
    }
}

fn plain_subject(name: &str) -> Subject {
    Subject {
        parameters: SubjectParameters {
            name: name.into(),
            interrogation_parameters: None,
        },
        excluded_periods: BTreeSet::new(),
    }
}

fn make_slot(
    subject_id: SubjectId,
    teacher_id: TeacherId,
    week_pattern: Option<WeekPatternId>,
) -> Slot {
    Slot {
        subject_id,
        teacher_id,
        start_time: collomatique_time::SlotStart {
            weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
            start_time: collomatique_time::WholeMinuteTime::new(
                chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
            )
            .unwrap(),
        },
        extra_info: String::new(),
        week_pattern,
        cost: 0,
    }
}

fn one_group_params(name: &str) -> GroupListParameters {
    GroupListParameters {
        name: name.into(),
        students_per_group: NonEmptyRangeInclusive::new(
            NonZeroU32::new(1).unwrap()..=NonZeroU32::new(3).unwrap(),
        )
        .expect("statically non-empty"),
        group_names: vec![None],
    }
}

/// A two-period document. Period 1 holds weeks `[w1a, w1b]`, period 2 holds
/// `[w2a, w2b]`, so weeks in period 2 sit at a non-zero global offset. `math`
/// runs interrogations everywhere (slot has no pattern); `art` has no
/// interrogations. `excluded` is a period math does not run on.
struct Built {
    period1: PeriodId,
    period2: PeriodId,
    excluded: PeriodId,
    w1a: WeekId,
    w2a: WeekId,
    w2b: WeekId,
    w_excluded: WeekId,
    math_slot: SlotId,
    art_slot: SlotId,
    student: StudentId,
    group_list: collomatique_state_colloscopes::ids::GroupListId,
    trivial_pattern: WeekPatternId,
}

fn build_document(app: &mut AppState<Data, String>) -> Built {
    macro_rules! apply_new {
        ($op:expr, $variant:path, $msg:expr) => {{
            let Ok(Some($variant(id))) = app.apply($op, $msg.into()) else {
                panic!(concat!("unexpected result: ", $msg));
            };
            id
        }};
    }
    macro_rules! apply_ok {
        ($op:expr, $msg:expr) => {{
            app.apply($op, $msg.into()).expect($msg);
        }};
    }

    // Three periods (front-inserted, so creation order reverses): build them so
    // that display order is period1, period2, excluded.
    let excluded = apply_new!(
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "add excluded"
    );
    let period2 = apply_new!(
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "add period2"
    );
    let period1 = apply_new!(
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "add period1"
    );

    // Two weeks per period.
    let w1a = apply_new!(
        Op::Week(WeekOp::AddFront(period1, WeekDesc::new(true))),
        NewId::WeekId,
        "add w1a"
    );
    let _w1b = apply_new!(
        Op::Week(WeekOp::AddAfter(w1a, WeekDesc::new(true))),
        NewId::WeekId,
        "add w1b"
    );
    let w2a = apply_new!(
        Op::Week(WeekOp::AddFront(period2, WeekDesc::new(true))),
        NewId::WeekId,
        "add w2a"
    );
    let w2b = apply_new!(
        Op::Week(WeekOp::AddAfter(w2a, WeekDesc::new(false))),
        NewId::WeekId,
        "add w2b (no interrogations)"
    );
    let w_excluded = apply_new!(
        Op::Week(WeekOp::AddFront(excluded, WeekDesc::new(true))),
        NewId::WeekId,
        "add w_excluded"
    );

    let trivial_pattern = apply_new!(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "trivial".into(),
            excluded_weeks: BTreeSet::new(),
        })),
        NewId::WeekPatternId,
        "add trivial pattern"
    );

    let math = apply_new!(
        Op::Subject(SubjectOp::AddAfter(None, interrogation_subject("Math"))),
        NewId::SubjectId,
        "add math"
    );
    // Exclude Math from the `excluded` period.
    let mut math_excluded = interrogation_subject("Math");
    math_excluded.excluded_periods = BTreeSet::from([excluded]);
    apply_ok!(
        Op::Subject(SubjectOp::Update(math, math_excluded)),
        "exclude math from excluded period"
    );

    let art = apply_new!(
        Op::Subject(SubjectOp::AddAfter(Some(math), plain_subject("Art"))),
        NewId::SubjectId,
        "add art"
    );

    let teacher = apply_new!(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([math]),
        })),
        NewId::TeacherId,
        "add teacher"
    );
    let art_teacher = apply_new!(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::new(),
        })),
        NewId::TeacherId,
        "add art teacher"
    );

    let math_slot = apply_new!(
        Op::Slot(SlotOp::AddAfter(None, make_slot(math, teacher, None))),
        NewId::SlotId,
        "add math slot"
    );
    // `art` has no interrogations, so it cannot host a slot; reuse the id space
    // by pointing `art_slot` at a non-existent slot instead. We only need a
    // dangling SlotId for the truth-table's "unknown slot" line.
    let art_slot = unsafe { SlotId::new(1u64 << 40) };
    let _ = art;
    let _ = art_teacher;

    let student = apply_new!(
        Op::Student(StudentOp::Add(Student::default())),
        NewId::StudentId,
        "add student"
    );

    let group_list = apply_new!(
        Op::GroupList(GroupListOp::Add(
            GroupList::new(one_group_params("GL"), GroupListFilling::default()).unwrap(),
        )),
        NewId::GroupListId,
        "add group list"
    );

    Built {
        period1,
        period2,
        excluded,
        w1a,
        w2a,
        w2b,
        w_excluded,
        math_slot,
        art_slot,
        student,
        group_list,
        trivial_pattern,
    }
}

#[test]
fn empty_colloscope_reads_as_no_rows() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let collo = Colloscope::default();

    // A fresh colloscope holds no rows.
    assert_eq!(collo.iter().count(), 0);
    assert_eq!(collo.group_lists_iter().count(), 0);
    assert!(collo.interrogation(ids.math_slot, ids.w1a).is_none());
    assert!(collo.group_list(ids.group_list).is_none());
}

#[test]
fn set_interrogation_round_trips_and_maps_week_id() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let mut collo = Colloscope::default();

    // A week in period 2 sits at a non-zero global offset — this exercises the
    // WeekId → (period, position) translation, not just index 0.
    collo.set_interrogation(ids.math_slot, ids.w2a, BTreeSet::from([0]));

    assert_eq!(
        collo.interrogation(ids.math_slot, ids.w2a),
        Some(&BTreeSet::from([0]))
    );
    // The other weeks stay absent.
    assert!(collo.interrogation(ids.math_slot, ids.w1a).is_none());

    // `iter` yields exactly the one row, correctly keyed.
    let rows: Vec<_> = collo.iter().collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, (ids.math_slot, ids.w2a));
    assert_eq!(rows[0].1, &BTreeSet::from([0]));

    // `interrogations_for_slot` agrees.
    let slot_rows: Vec<_> = collo.interrogations_for_slot(ids.math_slot).collect();
    assert_eq!(slot_rows, vec![(ids.w2a, &BTreeSet::from([0]))]);

    // Writing an empty set clears the row.
    collo.set_interrogation(ids.math_slot, ids.w2a, BTreeSet::new());
    assert!(collo.interrogation(ids.math_slot, ids.w2a).is_none());
    assert_eq!(collo.iter().count(), 0);
}

#[test]
fn set_group_list_round_trips_and_clears() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let mut collo = Colloscope::default();

    let placements = BTreeMap::from([(ids.student, 0u32)]);
    collo.set_group_list(ids.group_list, placements.clone());

    assert_eq!(collo.group_list(ids.group_list), Some(&placements));
    let lists: Vec<_> = collo.group_lists_iter().collect();
    assert_eq!(lists, vec![(ids.group_list, &placements)]);

    // Emptying the map clears the row.
    collo.set_group_list(ids.group_list, BTreeMap::new());
    assert!(collo.group_list(ids.group_list).is_none());
    assert_eq!(collo.group_lists_iter().count(), 0);
}

#[test]
fn is_interrogation_possible_truth_table() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let params = app.get_data().get_inner_data().params.clone();

    // Happy path: math slot, interrogation week in period 1.
    assert!(params.is_interrogation_possible(ids.math_slot, ids.w1a));
    // Interrogation week in period 2, non-zero offset.
    assert!(params.is_interrogation_possible(ids.math_slot, ids.w2a));

    // `w2b` carries no interrogations → impossible.
    assert!(!params.is_interrogation_possible(ids.math_slot, ids.w2b));

    // `w_excluded` belongs to a period math is excluded from → impossible.
    assert!(!params.is_interrogation_possible(ids.math_slot, ids.w_excluded));

    // A dangling slot id → impossible.
    assert!(!params.is_interrogation_possible(ids.art_slot, ids.w1a));

    // A dangling week id → impossible.
    let dangling_week = unsafe { WeekId::new(1u64 << 40) };
    assert!(!params.is_interrogation_possible(ids.math_slot, dangling_week));
}

#[test]
fn week_patterns_is_week_active_matches_parameters() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let params = app.get_data().get_inner_data().params.clone();

    for week in [ids.w1a, ids.w2a, ids.w2b, ids.w_excluded] {
        for pattern in [None, Some(ids.trivial_pattern)] {
            assert_eq!(
                params.is_week_active(week, pattern),
                params
                    .week_patterns
                    .is_week_active(&params.weeks, week, pattern),
                "delegation must agree for week {week:?} pattern {pattern:?}",
            );
        }
    }

    // Concretely: interrogation weeks are active under the trivial pattern,
    // the no-interrogation week is not.
    assert!(params.is_week_active(ids.w1a, Some(ids.trivial_pattern)));
    assert!(!params.is_week_active(ids.w2b, Some(ids.trivial_pattern)));
    let _ = (ids.period1, ids.period2, ids.excluded);
}
