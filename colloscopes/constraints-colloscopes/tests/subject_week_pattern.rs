//! A subject's week pattern, seen from the constraints layer.
//!
//! The scenario is the one the feature exists for: a nine-week period whose
//! eighth week runs tutorials but no interrogations. Without a way to say so,
//! a block-of-two subject sees nine active weeks, nine is not a multiple of
//! two, and every enrolled student gets a
//! [`InfeasibleConstraint::PeriodicityOncePerBlockInfeasible`] row — an
//! always-false constraint. Cutting a one-week period out of the nine is the
//! only workaround today, and it makes things worse: the one-week period is a
//! multiple of nothing, and it cuts an `ExactlyPeriodic` subject's run in two.
//!
//! A week pattern attached to the *subject* says it directly. The pattern can
//! only disable weeks, and it is ANDed with the slot's own pattern in
//! `tools::enumerate_weeks_for_slot`, the single choke point every ILP variable
//! domain flows through. So a subject-disabled week carries no variable at all,
//! and the periodicity families count eight active weeks instead of nine.
//!
//! The document is built through the checked op path, not from a
//! `.collomatique` fixture: the file format does not carry the field yet.

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use collomatique_constraints_colloscopes::{
    ColloscopeModel, ConstraintDesc, ConstraintSource, ExtraVarName, InfeasibleConstraint,
    InternalVar, ProgressiveConstraint, StructuralConstraint, Var, build_model,
};
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    AssignmentOp, Data, GroupListOp, IncompatOp, NewId, NonEmptyRangeInclusive, Op, PeriodOp,
    SlotOp, StudentOp, Subject, SubjectInterrogationParameters, SubjectOp, SubjectParameters,
    SubjectPeriodicity, TeacherOp, WeekOp, WeekPatternOp,
    colloscope_params::Parameters,
    group_lists::{GroupList, GroupListFilling, GroupListParameters},
    ids::{SlotId, SubjectId, WeekPatternId},
    incompats::Incompatibility,
    slots::Slot,
    students::Student,
    teachers::Teacher,
    week_patterns::WeekPattern,
    weeks::WeekDesc,
};

/// Weeks in the single period. Nine, so a block of two does not tile it.
const WEEK_COUNT: usize = 9;

/// The tutorials-only week, as a global week index — the eighth of nine.
const PAUSED_WEEK: usize = 7;

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("value should be non-zero")
}

fn ner(range: std::ops::RangeInclusive<NonZeroU32>) -> NonEmptyRangeInclusive<NonZeroU32> {
    NonEmptyRangeInclusive::new(range).expect("range should be non-empty")
}

fn subject(name: &str, periodicity: SubjectPeriodicity, pattern: Option<WeekPatternId>) -> Subject {
    Subject {
        parameters: SubjectParameters {
            name: name.into(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: ner(nz(1)..=nz(3)),
                groups_per_interrogation: ner(nz(1)..=nz(1)),
                duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
                take_duration_into_account: false,
                periodicity,
            }),
        },
        excluded_periods: BTreeSet::new(),
        week_pattern: pattern,
    }
}

/// 08:00 on `weekday`. Every slot of the document starts there.
fn slot_start(weekday: chrono::Weekday) -> collomatique_time::SlotStart {
    collomatique_time::SlotStart {
        weekday: collomatique_time::Weekday(weekday),
        start_time: collomatique_time::WholeMinuteTime::new(
            chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        )
        .unwrap(),
    }
}

fn slot(
    subject_id: SubjectId,
    teacher_id: collomatique_state_colloscopes::ids::TeacherId,
    weekday: chrono::Weekday,
) -> Slot {
    Slot {
        subject_id,
        teacher_id,
        start_time: slot_start(weekday),
        extra_info: String::new(),
        week_pattern: None,
        cost: 0,
    }
}

/// What the tests need to point at, once the document is built.
struct Built {
    params: Parameters,
    maths: SubjectId,
    physics: SubjectId,
    maths_slot: SlotId,
    physics_slot: SlotId,
}

