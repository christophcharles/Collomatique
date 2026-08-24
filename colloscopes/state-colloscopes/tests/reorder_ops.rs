//! Deterministic pins for the entity reorder ops and the first-element undo
//! edge, on the two entities whose lists are *not* the weeks (which
//! `week_ops.rs` covers).
//!
//! The property harness walks these paths statistically; pinning them by hand
//! matters because each hides an off-by-one that a fuzz mismatch would report
//! opaquely:
//!
//! * `SlotOp::ChangePosition` and `SubjectOp::ChangePosition` both move an
//!   entry by **detaching it and re-inserting it**, so `new_pos` indexes the
//!   list *after* the entry is gone. Moving the first of three entries to
//!   position 2 therefore lands it **last**, not second-to-last — and the
//!   reverse op the gate returns carries the pre-move position, which under
//!   the same rule puts it back.
//! * Removing a subject's *first* slot is the one removal whose reverse is the
//!   `AddAfter(id, None, slot)` arm (`None` = "place first"). Every other
//!   deterministic removal test drops a middle element, so it pins only the
//!   `Some(previous)` anchor.

use collomatique_state::{AppState, InMemoryData, traits::Manager};
use collomatique_state_colloscopes::{
    Data, NewId, NonEmptyRangeInclusive, Op, SlotOp, Subject, SubjectInterrogationParameters,
    SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp,
    ids::{SlotId, SubjectId, TeacherId},
    ops::{AnnotatedOp, AnnotatedSlotOp},
    slots::Slot,
    teachers::Teacher,
};
use std::collections::BTreeSet;
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

/// A slot starting at `hour`:00 — the hour is what tells the three fixture
/// slots apart in a failure message.
fn make_slot_at(subject_id: SubjectId, teacher_id: TeacherId, hour: u32) -> Slot {
    Slot {
        subject_id,
        teacher_id,
        start_time: collomatique_time::SlotStart {
            weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
            start_time: collomatique_time::WholeMinuteTime::new(
                chrono::NaiveTime::from_hms_opt(hour, 0, 0).unwrap(),
            )
            .unwrap(),
        },
        extra_info: String::new(),
        week_pattern: None,
        cost: 0,
    }
}

fn add_subject(
    app: &mut AppState<Data, String>,
    after: Option<SubjectId>,
    name: &str,
) -> SubjectId {
    match app.apply(
        Op::Subject(SubjectOp::AddAfter(after, interrogation_subject(name))),
        "Add subject".into(),
    ) {
        Ok(Some(NewId::SubjectId(id))) => id,
        other => panic!("adding a subject should return a subject id, got {other:?}"),
    }
}

fn add_teacher(app: &mut AppState<Data, String>, subject: SubjectId) -> TeacherId {
    match app.apply(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        "Add teacher".into(),
    ) {
        Ok(Some(NewId::TeacherId(id))) => id,
        other => panic!("adding a teacher should return a teacher id, got {other:?}"),
    }
}

/// Appends one slot at the end of `subject`'s list.
fn add_slot(
    app: &mut AppState<Data, String>,
    after: Option<SlotId>,
    subject: SubjectId,
    teacher: TeacherId,
    hour: u32,
) -> SlotId {
    match app.apply(
        Op::Slot(SlotOp::AddAfter(
            after,
            make_slot_at(subject, teacher, hour),
        )),
        "Add slot".into(),
    ) {
        Ok(Some(NewId::SlotId(id))) => id,
        other => panic!("adding a slot should return a slot id, got {other:?}"),
    }
}

/// A subject carrying three slots, at 8:00, 9:00 and 10:00 in that order.
fn subject_with_three_slots(
    app: &mut AppState<Data, String>,
) -> (SubjectId, TeacherId, Vec<SlotId>) {
    let subject = add_subject(app, None, "Math");
    let teacher = add_teacher(app, subject);

    let mut slots = Vec::new();
    let mut previous = None;
    for hour in [8, 9, 10] {
        let slot = add_slot(app, previous, subject, teacher, hour);
        previous = Some(slot);
        slots.push(slot);
    }
    (subject, teacher, slots)
}

fn slot_ids_of(data: &Data, subject: SubjectId) -> Vec<SlotId> {
    data.get_inner_data()
        .params
        .slots
        .slots_for_subject(subject)
        .expect("the fixture subject has slots")
        .map(|(slot_id, _)| *slot_id)
        .collect()
}

