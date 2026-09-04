//! The family has one op and it touches one field of two entity kinds, so what
//! the fixtures pin is narrow and blunt: the names really all change, the
//! contacts really are gone, no two people come out with the same name, and
//! *nothing else in the document moves* — the last one is what makes the tool
//! safe to hand a colloscope to a stranger.
//!
//! The base is the frozen hogwarts copy (`tests/fixtures/`): 34 people, several
//! of them with a phone number or an email, which an empty document could not
//! say anything about.

use super::*;
use crate::test_utils::hogwarts;
use collomatique_state::AppState;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::{InnerData, PersonWithContact};
use std::collections::BTreeSet;

/// Every person's name in the document, in the order the op walks them.
fn names(data: &Data) -> Vec<PersonWithContact> {
    let inner = data.get_inner_data();
    let students = inner
        .params
        .students
        .student_map
        .values()
        .map(|student| student.desc.clone());
    let teachers = inner
        .params
        .teachers
        .teacher_map
        .values()
        .map(|teacher| teacher.desc.clone());

    students.chain(teachers).collect()
}

/// The document with everybody's `desc` put back to `descs` — the inverse of
/// what the op does, so what comes out should be the base again if the op moved
/// nothing else.
fn with_descs_restored(data: &Data, descs: &[PersonWithContact]) -> InnerData {
    let mut inner = data.get_inner_data().clone();

    let student_ids: Vec<_> = inner.params.students.student_map.keys().collect();
    let teacher_ids: Vec<_> = inner.params.teachers.teacher_map.keys().collect();
    assert_eq!(student_ids.len() + teacher_ids.len(), descs.len());

    let mut descs = descs.iter();
    for id in student_ids {
        let student = inner.params.students.student_map.get_mut(&id).unwrap();
        student.desc = descs.next().unwrap().clone();
    }
    for id in teacher_ids {
        let teacher = inner.params.teachers.teacher_map.get_mut(&id).unwrap();
        teacher.desc = descs.next().unwrap().clone();
    }

    inner
}

/// Applies the op alone on the frozen base and hands back the state it
/// produced, warnings included.
fn anonymize(seed: u64) -> (AppState<Data, Desc>, Vec<CascadeWarning>) {
    let op = AnonymizeUpdateOp::AnonymizeNames { seed };

    let mut session = CascadeSession::new(hogwarts());
    op.apply_to_session(&mut session)
        .expect("the anonymize op cannot be rejected");

    session.commit(op.get_desc())
}

/// The op's whole job, on a realistic document: every student and every teacher
/// comes out under a different name, with no phone number and no email left,
/// and no two of them share a name. The cascade has nothing to say — `desc` is
/// read by no foreign key and no predicate.
#[test]
fn every_name_is_replaced_the_contacts_are_dropped_and_no_two_people_collide() {
    let base = hogwarts();
    let (state, warnings) = anonymize(42);

    assert!(
        warnings.is_empty(),
        "a rename is invisible to the invariants, so it cannot warn: {warnings:?}"
    );

    let before = names(base.get_data());
    let after = names(state.get_data());
    assert_eq!(before.len(), after.len(), "nobody is added or removed");

    for (old, new) in before.iter().zip(after.iter()) {
        assert_ne!(
            (&new.surname, &new.firstname),
            (&old.surname, &old.firstname),
            "{old:?} kept their name",
        );
        assert_eq!(new.tel, None, "{new:?} kept a phone number");
        assert_eq!(new.email, None, "{new:?} kept an email");
    }

    let distinct: BTreeSet<_> = after
        .iter()
        .map(|desc| (desc.surname.clone(), desc.firstname.clone()))
        .collect();
    assert_eq!(
        distinct.len(),
        after.len(),
        "two people came out under the same name",
    );
}

/// Only the names moved. Putting the old `desc` values back on the anonymized
/// document has to give the base document back, field for field — the
/// exclusions, the subjects, the slots, the colloscope, all of it.
#[test]
fn nothing_but_the_names_changes() {
    let base = hogwarts();
    let (state, _warnings) = anonymize(42);

    assert_eq!(
        with_descs_restored(state.get_data(), &names(base.get_data())),
        *base.get_data().get_inner_data(),
    );
}

/// The seed is the op's whole payload: the same one gives the same names back,
/// and a different one gives different ones. That is what lets the op sit in a
/// history and be replayed.
#[test]
fn the_seed_decides_the_names() {
    let (first, _) = anonymize(42);
    let (again, _) = anonymize(42);
    let (other, _) = anonymize(43);

    assert_eq!(names(first.get_data()), names(again.get_data()));
    assert_ne!(names(first.get_data()), names(other.get_data()));
}

/// The whole anonymization is one history slot, named for the button that
/// fires it: a single undo hands the real names back.
#[test]
fn one_undo_restores_every_real_name() {
    let base = hogwarts();
    let (mut state, _warnings) = anonymize(42);

    assert_eq!(
        state.get_undo_name().map(|(_category, text)| text.as_str()),
        Some("Anonymiser les noms"),
    );

    state.undo().expect("one step to undo");
    assert_eq!(state.get_data(), base.get_data());
    assert!(
        !state.can_undo(),
        "the 34 renames should all be in that one step"
    );
}
