//! Colloscope cascade fixtures (step-6 commit 7, plan §9).
//!
//! Each fixture builds a document through the public op surface
//! (`AppState` + `Manager::apply`), then takes `app.get_data().clone()`,
//! annotates the target op itself through `Data::annotate`, and drives
//! [collomatique_state::apply_cascade] on that [Data] directly. What is under
//! test is the resolution map (`src/resolution.rs`) driven by the real engine,
//! not the `AppState` surface.
//!
//! Two rules govern the assertions in this file.
//!
//! **The expected op list is derived on paper first.** Every fixture asserts
//! something about the ops that landed, and that list comes from the plan's
//! §8.1 / §8.2 arm tables *before* the test is ever run. A difference between
//! the hand-derived list and what the engine produced is a finding to explain —
//! possibly a map bug — never a value to paste back in.
//!
//! **Sequence versus content.** Asserting the *order* of the landed ops is only
//! meaningful where the engine actually made a choice, i.e. where one failing
//! apply reported more than one broken invariant and the engine picked
//! `set.first()` out of the `BTreeSet`. Fixture `1a` is the one that pins that
//! canonical pick order; every later fixture asserts content (length plus
//! `contains`) and is deliberately blind to order.

use collomatique_state::{AppState, InMemoryData, apply_cascade, traits::Manager};
use collomatique_state_colloscopes::{
    Data, NewId, Op, PeriodOp, StudentOp, Subject, SubjectOp, SubjectParameters,
    ids::{PeriodId, StudentId, SubjectId},
    ops::{AnnotatedOp, AnnotatedPeriodOp, AnnotatedStudentOp, AnnotatedSubjectOp},
    students::Student,
};
use std::collections::BTreeSet;

/// A subject with no interrogations, excluding exactly `excluded`.
///
/// Interrogations are off on purpose: this file's first fixtures walk
/// `DanglingFk` sites only, and a subject without interrogations cannot host a
/// slot, an association or a balancing entry — so nothing else in the document
/// can reference it by accident.
fn plain_subject(name: &str, excluded: BTreeSet<PeriodId>) -> Subject {
    Subject {
        parameters: SubjectParameters {
            name: name.into(),
            interrogation_parameters: None,
        },
        excluded_periods: excluded,
    }
}

/// A student with default identity, excluding exactly `excluded`.
fn plain_student(excluded: BTreeSet<PeriodId>) -> Student {
    Student {
        desc: Default::default(),
        excluded_periods: excluded,
    }
}

/// The forward op of every landed step, in order.
fn forward_ops(
    applied: &collomatique_state::history::AggregatedOp<AnnotatedOp>,
) -> Vec<AnnotatedOp> {
    applied.inner().iter().map(|r| r.inner().clone()).collect()
}

/// Fixture `1a` — **order**. The minimal document in which the engine genuinely
/// *chooses*.
///
/// A period `P` excluded by one subject and by one student, and referenced by
/// nothing else. `PeriodOp::Remove(P)` therefore fails its first apply with
/// **two** broken invariants at once — `DanglingFk(Period { P,
/// SubjectExcludedPeriods(S) })` and `DanglingFk(Period { P,
/// StudentExcludedPeriods(St) })` — whose fixes are two *different* ops.
///
/// The expected sequence, derived from the tables and not from a run: both
/// breaks are `Reference::Period` on the same target, so the `BTreeSet` order
/// is decided by `PeriodRefSite`'s declaration order, where
/// `SubjectExcludedPeriods` precedes `StudentExcludedPeriods` (`refs.rs`). So
/// round 1 picks the subject site and lands `Subject(Update(S, minus P))`;
/// round 2 retries the target, now sees only the student break, and lands
/// `Student(Update(St, minus P))`; round 3 retries the target and it lands.
///
/// This fixture, and only this one, pins that canonical pick order — it is the
/// tripwire on `FixableInvariant`'s derived `Ord`. A reorder of the enums, or a
/// new variant inserted in the middle, changes the picks and fails here, on two
/// ops, where the diff is readable at a glance.
#[test]
fn fixture_1a_two_simultaneous_breaks_are_fixed_in_canonical_order() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let period: PeriodId = match app.apply(Op::Period(PeriodOp::AddFront), "add period".into()) {
        Ok(Some(NewId::PeriodId(id))) => id,
        other => panic!("adding a period should return a period id, got {other:?}"),
    };
    let excluding_period = BTreeSet::from([period]);

    let subject: SubjectId = match app.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            plain_subject("Math", excluding_period.clone()),
        )),
        "add subject".into(),
    ) {
        Ok(Some(NewId::SubjectId(id))) => id,
        other => panic!("adding a subject should return a subject id, got {other:?}"),
    };
    let student: StudentId = match app.apply(
        Op::Student(StudentOp::Add(plain_student(excluding_period.clone()))),
        "add student".into(),
    ) {
        Ok(Some(NewId::StudentId(id))) => id,
        other => panic!("adding a student should return a student id, got {other:?}"),
    };

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::Period(PeriodOp::Remove(period)));

    let applied = apply_cascade(&mut data, target).expect("the cascade resolves both breaks");

    assert_eq!(
        forward_ops(&applied),
        vec![
            AnnotatedOp::from(AnnotatedSubjectOp::Update(
                subject,
                plain_subject("Math", BTreeSet::new()),
            )),
            AnnotatedOp::from(AnnotatedStudentOp::Update(
                student,
                plain_student(BTreeSet::new()),
            )),
            AnnotatedOp::from(AnnotatedPeriodOp::Remove(period)),
        ],
    );

    let params = &data.get_inner_data().params;
    assert!(
        params.periods.find_period_position(period).is_none(),
        "the target period is gone"
    );
    assert_eq!(
        params.subjects.find_subject(subject),
        Some(&plain_subject("Math", BTreeSet::new())),
        "the subject survives, minus the removed period"
    );
    assert_eq!(
        params.students.student_map.get(&student),
        Some(&plain_student(BTreeSet::new())),
        "the student survives, minus the removed period"
    );
}