fn subject_ids(data: &Data) -> Vec<SubjectId> {
    data.get_inner_data()
        .params
        .subjects
        .ordered_subject_list
        .keys()
        .collect()
}

/// `SlotOp::ChangePosition` moves by detach-then-insert, so moving the first
/// of three slots to position 2 lands it last. The reverse op restores the
/// original order exactly.
#[test]
fn change_slot_position_then_undo_restores_the_order() {
    let mut app = AppState::<_, String>::new(Data::new());
    let (subject, _teacher, slots) = subject_with_three_slots(&mut app);

    // Reverse-then-forward via the raw Data path, mirroring Manager::apply.
    let mut data: Data = app.get_data().clone();
    let before = data.clone();

    let (annotated, _) = data.annotate(Op::Slot(SlotOp::ChangePosition(slots[0], 2)));
    let rev = data
        .apply(&annotated)
        .expect("moving a slot inside its own subject should succeed");

    assert_eq!(
        slot_ids_of(&data, subject),
        vec![slots[1], slots[2], slots[0]],
        "`new_pos` indexes the list with the moved slot already detached, so \
         the first of three moved to position 2 lands last",
    );
    assert_eq!(
        data.get_inner_data()
            .params
            .slots
            .find_slot_subject_and_position(slots[0]),
        Some((subject, 2)),
        "the slot's own back-reference must agree with the ordering sidecar",
    );

    data.apply(&rev)
        .expect("the reverse of a successful op must apply");

    assert_eq!(
        slot_ids_of(&data, subject),
        vec![slots[0], slots[1], slots[2]],
        "the reverse carries the pre-move position, which puts the slot back",
    );
    assert!(data == before, "move + undo must restore the prior state");
}

/// `SubjectOp::ChangePosition` has the same detach-then-insert semantics over
/// the global subject list.
#[test]
fn change_subject_position_then_undo_restores_the_order() {
    let mut app = AppState::<_, String>::new(Data::new());

    let math = add_subject(&mut app, None, "Math");
    let physics = add_subject(&mut app, Some(math), "Physics");
    let chemistry = add_subject(&mut app, Some(physics), "Chemistry");

    let mut data: Data = app.get_data().clone();
    let before = data.clone();
    assert_eq!(subject_ids(&data), vec![math, physics, chemistry]);

    let (annotated, _) = data.annotate(Op::Subject(SubjectOp::ChangePosition(math, 2)));
    let rev = data
        .apply(&annotated)
        .expect("moving a subject inside the list should succeed");

    assert_eq!(
        subject_ids(&data),
        vec![physics, chemistry, math],
        "`new_pos` indexes the list with the moved subject already detached",
    );
    assert_eq!(
        data.get_inner_data()
            .params
            .subjects
            .find_subject_position(math),
        Some(2),
    );

    data.apply(&rev)
        .expect("the reverse of a successful op must apply");

    assert_eq!(subject_ids(&data), vec![math, physics, chemistry]);
    assert!(data == before, "move + undo must restore the prior state");
}

/// Removing a subject's *first* slot: the reverse is the `AddAfter(id, None,
/// slot)` arm, whose `None` anchor means "place first". Undo restores the same
/// slot id at the same position — the identity the history replay rests on.
#[test]
fn remove_first_slot_then_undo_restores_identity() {
    let mut app = AppState::<_, String>::new(Data::new());
    let (subject, teacher, slots) = subject_with_three_slots(&mut app);

    let mut data: Data = app.get_data().clone();
    let before = data.clone();

    let (annotated, _) = data.annotate(Op::Slot(SlotOp::Remove(slots[0])));
    let rev = data
        .apply(&annotated)
        .expect("removing an unreferenced slot should succeed");

    assert_eq!(
        rev,
        AnnotatedOp::Slot(AnnotatedSlotOp::AddAfter(
            slots[0],
            None,
            make_slot_at(subject, teacher, 8),
        )),
        "removing the first slot must reverse through the `None` anchor",
    );
    assert_eq!(slot_ids_of(&data, subject), vec![slots[1], slots[2]]);

    data.apply(&rev)
        .expect("the reverse of a successful op must apply");

    assert_eq!(
        slot_ids_of(&data, subject),
        vec![slots[0], slots[1], slots[2]],
        "the restored slot must keep its original id at its original position",
    );
    assert!(data == before, "remove + undo must restore the prior state");
}