/// Which knobs of the document the test wants turned on.
#[derive(Default, Clone, Copy)]
struct Scenario {
    /// Whether maths wears the « Pause colles Noël » pattern.
    maths_paused: bool,
    /// Whether physics wears it.
    physics_paused: bool,
    /// An « Indisponibilité » on maths, every week, over the maths slot's own
    /// time window, that must be left entirely free.
    maths_incompat: bool,
}

/// One nine-week period, two interrogated subjects sharing a teacher and a
/// group list, two students enrolled in both.
///
/// The « Pause colles Noël » pattern that switches [`PAUSED_WEEK`] off always
/// exists, so two documents differ in exactly the fields [`Scenario`] names.
fn build_document(scenario: Scenario) -> Built {
    let mut app = AppState::<_, String>::new(Data::new());

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

    let period = apply_new!(
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "add period"
    );

    let mut weeks = Vec::with_capacity(WEEK_COUNT);
    let first = apply_new!(
        Op::Week(WeekOp::AddFront(period, WeekDesc::new(true))),
        NewId::WeekId,
        "add first week"
    );
    weeks.push(first);
    for _ in 1..WEEK_COUNT {
        let previous = *weeks.last().expect("the first week is already in");
        weeks.push(apply_new!(
            Op::Week(WeekOp::AddAfter(previous, WeekDesc::new(true))),
            NewId::WeekId,
            "add week"
        ));
    }

    let pause = apply_new!(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "Pause colles Noël".into(),
            excluded_weeks: BTreeSet::from([weeks[PAUSED_WEEK]]),
        })),
        NewId::WeekPatternId,
        "add the pause pattern"
    );

    let maths = apply_new!(
        Op::Subject(SubjectOp::AddAfter(
            None,
            subject(
                "Maths",
                SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                    weeks_per_block: nz(2),
                    minimum_week_separation: nz(1),
                },
                scenario.maths_paused.then_some(pause),
            )
        )),
        NewId::SubjectId,
        "add maths"
    );
    let physics = apply_new!(
        Op::Subject(SubjectOp::AddAfter(
            Some(maths),
            subject(
                "Physique",
                SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: nz(4),
                },
                scenario.physics_paused.then_some(pause),
            )
        )),
        NewId::SubjectId,
        "add physics"
    );

    let teacher = apply_new!(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([maths, physics]),
        })),
        NewId::TeacherId,
        "add teacher"
    );
    let maths_slot = apply_new!(
        Op::Slot(SlotOp::AddAfter(
            None,
            slot(maths, teacher, chrono::Weekday::Mon)
        )),
        NewId::SlotId,
        "add maths slot"
    );
    let physics_slot = apply_new!(
        Op::Slot(SlotOp::AddAfter(
            None,
            slot(physics, teacher, chrono::Weekday::Tue)
        )),
        NewId::SlotId,
        "add physics slot"
    );

    let mut students = BTreeSet::new();
    for _ in 0..2 {
        students.insert(apply_new!(
            Op::Student(StudentOp::Add(Student::default())),
            NewId::StudentId,
            "add student"
        ));
    }

    let group_list = apply_new!(
        Op::GroupList(GroupListOp::Add(
            GroupList::new(
                GroupListParameters {
                    name: "Groupes".into(),
                    students_per_group: ner(nz(1)..=nz(3)),
                    group_names: vec![None, None],
                },
                GroupListFilling::default(),
            )
            .expect("the group list should be valid"),
        )),
        NewId::GroupListId,
        "add group list"
    );

    for subject_id in [maths, physics] {
        apply_ok!(
            Op::Assignment(AssignmentOp::SetRow(period, subject_id, students.clone())),
            "enroll the students"
        );
        apply_ok!(
            Op::GroupList(GroupListOp::AssignToSubject(
                period,
                subject_id,
                Some(group_list)
            )),
            "associate the group list"
        );
    }

    if scenario.maths_incompat {
        // One slot and `minimum_free_slots == 1` is what selects the saturated
        // branch of the incompat builder; the window is the maths slot's own
        // Monday 08:00–09:00, so it overlaps it; `None` covers every week.
        apply_ok!(
            Op::Incompat(IncompatOp::Add(Incompatibility {
                subject_id: maths,
                name: "Indisponibilité".into(),
                slots: vec![
                    collomatique_time::SlotWithDuration::new(
                        slot_start(chrono::Weekday::Mon),
                        collomatique_time::NonZeroMinutes::new(60).unwrap(),
                    )
                    .expect("the window should not cross midnight"),
                ],
                minimum_free_slots: nz(1),
                week_pattern_id: None,
            })),
            "add the incompat"
        );
    }

    Built {
        params: app.get_data().get_inner_data().params.clone(),
        maths,
        physics,
        maths_slot,
        physics_slot,
    }
}

