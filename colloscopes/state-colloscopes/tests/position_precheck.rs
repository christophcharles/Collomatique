//! Precedence pins for the unified position precheck.
//!
//! The three reorder ops (`WeekOp::Move`, `SlotOp::ChangePosition`,
//! `SubjectOp::ChangePosition`) all reject an out-of-range destination with
//! their entity's `PositionOutOfBounds`. When the op is *doubly* bad — a
//! dangling target **and** an impossible position — the diagnostic the user
//! needs is the dangling target: the position is meaningless for an entity
//! that is not there. These tests pin that order for each entity.

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    Data, Error, InvalidOp, NewId, NonEmptyRangeInclusive, Op, PeriodOp, PrecheckError, SlotOp,
    SlotPrecheckError, Subject, SubjectInterrogationParameters, SubjectOp, SubjectParameters,
    SubjectPeriodicity, SubjectPrecheckError, TeacherOp, WeekOp, WeekPrecheckError,
    ids::{SubjectId, TeacherId},
    slots::Slot,
    teachers::Teacher,
    weeks::WeekDesc,
};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

/// A position far beyond any list these fixtures build, so the bounds check
/// would certainly fire if it ran first.
const WAY_PAST_THE_END: usize = 99;

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
        week_pattern: None,
    }
}

fn make_slot(subject_id: SubjectId, teacher_id: TeacherId) -> Slot {
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
        week_pattern: None,
        cost: 0,
    }
}

fn add_subject(app: &mut AppState<Data, String>, name: &str) -> SubjectId {
    match app.apply(
        Op::Subject(SubjectOp::AddAfter(None, interrogation_subject(name))),
        "Add subject".into(),
    ) {
        Ok(Some(NewId::SubjectId(id))) => id,
        other => panic!("adding a subject should return a subject id, got {other:?}"),
    }
}

/// Removes the entity again, so its id is dead but genuinely issued — no
/// forgery needed to obtain a dangling target.
fn dead_subject(app: &mut AppState<Data, String>) -> SubjectId {
    let doomed = add_subject(app, "Doomed");
    app.apply(Op::Subject(SubjectOp::Remove(doomed)), "Remove".into())
        .expect("removing an unreferenced subject should succeed");
    doomed
}

/// `SubjectOp::ChangePosition` on a removed subject with an impossible
/// position reports the subject, not the position.
#[test]
fn a_doubly_bad_subject_reorder_reports_the_dangling_subject() {
    let mut app = AppState::<_, String>::new(Data::new());

    // One live subject, so the list is non-empty and 99 is out of range.
    let _live = add_subject(&mut app, "Math");
    let dead = dead_subject(&mut app);

    let result = app.apply(
        Op::Subject(SubjectOp::ChangePosition(dead, WAY_PAST_THE_END)),
        "Reorder a removed subject".into(),
    );

    assert_eq!(
        result,
        Err(Error::InvalidOp(InvalidOp::Precheck(
            PrecheckError::Subject(SubjectPrecheckError::InvalidSubjectId(dead))
        ))),
        "the dangling subject must win over the out-of-range position, got {result:?}",
    );
}

/// `SlotOp::ChangePosition` on a removed slot with an impossible position
/// reports the slot, not the position.
#[test]
fn a_doubly_bad_slot_reorder_reports_the_dangling_slot() {
    let mut app = AppState::<_, String>::new(Data::new());

    let subject = add_subject(&mut app, "Math");
    let Ok(Some(NewId::TeacherId(teacher))) = app.apply(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        "Add teacher".into(),
    ) else {
        panic!("adding a teacher should return a teacher id");
    };
    let Ok(Some(NewId::SlotId(dead))) = app.apply(
        Op::Slot(SlotOp::AddAfter(None, make_slot(subject, teacher))),
        "Add slot".into(),
    ) else {
        panic!("adding a slot should return a slot id");
    };
    app.apply(Op::Slot(SlotOp::Remove(dead)), "Remove slot".into())
        .expect("removing an unreferenced slot should succeed");

    let result = app.apply(
        Op::Slot(SlotOp::ChangePosition(dead, WAY_PAST_THE_END)),
        "Reorder a removed slot".into(),
    );

    assert_eq!(
        result,
        Err(Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Slot(
            SlotPrecheckError::InvalidSlotId(dead)
        )))),
        "the dangling slot must win over the out-of-range position, got {result:?}",
    );
}

/// `WeekOp::Move` on a removed week with an impossible destination position
/// reports the week, not the position. (The destination period is live here,
/// so the only competing check is the bounds one.)
#[test]
fn a_doubly_bad_week_move_reports_the_dangling_week() {
    let mut app = AppState::<_, String>::new(Data::new());

    let Ok(Some(NewId::PeriodId(period))) = app.apply(Op::Period(PeriodOp::AddFront), "Add".into())
    else {
        panic!("adding a period should return a period id");
    };
    let Ok(Some(NewId::WeekId(dead))) = app.apply(
        Op::Week(WeekOp::AddFront(period, WeekDesc::new(true))),
        "Add week".into(),
    ) else {
        panic!("adding a week should return a week id");
    };
    app.apply(Op::Week(WeekOp::Remove(dead)), "Remove week".into())
        .expect("removing a trivially-active week should succeed");

    let result = app.apply(
        Op::Week(WeekOp::Move(dead, period, WAY_PAST_THE_END)),
        "Move a removed week".into(),
    );

    assert_eq!(
        result,
        Err(Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Week(
            WeekPrecheckError::InvalidWeekId(dead)
        )))),
        "the dangling week must win over the out-of-range position, got {result:?}",
    );
}

/// The bounds check itself still fires — and now names its scope — when the
/// target is live. This keeps the precedence pins above honest: they pass
/// because existence is tested first, not because the bounds check is dead.
#[test]
fn a_live_target_with_an_impossible_position_reports_the_bounds() {
    let mut app = AppState::<_, String>::new(Data::new());

    let subject = add_subject(&mut app, "Math");
    let result = app.apply(
        Op::Subject(SubjectOp::ChangePosition(subject, WAY_PAST_THE_END)),
        "Reorder past the end".into(),
    );

    assert_eq!(
        result,
        Err(Error::InvalidOp(InvalidOp::Precheck(
            PrecheckError::Subject(SubjectPrecheckError::PositionOutOfBounds {
                position: WAY_PAST_THE_END,
                size: 1,
            })
        ))),
        "a live subject with an impossible position must report the bounds, got {result:?}",
    );

    // Sanity: nothing moved.
    assert_eq!(
        app.get_data()
            .get_inner_data()
            .params
            .subjects
            .find_subject_position(subject),
        Some(0),
    );
}
