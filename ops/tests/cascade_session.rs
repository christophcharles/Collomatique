//! [CascadeSession] on a real document.
//!
//! The session struct is what every user-facing op will be applied through, so
//! these tests drive it directly with **elementary** ops — no [UpdateOp] exists
//! on the new path yet. The base document is the frozen hogwarts copy
//! (`tests/fixtures/hogwarts.collomatique`, deliberately decoupled from the
//! living `examples/` file so the example can evolve without touching the
//! tests): a real teacher, with real slots, tied together by a real slot
//! pairing rule — a two-level cascade nobody had to build by hand.
//!
//! Ids are looked up by name rather than written as literals: the fixture is
//! frozen, but a test that says « Bibine » says what it means.

use collomatique_ops::{CascadeSession, CascadeWarning, Desc, OpCategory};
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    Data, Error, Fix, NewId, Op, PersonWithContact, SlotOp, SlotPairingOp, TeacherOp,
    ids::{Id, SlotId, SlotPairingRuleId, SubjectId, TeacherId},
    teachers::Teacher,
};
use collomatique_storage::deserialize_data;
use std::collections::BTreeSet;

const HOGWARTS: &str = include_str!("fixtures/hogwarts.collomatique");

fn desc(text: &str) -> Desc {
    (OpCategory::Teachers, text.to_string())
}

/// The frozen base document, decoded and wrapped in a blank-history state.
fn hogwarts() -> AppState<Data, Desc> {
    let (inner_data, caveats) =
        deserialize_data(HOGWARTS).expect("the frozen fixture should decode");
    assert!(
        caveats.is_empty(),
        "the frozen fixture should decode cleanly, got {caveats:?}"
    );
    let data = Data::from_inner_data(inner_data)
        .expect("the frozen fixture should pass the invariant gate");

    AppState::new(data)
}

fn teacher_by_surname(data: &Data, surname: &str) -> TeacherId {
    data.get_inner_data()
        .params
        .teachers
        .teacher_map
        .iter()
        .find(|(_id, teacher)| teacher.desc.surname == surname)
        .map(|(id, _teacher)| id)
        .unwrap_or_else(|| panic!("the fixture should have a teacher named {surname}"))
}

/// The teacher's slots, in the document's own order — which is the order the
/// cascade meets them.
fn slots_of(data: &Data, teacher: TeacherId) -> Vec<SlotId> {
    let mut slots: Vec<_> = data
        .get_inner_data()
        .params
        .slots
        .all_slots()
        .filter(|(_id, slot)| slot.teacher_id == teacher)
        .map(|(id, _slot)| *id)
        .collect();
    slots.sort();

    slots
}

/// The slot pairing rules naming any of `slots`.
fn slot_pairing_rules_over(data: &Data, slots: &[SlotId]) -> Vec<SlotPairingRuleId> {
    data.get_inner_data()
        .params
        .slot_pairings
        .slot_pairing_rule_map
        .iter()
        .filter(|(_id, rule)| {
            slots.contains(&rule.antecedent().slot_id) || slots.contains(&rule.consequent().slot_id)
        })
        .map(|(id, _rule)| id)
        .collect()
}

fn teacher(surname: &str, firstname: &str, subjects: BTreeSet<SubjectId>) -> Teacher {
    Teacher {
        desc: PersonWithContact {
            surname: surname.to_string(),
            firstname: firstname.to_string(),
            tel: None,
            email: None,
        },
        subjects,
    }
}

fn fixes(warnings: &[CascadeWarning]) -> Vec<Fix> {
    warnings.iter().map(|w| w.fix().clone()).collect()
}

/// Removing Mme Bibine cannot land alone: her two slots reference her, and one
/// slot pairing rule references *both* her slots. The engine unwinds
/// depth-first, so the rule — the deepest repair — lands first, then the slot
/// that dragged it in, then the second slot, and finally the teacher.
///
/// Both halves are pinned: the warning log (the fixes, in application order)
/// and the document itself, rebuilt by applying exactly those elementary ops
/// to the base in exactly that order.
#[test]
fn deleting_a_teacher_logs_the_slots_the_cascade_had_to_remove() {
    let base = hogwarts();
    let bibine = teacher_by_surname(base.get_data(), "Bibine");
    let slots = slots_of(base.get_data(), bibine);
    let rules = slot_pairing_rules_over(base.get_data(), &slots);
    assert_eq!(slots.len(), 2, "the fixture's Bibine should have two slots");
    assert_eq!(
        rules.len(),
        1,
        "the fixture should pair Bibine's two slots by exactly one rule"
    );

    let mut session = CascadeSession::new(base.clone());
    let new_id = session
        .apply(Op::Teacher(TeacherOp::Remove(bibine)), desc("Supprimer"))
        .expect("the cascade clears the way for the removal");
    let (state, warnings) = session.commit(desc("Supprimer un colleur"));

    assert_eq!(new_id, None, "a removal creates nothing");
    assert_eq!(
        fixes(&warnings),
        vec![
            Fix::DeleteSlotPairingRule { rule: rules[0] },
            Fix::DeleteSlot { slot: slots[0] },
            Fix::DeleteSlot { slot: slots[1] },
        ],
    );

    // The document lost exactly that, and nothing else: rebuild it by applying
    // the fixes and then the target to the base, each valid in that order.
    let mut expected = base.clone();
    for op in [
        Op::SlotPairing(SlotPairingOp::Remove(rules[0])),
        Op::Slot(SlotOp::Remove(slots[0])),
        Op::Slot(SlotOp::Remove(slots[1])),
        Op::Teacher(TeacherOp::Remove(bibine)),
    ] {
        expected
            .apply(op, desc("Expected"))
            .expect("each expected op lands in cascade order");
    }
    assert_eq!(state.get_data(), expected.get_data());
}

