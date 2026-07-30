//! Unit tests for the `WeekOp` family (commit 2 of the WeekId split).
//!
//! These pin the guard and content-preservation behaviour of the week ops
//! through the apply gate deterministically — scenarios the property harness
//! exercises statistically but may not hit reliably on any given seed:
//!
//! * `Remove` blocked by a non-trivial week-pattern bit, and `Remove` + undo
//!   restoring the exact same week identity;
//! * `Update(false)` blocked by a non-empty colloscope cell;
//! * `Move` carrying a non-empty cell to a compatible period, and `Move`
//!   blocked when the destination period does not run the slot's subject;
//! * `Move` inside a week's *own* period — the branch whose bounds check uses
//!   the *post-detachment* length, so one position past the last week must be
//!   rejected rather than panic inside `Vec::insert` — plus the one-week edge
//!   that empties the ordering row in flight;
//! * removing the *first* week, whose reverse is the `AddFront` arm (the
//!   removal test above drops a middle week and pins only `AddAfter`).

use collomatique_state::{AppState, InMemoryData, traits::Manager};
use collomatique_state_colloscopes::{
    ColloscopeOp, Convergence, Data, Error, FixableInvariant, GroupListOp, InvalidOp, NewId,
    NonEmptyRangeInclusive, Op, PeriodOp, PrecheckError, Reference, SlotOp, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp,
    WeekOp, WeekPatternOp, WeekPrecheckError, WeekRefSite,
    group_lists::{GroupList, GroupListFilling, GroupListParameters},
    ids::{PeriodId, SlotId, SubjectId, TeacherId, WeekId},
    ops::{AnnotatedOp, AnnotatedWeekOp},
    slots::Slot,
    teachers::Teacher,
    week_patterns::WeekPattern,
    weeks::WeekDesc,
};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

