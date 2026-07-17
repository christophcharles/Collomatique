//! Unit tests for the `WeekOp` family (commit 2 of the WeekId split).
//!
//! These pin the guard and content-preservation behaviour of `apply_week`
//! deterministically — scenarios the property harness exercises statistically
//! but may not hit reliably on any given seed:
//!
//! * `Remove` blocked by a non-trivial week-pattern bit, and `Remove` + undo
//!   restoring the exact same week identity;
//! * `Update(false)` blocked by a non-empty colloscope cell;
//! * `Move` carrying a non-empty cell to a compatible period, and `Move`
//!   blocked when the destination period does not run the slot's subject.

use collomatique_state::{AppState, InMemoryData, traits::Manager};
use collomatique_state_colloscopes::{
    ColloscopeOp, Data, Error, GroupListOp, NewId, Op, PeriodOp, SlotOp, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp,
    WeekError, WeekOp, WeekPatternOp,
    colloscopes::ColloscopeInterrogation,
    group_lists::GroupListParameters,
    ids::{PeriodId, SlotId, SubjectId, TeacherId, WeekId},
    periods::WeekDesc,
    slots::Slot,
    teachers::Teacher,
    week_patterns::WeekPattern,
};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

fn interrogation_subject(name: &str, excluded: BTreeSet<PeriodId>) -> Subject {
    Subject {
        parameters: SubjectParameters {
            name: name.into(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                groups_per_interrogation: NonZeroU32::new(1).unwrap()..=NonZeroU32::new(1).unwrap(),
                duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
                },
            }),
        },
        excluded_periods: excluded,
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

/// Creates a period (at the front, or after `after`) carrying `weeks`, one
/// spliced in at a time via the `WeekOp` family — periods are created empty.
fn add_period(
    app: &mut AppState<Data, String>,
    after: Option<PeriodId>,
    weeks: Vec<WeekDesc>,
) -> PeriodId {
    let period_op = match after {
        None => PeriodOp::AddFront,
        Some(a) => PeriodOp::AddAfter(a),
    };
    let period = match app.apply(Op::Period(period_op), "Add period".into()) {
        Ok(Some(NewId::PeriodId(id))) => id,
        other => panic!("adding a period should return a period id, got {other:?}"),
    };
    let mut prev: Option<WeekId> = None;
    for desc in weeks {
        let week_op = match prev {
            None => WeekOp::AddFront(period, desc),
            Some(w) => WeekOp::AddAfter(w, desc),
        };
        match app.apply(Op::Week(week_op), "Add week".into()) {
            Ok(Some(NewId::WeekId(w))) => prev = Some(w),
            other => panic!("adding a week should return a week id, got {other:?}"),
        }
    }
    period
}

fn week_ids_of(app: &AppState<Data, String>, period: PeriodId) -> Vec<WeekId> {
    let params = &app.get_data().get_inner_data().params;
    let count = params.periods.week_count_of(period).expect("valid period");
    (0..count)
        .map(|i| {
            params
                .periods
                .week_id_at(period, i)
                .expect("valid position")
        })
        .collect()
}

fn cell_at(
    app: &AppState<Data, String>,
    period: PeriodId,
    slot: SlotId,
    pos: usize,
) -> Option<ColloscopeInterrogation> {
    app.get_data()
        .get_inner_data()
        .colloscope
        .period_map
        .get(&period)
        .expect("valid period")
        .slot_map
        .get(&slot)
        .expect("valid slot")
        .interrogations[pos]
        .clone()
}

/// `WeekOp::Remove` must refuse to drop a week that a week pattern marks
/// inactive: undo re-adds a trivially-active week, so a non-trivial bit could
/// not be restored. Same guard family as the old whole-period shrink.
#[test]
fn remove_week_blocked_by_non_trivial_pattern() {
    let mut app = AppState::<_, String>::new(Data::new());

    let period = add_period(
        &mut app,
        None,
        vec![
            WeekDesc::new(true),
            WeekDesc::new(true),
            WeekDesc::new(true),
        ],
    );

    let weeks = week_ids_of(&app, period);

    // A pattern that skips the middle week.
    let Ok(Some(NewId::WeekPatternId(_))) = app.apply(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "skip middle".into(),
            excluded_weeks: std::collections::BTreeSet::from([weeks[1]]),
        })),
        "Add week pattern".into(),
    ) else {
        panic!("adding a week pattern should return a week pattern id");
    };

    let result = app.apply(
        Op::Week(WeekOp::Remove(weeks[1])),
        "Remove middle week".into(),
    );
    assert!(
        matches!(
            result,
            Err(Error::Week(WeekError::NonTrivialWeekPattern(w, _))) if w == weeks[1]
        ),
        "removing a week a pattern skips must fail, got {result:?}",
    );

    // The trivially-active outer weeks can still be removed.
    let Ok(None) = app.apply(
        Op::Week(WeekOp::Remove(weeks[2])),
        "Remove last week".into(),
    ) else {
        panic!("removing a trivially-active week should succeed");
    };
}

