//! Spans that require an interrogation but hold no slot at all.
//!
//! `count_interrogations_expr` sums the slot-week pairs that fall inside a
//! span. When the slots' week patterns empty a span, that sum is the constant
//! zero, and the periodicity families still write `0 == 1` (or `0 >= 1`) over
//! it. The model is then infeasible with no variable involved, and the blame
//! shown to the user is a `Level3` count row that says nothing about *why*
//! nothing fits.
//!
//! [`InfeasibleConstraint::NoSlotsForWeekSpan`] replaces those rows with a
//! `Level0` marker naming the subject, the student and the week span.
//!
//! Two shapes are covered here, both taken from a real document:
//!
//! * a block whose weeks are all excluded by the slot's pattern, while the
//!   tiling itself is perfectly fine — the block count row is an `eq(1)`;
//! * a subject whose slot is excluded on *every* week of the year, so the
//!   whole-year row is a `geq(count_min)`.

use std::collections::BTreeSet;
use std::num::NonZeroU32;

use collomatique_constraints_colloscopes::{
    ColloscopeModel, ConstraintDesc, ConstraintSource, InfeasibleConstraint, ProgressiveConstraint,
    build_model,
};
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    AssignmentOp, Data, GroupListOp, NewId, NonEmptyRangeInclusive, Op, PeriodOp, SlotOp,
    StudentOp, Subject, SubjectInterrogationParameters, SubjectOp, SubjectParameters,
    SubjectPeriodicity, TeacherOp, WeekOp, WeekPatternOp,
    colloscope_params::Parameters,
    group_lists::{GroupList, GroupListFilling, GroupListParameters},
    ids::{SubjectId, WeekPatternId},
    slots::Slot,
    students::Student,
    teachers::Teacher,
    week_patterns::WeekPattern,
    weeks::WeekDesc,
};

/// Weeks in the single period. Four, so blocks of two tile it exactly.
const WEEK_COUNT: usize = 4;

/// Enrolled students. Every marker is per-student, so this is how many rows
/// each empty span should produce.
const STUDENT_COUNT: usize = 2;

fn nz(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("value should be non-zero")
}

fn ner<T: Ord + Clone>(range: std::ops::RangeInclusive<T>) -> NonEmptyRangeInclusive<T> {
    NonEmptyRangeInclusive::new(range).expect("range should be non-empty")
}

fn subject(name: &str, periodicity: SubjectPeriodicity) -> Subject {
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
        week_pattern: None,
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
    week_pattern: WeekPatternId,
) -> Slot {
    Slot {
        subject_id,
        teacher_id,
        start_time: slot_start(weekday),
        extra_info: String::new(),
        week_pattern: Some(week_pattern),
        cost: 0,
    }
}

/// What the tests need to point at, once the document is built.
struct Built {
    params: Parameters,
    info: SubjectId,
    year: SubjectId,
}