/// Every `PeriodicityOncePerBlockInfeasible` row the model carries.
fn block_infeasibilities(model: &ColloscopeModel) -> Vec<&InfeasibleConstraint> {
    model
        .problem()
        .get_constraints()
        .iter()
        .filter_map(|(_constraint, source)| match source {
            ConstraintSource::User(ConstraintDesc::Level0(
                c @ InfeasibleConstraint::PeriodicityOncePerBlockInfeasible { .. },
            )) => Some(c),
            _ => None,
        })
        .collect()
}

/// Every `(first_week, last_week)` a `PeriodicityInterrogationCountExact` row
/// covers for `subject`.
fn exact_count_windows(model: &ColloscopeModel, subject: SubjectId) -> BTreeSet<(usize, usize)> {
    model
        .problem()
        .get_constraints()
        .iter()
        .filter_map(|(_constraint, source)| match source {
            ConstraintSource::User(ConstraintDesc::Level3(
                ProgressiveConstraint::PeriodicityInterrogationCountExact {
                    subject: s,
                    first_week,
                    last_week,
                    ..
                },
            )) if *s == subject => Some((first_week.0, last_week.0)),
            _ => None,
        })
        .collect()
}

/// Which global weeks carry an interrogation variable for `slot`.
///
/// Both variable families are read: `GroupInInterrogation` is the base variable
/// whose domain `compute_week_range` derives from the choke point, and
/// `StudentAtInterrogation` is the extra every periodicity sum is written over.
fn weeks_with_variables(model: &ColloscopeModel, slot: SlotId) -> BTreeSet<usize> {
    model
        .problem()
        .get_variables()
        .keys()
        .filter_map(|var| match var {
            InternalVar::Base(Var::GroupInInterrogation { slot: s, week, .. })
            | InternalVar::Extra(ExtraVarName::StudentAtInterrogation { slot: s, week, .. })
                if *s == slot =>
            {
                Some(week.0)
            }
            _ => None,
        })
        .collect()
}

/// Every global week carrying an `IncompatSaturated` row.
fn incompat_saturated_weeks(model: &ColloscopeModel) -> BTreeSet<usize> {
    model
        .problem()
        .get_constraints()
        .iter()
        .filter_map(|(_constraint, source)| match source {
            ConstraintSource::User(ConstraintDesc::Level1(
                StructuralConstraint::IncompatSaturated { week, .. },
            )) => Some(week.0),
            _ => None,
        })
        .collect()
}

#[test]
fn nine_weeks_do_not_tile_into_blocks_of_two() {
    // The state layer accepts this document — the multiple rule is solver-side
    // blame, not a gate invariant — so the builder is where it surfaces.
    let built = build_document(Scenario::default());
    let model = build_model(&built.params);

    let infeasible = block_infeasibilities(&model);
    assert_eq!(
        infeasible.len(),
        2,
        "nine active weeks and blocks of two: every enrolled student should get an \
         infeasible row, found {infeasible:?}",
    );
    for row in infeasible {
        let InfeasibleConstraint::PeriodicityOncePerBlockInfeasible {
            subject,
            first_week,
            last_week,
            weeks_per_block,
            ..
        } = row
        else {
            unreachable!("filtered above");
        };
        assert_eq!(*subject, built.maths);
        assert_eq!((first_week.0, last_week.0), (0, WEEK_COUNT - 1));
        assert_eq!(*weeks_per_block, 2);
    }
}