/// `WeekOp::Remove` followed by its own reverse restores the exact prior
/// state — same week id, same position — as required by the history-replay
/// invariant.
#[test]
fn remove_week_then_undo_restores_identity() {
    let mut app = AppState::<_, String>::new(Data::new());

    let period = add_period(
        &mut app,
        None,
        vec![
            WeekDesc::new(true),
            WeekDesc::new(true),
            WeekDesc::new(true),
        ],
    );

    let weeks = week_ids_of(&app, period);
    let middle = weeks[1];

    // Reverse-then-forward via the raw Data path, mirroring Manager::apply.
    let mut data: Data = app.get_data().clone();
    let before = data.clone();

    let (annotated, _) = data.annotate(Op::Week(WeekOp::Remove(middle)));
    let rev = data
        .apply(&annotated)
        .expect("removing the week should succeed");
    data.apply(&rev)
        .expect("the reverse of a successful op must apply");

    assert!(data == before, "remove + undo must restore the prior state");
    assert_eq!(
        data.get_inner_data().params.periods.week_id_at(period, 1),
        Some(middle),
        "the restored week must keep its original id at its original position",
    );
}

/// `WeekOp::Update` that turns interrogations off must be refused when the
/// week's colloscope cell is non-empty — the same silencing guard the whole
/// period update enforced.
#[test]
fn update_week_to_inactive_blocked_by_filled_cell() {
    let mut app = AppState::<_, String>::new(Data::new());

    let period = add_period(
        &mut app,
        None,
        vec![WeekDesc::new(true), WeekDesc::new(true)],
    );
    let Ok(Some(NewId::SubjectId(subject))) = app.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new()),
        )),
        "Add subject".into(),
    ) else {
        panic!("adding a subject should return a subject id");
    };
    let Ok(Some(NewId::TeacherId(teacher))) = app.apply(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        "Add teacher".into(),
    ) else {
        panic!("adding a teacher should return a teacher id");
    };
    let Ok(Some(NewId::SlotId(slot))) = app.apply(
        Op::Slot(SlotOp::AddAfter(None, make_slot(subject, teacher))),
        "Add slot".into(),
    ) else {
        panic!("adding a slot should return a slot id");
    };
    let Ok(Some(NewId::GroupListId(group_list))) = app.apply(
        Op::GroupList(GroupListOp::Add(GroupListParameters {
            name: "Liste".into(),
            students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            group_names: vec![None; 2],
        })),
        "Add group list".into(),
    ) else {
        panic!("adding a group list should return a group list id");
    };
    let Ok(None) = app.apply(
        Op::GroupList(GroupListOp::AssignToSubject(
            period,
            subject,
            Some(group_list),
        )),
        "Associate group list".into(),
    ) else {
        panic!("associating the group list should succeed");
    };

    let weeks = week_ids_of(&app, period);
    // Fill the cell of the first week.
    let Ok(None) = app.apply(
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot,
            weeks[0],
            BTreeSet::from([1]),
        )),
        "Fill interrogation".into(),
    ) else {
        panic!("filling the interrogation should succeed");
    };

    // Turning the week inactive would silence the non-empty cell.
    let result = app.apply(
        Op::Week(WeekOp::Update(weeks[0], WeekDesc::new(false))),
        "Deactivate week".into(),
    );
    assert!(
        matches!(
            result,
            Err(Error::Week(WeekError::NotCompatibleSlotInColloscope(w, s)))
                if w == weeks[0] && s == slot
        ),
        "deactivating a week with a filled cell must fail, got {result:?}",
    );

    // The second week is empty, so it can be deactivated.
    let Ok(None) = app.apply(
        Op::Week(WeekOp::Update(weeks[1], WeekDesc::new(false))),
        "Deactivate empty week".into(),
    ) else {
        panic!("deactivating an empty week should succeed");
    };
}