/// The whole session — several ops, and every repair they cascaded — is one
/// history slot on the manager that comes back: a single undo takes the
/// document back to where the composite found it, and there is nothing left to
/// undo after it.
#[test]
fn commit_collapses_the_whole_session_into_one_undo_step() {
    let base = hogwarts();
    let bibine = teacher_by_surname(base.get_data(), "Bibine");

    let mut session = CascadeSession::new(base.clone());
    session
        .apply(Op::Teacher(TeacherOp::Remove(bibine)), desc("Supprimer"))
        .expect("the cascade clears the way");
    session
        .apply(
            Op::Teacher(TeacherOp::Add(teacher("Rusard", "Argus", BTreeSet::new()))),
            desc("Ajouter"),
        )
        .expect("an unattached teacher breaks nothing");
    let (mut state, _warnings) = session.commit(desc("Remplacer un colleur"));

    assert_eq!(
        state.get_undo_name().map(|(_category, text)| text.as_str()),
        Some("Remplacer un colleur"),
    );

    state.undo().expect("one step to undo");
    assert_eq!(state.get_data(), base.get_data());
    assert!(
        !state.can_undo(),
        "the session's ops and their fixes should all be in that one step"
    );
}

/// Cancelling unwinds everything the session did and hands the manager back as
/// it was — history included. The warnings go with it: they describe repairs
/// that no longer happened.
#[test]
fn cancel_returns_the_manager_untouched() {
    let base = hogwarts();
    let bibine = teacher_by_surname(base.get_data(), "Bibine");

    let mut session = CascadeSession::new(base.clone());
    session
        .apply(Op::Teacher(TeacherOp::Remove(bibine)), desc("Supprimer"))
        .expect("the cascade clears the way");
    assert_ne!(
        session.get_data(),
        base.get_data(),
        "the session should see its own modification"
    );

    let state = session.cancel();

    assert_eq!(state.get_data(), base.get_data());
    assert!(!state.can_undo());
}

/// An op that creates an entity hands its id back *inline*, so the next op of
/// the same session can use it. That is the whole reason the session applies
/// one op at a time instead of planning a list ahead.
#[test]
fn an_add_hands_its_id_back_for_the_next_op_to_use() {
    let base = hogwarts();

    let mut session = CascadeSession::new(base.clone());
    let new_id = session
        .apply(
            Op::Teacher(TeacherOp::Add(teacher("Rusard", "Argus", BTreeSet::new()))),
            desc("Ajouter"),
        )
        .expect("an unattached teacher breaks nothing");
    let Some(NewId::TeacherId(rusard)) = new_id else {
        panic!("adding a teacher should return a teacher id, got {new_id:?}");
    };

    session
        .apply(
            Op::Teacher(TeacherOp::Update(
                rusard,
                teacher("Rusard", "Argus Severus", BTreeSet::new()),
            )),
            desc("Modifier"),
        )
        .expect("the id the session just issued names a live teacher");
    let (state, warnings) = session.commit(desc("Ajouter un colleur"));

    assert_eq!(fixes(&warnings), vec![]);
    assert_eq!(
        state
            .get_data()
            .get_inner_data()
            .params
            .teachers
            .teacher_map
            .get(&rusard)
            .expect("the new teacher is in the committed document")
            .desc
            .firstname,
        "Argus Severus",
    );
}

/// A break caused by the failing op's own payload is not repairable: no state
/// the map could reach holds the offending material — it was rolled back with
/// the op. The engine convicts the target, and the session is left exactly as
/// the previous op left it, warning log included.
#[test]
fn a_convicted_op_errs_and_logs_nothing() {
    let base = hogwarts();
    let bibine = teacher_by_surname(base.get_data(), "Bibine");
    let slots = slots_of(base.get_data(), bibine);
    let rules = slot_pairing_rules_over(base.get_data(), &slots);
    let dangling_subject = unsafe { SubjectId::new(1u64 << 40) };

    let mut session = CascadeSession::new(base.clone());
    session
        .apply(Op::Teacher(TeacherOp::Remove(bibine)), desc("Supprimer"))
        .expect("the cascade clears the way");
    let data_before = session.get_data().clone();

    let err = session
        .apply(
            Op::Teacher(TeacherOp::Add(teacher(
                "Rusard",
                "Argus",
                BTreeSet::from([dangling_subject]),
            ))),
            desc("Ajouter"),
        )
        .expect_err("a teacher teaching a subject that does not exist cannot land");

    assert!(
        matches!(err, Error::BrokenInvariants(_)),
        "the payload's dangling subject is a broken invariant, got {err:?}"
    );
    assert_eq!(session.get_data(), &data_before);

    // Still exactly what the teacher removal logged, nothing appended.
    let (_state, warnings) = session.commit(desc("Supprimer un colleur"));
    assert_eq!(
        fixes(&warnings),
        vec![
            Fix::DeleteSlotPairingRule { rule: rules[0] },
            Fix::DeleteSlot { slot: slots[0] },
            Fix::DeleteSlot { slot: slots[1] },
        ],
    );
}