fn interrogation_subject(name: &str, excluded: BTreeSet<PeriodId>) -> Subject {
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

fn week_ids_in(data: &Data, period: PeriodId) -> Vec<WeekId> {
    let params = &data.get_inner_data().params;
    let count = params.weeks.week_count_for_period(period).unwrap_or(0);
    (0..count)
        .map(|i| params.weeks.week_id_at(period, i).expect("valid position"))
        .collect()
}

fn week_ids_of(app: &AppState<Data, String>, period: PeriodId) -> Vec<WeekId> {
    week_ids_in(app.get_data(), period)
}

/// The assigned groups on the colloscope cell at `(slot, week-at-pos)`, read
/// through the sparse surface (`None` = empty/absent).
fn cell_at(
    app: &AppState<Data, String>,
    period: PeriodId,
    slot: SlotId,
    pos: usize,
) -> Option<BTreeSet<u32>> {
    let data = app.get_data();
    let inner = data.get_inner_data();
    let week_id = inner
        .params
        .weeks
        .week_id_at(period, pos)
        .expect("valid position");
    inner.colloscope.interrogation(slot, week_id).cloned()
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
    let Ok(Some(NewId::WeekPatternId(pattern_id))) = app.apply(
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
    assert_eq!(
        result,
        Err(Error::BrokenInvariants(BTreeSet::from([
            FixableInvariant::DanglingFk(Reference::Week {
                target: weeks[1],
                site: WeekRefSite::WeekPatternExcludedWeek(pattern_id),
            })
        ]))),
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
        data.get_inner_data().params.weeks.week_id_at(period, 1),
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
        Op::GroupList(GroupListOp::Add(
            GroupList::new(
                GroupListParameters {
                    name: "Liste".into(),
                    students_per_group: NonEmptyRangeInclusive::new(
                        NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                    )
                    .expect("statically non-empty"),
                    group_names: vec![None; 2],
                },
                GroupListFilling::default(),
            )
            .unwrap(),
        )),
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
    assert_eq!(
        result,
        Err(Error::BrokenInvariants(BTreeSet::from([
            FixableInvariant::Convergence(Convergence::InterrogationOnInactiveWeek(slot, weeks[0]))
        ]))),
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
        Op::GroupList(GroupListOp::Add(
            GroupList::new(
                GroupListParameters {
                    name: "Liste".into(),
                    students_per_group: NonEmptyRangeInclusive::new(
                        NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                    )
                    .expect("statically non-empty"),
                    group_names: vec![None; 2],
                },
                GroupListFilling::default(),
            )
            .unwrap(),
        )),
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
            .weeks
            .week_id_at(period_b, 0),
        Some(moved),
        "the moved week keeps its id at the destination",
    );
    assert_eq!(
        cell_at(&app, period_b, slot, 0),
        Some(BTreeSet::from([1])),
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
        Op::GroupList(GroupListOp::Add(
            GroupList::new(
                GroupListParameters {
                    name: "Liste".into(),
                    students_per_group: NonEmptyRangeInclusive::new(
                        NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                    )
                    .expect("statically non-empty"),
                    group_names: vec![None; 2],
                },
                GroupListFilling::default(),
            )
            .unwrap(),
        )),
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
    assert_eq!(
        result,
        Err(Error::BrokenInvariants(BTreeSet::from([
            FixableInvariant::Convergence(Convergence::InterrogationSlotNotRunningOnPeriod(
                slot, moved,
            )),
            FixableInvariant::Convergence(Convergence::InterrogationGroupOutOfBounds(
                slot, moved, 0,
            )),
        ]))),
        "moving a filled week to a period lacking the slot must fail, got {result:?}",
    );
}

/// `WeekOp::Move` with `dest_period == src_period` is a plain reorder, and the
/// one branch where the bounds check must look at the *post-detachment* length:
/// `dest_len_post` subtracts the week being moved, so the last legal position
/// in a three-week period is 2. Like the slot and subject reorders, the move
/// detaches then re-inserts, so the first week moved to position 2 lands last.
#[test]
fn move_week_within_its_own_period_then_undo() {
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

    // The boundary the `dest_len_post` adjustment exists for: with the week
    // detached the period holds two, so 2 is the last legal position and 3 is
    // one past the end. Without the adjustment 3 would be waved through and
    // then panic inside `Vec::insert`.
    let result = app.apply(
        Op::Week(WeekOp::Move(weeks[0], period, 3)),
        "Move past the end of its own period".into(),
    );
    assert_eq!(
        result,
        Err(Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Week(
            WeekPrecheckError::PositionOutOfBounds {
                period,
                position: 3,
                size: 2,
            }
        )))),
        "a same-period move must be bounded by the post-detachment length, got {result:?}",
    );

    // Reverse-then-forward via the raw Data path, mirroring Manager::apply.
    let mut data: Data = app.get_data().clone();
    let before = data.clone();

    let (annotated, _) = data.annotate(Op::Week(WeekOp::Move(weeks[0], period, 2)));
    let rev = data
        .apply(&annotated)
        .expect("a same-period move to the last position should succeed");

    assert_eq!(
        week_ids_in(&data, period),
        vec![weeks[1], weeks[2], weeks[0]],
        "the moved week is detached before `dest_pos` is honoured, so the \
         first of three moved to position 2 lands last",
    );
    assert_eq!(
        data.get_inner_data().params.weeks.week_position(weeks[0]),
        Some((period, 2)),
        "the week's own back-reference must agree with the ordering sidecar",
    );

    data.apply(&rev)
        .expect("the reverse of a successful op must apply");

    assert_eq!(
        week_ids_in(&data, period),
        vec![weeks[0], weeks[1], weeks[2]],
    );
    assert!(data == before, "move + undo must restore the prior state");
}

/// The transiently-empty-row edge: moving a period's *only* week to its own
/// position 0. Detaching empties the ordering row, so this is the one input
/// that runs `move_week_entry`'s empty-row path end to end and checks the
/// sparse canonical form survives it.
///
/// It does **not** discriminate the row-keepalive guard
/// (`order.is_empty() && src_period != dest_period`) from its absence, and the
/// plan that asked for this test was wrong to say it would: dropping the row
/// and letting the `else` branch re-create it at `dest_pos == 0` produces an
/// equal table, and deleting `&& src_period != dest_period` leaves the whole
/// suite green (checked by hand). The guard is defensive, not observable here;
/// what this test pins is the *outcome*, which is what a consumer sees.
#[test]
fn move_a_periods_only_week_to_its_own_position() {
    let mut app = AppState::<_, String>::new(Data::new());

    let period = add_period(&mut app, None, vec![WeekDesc::new(true)]);
    let weeks = week_ids_of(&app, period);

    let mut data: Data = app.get_data().clone();
    let before = data.clone();

    let (annotated, _) = data.annotate(Op::Week(WeekOp::Move(weeks[0], period, 0)));
    let rev = data
        .apply(&annotated)
        .expect("moving a period's only week onto its own position should succeed");

    assert_eq!(
        week_ids_in(&data, period),
        vec![weeks[0]],
        "the week is back in its own row, which was never dropped",
    );
    assert!(
        data == before,
        "a move onto the week's own position changes nothing",
    );

    data.apply(&rev)
        .expect("the reverse of a successful op must apply");

    assert_eq!(week_ids_in(&data, period), vec![weeks[0]]);
    assert!(data == before);
}

/// Removing a period's *first* week: the reverse is the `AddFront` arm, which
/// no other deterministic test reaches — `remove_week_then_undo_restores_identity`
/// removes a middle week and so pins only `AddAfter`.
#[test]
fn remove_first_week_then_undo_restores_identity() {
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

    let mut data: Data = app.get_data().clone();
    let before = data.clone();

    let (annotated, _) = data.annotate(Op::Week(WeekOp::Remove(weeks[0])));
    let rev = data
        .apply(&annotated)
        .expect("removing a trivially-active week should succeed");

    assert_eq!(
        rev,
        AnnotatedOp::Week(AnnotatedWeekOp::AddFront(
            weeks[0],
            period,
            WeekDesc::new(true),
        )),
        "removing the first week must reverse through the `AddFront` arm",
    );
    assert_eq!(week_ids_in(&data, period), vec![weeks[1], weeks[2]]);

    data.apply(&rev)
        .expect("the reverse of a successful op must apply");

    assert_eq!(
        week_ids_in(&data, period),
        vec![weeks[0], weeks[1], weeks[2]],
        "the restored week must keep its original id at its original position",
    );
    assert!(data == before, "remove + undo must restore the prior state");
}