/// One four-week period, two interrogated subjects sharing a teacher and a
/// group list, two students enrolled in both.
///
/// « Info » tiles into the blocks (0,1) and (2,3), but its slot wears a
/// pattern that switches weeks 2 and 3 off: the second block holds no
/// slot-week. « Année » wants one interrogation in the year and its slot is
/// switched off on every week.
fn build_document() -> Built {
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

    let first_half = apply_new!(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "Premier semestre".into(),
            excluded_weeks: BTreeSet::from([weeks[2], weeks[3]]),
        })),
        NewId::WeekPatternId,
        "add the half-year pattern"
    );
    let never = apply_new!(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "Jamais".into(),
            excluded_weeks: weeks.iter().copied().collect(),
        })),
        NewId::WeekPatternId,
        "add the empty pattern"
    );

    let info = apply_new!(
        Op::Subject(SubjectOp::AddAfter(
            None,
            subject(
                "Info",
                SubjectPeriodicity::OnceForEveryBlockOfWeeks {
                    weeks_per_block: nz(2),
                    minimum_week_separation: nz(1),
                },
            )
        )),
        NewId::SubjectId,
        "add info"
    );
    let year = apply_new!(
        Op::Subject(SubjectOp::AddAfter(
            Some(info),
            subject(
                "Année",
                SubjectPeriodicity::AmountInYear {
                    interrogation_count_in_year: ner(1..=2),
                    minimum_week_separation: 0,
                },
            )
        )),
        NewId::SubjectId,
        "add year"
    );

    let teacher = apply_new!(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([info, year]),
        })),
        NewId::TeacherId,
        "add teacher"
    );
    apply_ok!(
        Op::Slot(SlotOp::AddAfter(
            None,
            slot(info, teacher, chrono::Weekday::Mon, first_half)
        )),
        "add info slot"
    );
    apply_ok!(
        Op::Slot(SlotOp::AddAfter(
            None,
            slot(year, teacher, chrono::Weekday::Tue, never)
        )),
        "add year slot"
    );

    let mut students = BTreeSet::new();
    for _ in 0..STUDENT_COUNT {
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

    for subject_id in [info, year] {
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

    Built {
        params: app.get_data().get_inner_data().params.clone(),
        info,
        year,
    }
}

/// Every `NoSlotsForWeekSpan` row `subject` carries, as
/// `(first_week, last_week, required_count)`.
fn no_slots_rows(model: &ColloscopeModel, subject: SubjectId) -> Vec<(usize, usize, u32)> {
    model
        .problem()
        .get_constraints()
        .iter()
        .filter_map(|(_constraint, source)| match source {
            ConstraintSource::User(ConstraintDesc::Level0(
                InfeasibleConstraint::NoSlotsForWeekSpan {
                    subject: s,
                    first_week,
                    last_week,
                    required_count,
                    ..
                },
            )) if *s == subject => Some((first_week.0, last_week.0, *required_count)),
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

/// Every `(first_week, last_week)` a `PeriodicityInterrogationCountMin` row
/// covers for `subject`.
fn min_count_windows(model: &ColloscopeModel, subject: SubjectId) -> BTreeSet<(usize, usize)> {
    model
        .problem()
        .get_constraints()
        .iter()
        .filter_map(|(_constraint, source)| match source {
            ConstraintSource::User(ConstraintDesc::Level3(
                ProgressiveConstraint::PeriodicityInterrogationCountMin {
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

#[test]
fn empty_block_gets_a_no_slots_row() {
    let built = build_document();
    let model = build_model(&built.params);

    assert_eq!(
        no_slots_rows(&model, built.info),
        vec![(2, 3, 1); STUDENT_COUNT],
        "the second block holds no slot-week, so every enrolled student should get \
         a marker naming that span",
    );
    assert!(
        !exact_count_windows(&model, built.info).contains(&(2, 3)),
        "the marker replaces the count row it stands for, and that row would be an \
         empty sum forced to one: {:?}",
        exact_count_windows(&model, built.info),
    );
}

#[test]
fn slotless_year_gets_a_no_slots_row() {
    let built = build_document();
    let model = build_model(&built.params);

    assert_eq!(
        no_slots_rows(&model, built.year),
        vec![(0, WEEK_COUNT - 1, 1); STUDENT_COUNT],
        "the subject's only slot is switched off on every week, so the whole-year \
         minimum should be reported as an empty span",
    );
    assert!(
        min_count_windows(&model, built.year).is_empty(),
        "the marker replaces the minimum row it stands for: {:?}",
        min_count_windows(&model, built.year),
    );
}

#[test]
fn healthy_block_keeps_its_count_row() {
    let built = build_document();
    let model = build_model(&built.params);

    assert!(
        exact_count_windows(&model, built.info).contains(&(0, 1)),
        "the first block does hold slot-weeks, so it should keep its count row: {:?}",
        exact_count_windows(&model, built.info),
    );
}