#[test]
fn a_subject_week_pattern_makes_the_blocks_tile() {
    // The same nine weeks, with the eighth switched off for maths only: eight
    // active weeks, four clean blocks, no infeasible row.
    let built = build_document(Scenario {
        maths_paused: true,
        ..Default::default()
    });
    let model = build_model(&built.params);

    assert!(
        block_infeasibilities(&model).is_empty(),
        "eight subject-active weeks tile into blocks of two, so no infeasible row \
         should be emitted, found {:?}",
        block_infeasibilities(&model),
    );

    // Four blocks, one of them stretching around the pause: the pairs are
    // (0,1) (2,3) (4,5) and (6,8).
    assert_eq!(
        exact_count_windows(&model, built.maths),
        BTreeSet::from([(0, 1), (2, 3), (4, 5), (6, 8)]),
        "the tiling should chunk the subject-active weeks, not the calendar ones",
    );
}

#[test]
fn a_paused_week_carries_no_interrogation_variable() {
    let built = build_document(Scenario {
        maths_paused: true,
        ..Default::default()
    });
    let model = build_model(&built.params);

    let maths_weeks = weeks_with_variables(&model, built.maths_slot);
    assert!(
        !maths_weeks.contains(&PAUSED_WEEK),
        "the subject's pattern switches week {PAUSED_WEEK} off, so nothing can be \
         scheduled there — yet the model has variables on it: {maths_weeks:?}",
    );
    assert_eq!(
        maths_weeks,
        (0..WEEK_COUNT).filter(|w| *w != PAUSED_WEEK).collect(),
        "every other week should still carry its variables",
    );

    // The pattern is per subject: physics does not wear it and keeps week 7.
    let physics_weeks = weeks_with_variables(&model, built.physics_slot);
    assert_eq!(
        physics_weeks,
        (0..WEEK_COUNT).collect::<BTreeSet<_>>(),
        "physics wears no pattern, so the pause must not touch it",
    );
}

#[test]
fn an_exactly_periodic_subject_keeps_one_run_across_the_pause() {
    // Physics runs every four weeks. With the pause, its eight active weeks are
    // 0..=6 and 8 — one uninterrupted run, because a subject pattern does not
    // cut the period the way an excluded period does.
    let built = build_document(Scenario {
        physics_paused: true,
        ..Default::default()
    });
    let model = build_model(&built.params);

    let windows = exact_count_windows(&model, built.physics);
    assert_eq!(
        windows,
        BTreeSet::from([(0, 3), (1, 4), (2, 5), (3, 6), (4, 8)]),
        "the run should slide over the subject-active weeks and stretch across the \
         pause, not stop at it",
    );

    // A cut run would show up as an infeasibility instead of windows.
    let cut = model
        .problem()
        .get_constraints()
        .iter()
        .any(|(_c, source)| {
            matches!(
                source,
                ConstraintSource::User(ConstraintDesc::Level0(
                    InfeasibleConstraint::PeriodicityExactlyPeriodicInfeasible { .. }
                ))
            )
        });
    assert!(
        !cut,
        "the run is eight weeks long, longer than the periodicity of four"
    );
}

#[test]
fn an_incompat_stops_at_a_paused_week() {
    // The incompat overlap scan looks at the slot's own pattern alone, so on
    // the paused week it forbids an interrogation the model has no variable
    // for: the build panics with `UndeclaredExtra`.
    let paused = build_document(Scenario {
        maths_paused: true,
        maths_incompat: true,
        ..Default::default()
    });
    let model = build_model(&paused.params);
    assert_eq!(
        incompat_saturated_weeks(&model),
        (0..WEEK_COUNT)
            .filter(|w| *w != PAUSED_WEEK)
            .collect::<BTreeSet<_>>(),
        "the paused week carries no interrogation to forbid",
    );

    // Without the pattern the same incompat covers all nine weeks, so the
    // filter is the pause and nothing else.
    let running = build_document(Scenario {
        maths_incompat: true,
        ..Default::default()
    });
    assert_eq!(
        incompat_saturated_weeks(&build_model(&running.params)),
        (0..WEEK_COUNT).collect::<BTreeSet<_>>(),
    );
}