/// `WeekOp::Move` carries a week's colloscope content to the destination
/// period when the slot runs there and the groups fit the association bounds.
#[test]
fn move_week_preserves_filled_cell() {
    let mut app = AppState::<_, String>::new(Data::new());

    let period_a = add_period(
        &mut app,
        None,
        vec![WeekDesc::new(true), WeekDesc::new(true)],
    );
    let period_b = add_period(
        &mut app,
        Some(period_a),
        vec![WeekDesc::new(true), WeekDesc::new(true)],
    );
    // Subject runs on both periods.
    let Ok(Some(NewId::SubjectId(subject))) = app.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new()),
        )),
        "Add subject".into(),
    ) else {
        panic!("adding a subject should return a subject id");
    };
    let Ok(Some(NewId::TeacherId(teacher))) = app.apply(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        "Add teacher".into(),
    ) else {
        panic!("adding a teacher should return a teacher id");
    };
    let Ok(Some(NewId::SlotId(slot))) = app.apply(
        Op::Slot(SlotOp::AddAfter(None, make_slot(subject, teacher))),
        "Add slot".into(),
    ) else {
        panic!("adding a slot should return a slot id");
    };
    let Ok(Some(NewId::GroupListId(group_list))) = app.apply(
        Op::GroupList(GroupListOp::Add(GroupListParameters {
            name: "Liste".into(),
            students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            group_names: vec![None; 2],
        })),
        "Add group list".into(),
    ) else {
        panic!("adding a group list should return a group list id");
    };
    // Associate on both periods so a filled cell can travel.
    let Ok(None) = app.apply(
        Op::GroupList(GroupListOp::AssignToSubject(
            period_a,
            subject,
            Some(group_list),
        )),
        "Associate on A".into(),
    ) else {
        panic!("associating on A should succeed");
    };
    let Ok(None) = app.apply(
        Op::GroupList(GroupListOp::AssignToSubject(
            period_b,
            subject,
            Some(group_list),
        )),
        "Associate on B".into(),
    ) else {
        panic!("associating on B should succeed");
    };

    let weeks_a = week_ids_of(&app, period_a);
    let moved = weeks_a[1];
    // Fill the cell of the moved week.
    let Ok(None) = app.apply(
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot,
            moved,
            BTreeSet::from([1]),
        )),
        "Fill interrogation".into(),
    ) else {
        panic!("filling the interrogation should succeed");
    };

    // Move the filled week to the front of period B.
    let Ok(None) = app.apply(
        Op::Week(WeekOp::Move(moved, period_b, 0)),
        "Move week to B".into(),
    ) else {
        panic!("moving the week should succeed");
    };

    // Its content survived the move.
    assert_eq!(
        app.get_data()
            .get_inner_data()
            .params
            .periods
            .week_id_at(period_b, 0),
        Some(moved),
        "the moved week keeps its id at the destination",
    );
    assert_eq!(
        cell_at(&app, period_b, slot, 0),
        Some(ColloscopeInterrogation {
            assigned_groups: BTreeSet::from([1]),
        }),
        "the filled cell must travel with the week",
    );
    // The source no longer holds the week.
    assert_eq!(week_ids_of(&app, period_a), vec![weeks_a[0]]);
}

/// `WeekOp::Move` must refuse to carry a non-empty cell to a period that does
/// not run the slot's subject (the content has nowhere to land).
#[test]
fn move_week_blocked_when_destination_lacks_slot() {
    let mut app = AppState::<_, String>::new(Data::new());

    let period_a = add_period(
        &mut app,
        None,
        vec![WeekDesc::new(true), WeekDesc::new(true)],
    );
    let period_b = add_period(
        &mut app,
        Some(period_a),
        vec![WeekDesc::new(true), WeekDesc::new(true)],
    );
    // Subject runs on A only (excluded from B), so B has no slot for it.
    let Ok(Some(NewId::SubjectId(subject))) = app.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::from([period_b])),
        )),
        "Add subject".into(),
    ) else {
        panic!("adding a subject should return a subject id");
    };
    let Ok(Some(NewId::TeacherId(teacher))) = app.apply(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        "Add teacher".into(),
    ) else {
        panic!("adding a teacher should return a teacher id");
    };
    let Ok(Some(NewId::SlotId(slot))) = app.apply(
        Op::Slot(SlotOp::AddAfter(None, make_slot(subject, teacher))),
        "Add slot".into(),
    ) else {
        panic!("adding a slot should return a slot id");
    };
    let Ok(Some(NewId::GroupListId(group_list))) = app.apply(
        Op::GroupList(GroupListOp::Add(GroupListParameters {
            name: "Liste".into(),
            students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
            group_names: vec![None; 2],
        })),
        "Add group list".into(),
    ) else {
        panic!("adding a group list should return a group list id");
    };
    let Ok(None) = app.apply(
        Op::GroupList(GroupListOp::AssignToSubject(
            period_a,
            subject,
            Some(group_list),
        )),
        "Associate on A".into(),
    ) else {
        panic!("associating on A should succeed");
    };

    let weeks_a = week_ids_of(&app, period_a);
    let moved = weeks_a[0];
    let Ok(None) = app.apply(
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot,
            moved,
            BTreeSet::from([0]),
        )),
        "Fill interrogation".into(),
    ) else {
        panic!("filling the interrogation should succeed");
    };

    let result = app.apply(
        Op::Week(WeekOp::Move(moved, period_b, 0)),
        "Move filled week to B".into(),
    );
    assert!(
        matches!(
            result,
            Err(Error::Week(WeekError::NotCompatibleSlotInColloscope(w, s)))
                if w == moved && s == slot
        ),
        "moving a filled week to a period lacking the slot must fail, got {result:?}",
    );
}
