//! The `Assign` op's public error surface, pinned across the address/content
//! split.
//!
//! `AssignmentsUpdateOp::Assign` turns one student's membership into a whole-row
//! `AssignmentOp::SetRow` and translates the state layer's rejection back into
//! [`AssignError`]. That translation used to read a *precheck* error: the state
//! layer swept `SetRow`'s payload students before the op could land, so a dead
//! student arrived as `AssignmentPrecheckError::InvalidStudentId`. The sweep
//! moved to the dangling-FK net (the op's *address* is prechecked, the ids it
//! writes into the document are content), so the same user mistake now arrives
//! as `Error::BrokenInvariants` carrying `DanglingFk @ AssignmentsStudent`.
//!
//! [`AssignError`] itself did not change, and these tests are what says so.
//! They also pin the *order* of the scans in that translation, which is the one
//! thing the tier move could silently alter: a single set can now carry both a
//! dangle and a convergence break, where the precheck used to settle the matter
//! before either was visible.
//!
//! The stakes are higher than a wrong variant. The translation ends in
//! `panic!("Unexpected invariant breaks during Assign")`, so a break shape it
//! fails to recognize is a production crash, not a mislabelled message.

use collomatique_ops::{
    AssignError, AssignmentsUpdateError, AssignmentsUpdateOp, OpCategory, UpdateError, UpdateOp,
};
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::ids::{PeriodId, StudentId, SubjectId};
use collomatique_state_colloscopes::students::Student;
use collomatique_state_colloscopes::{
    Data, NewId, Op, PeriodOp, StudentOp, Subject, SubjectOp, SubjectParameters,
};
use std::collections::BTreeSet;

type Desc = (OpCategory, String);

fn desc(text: &str) -> Desc {
    (OpCategory::None, text.to_string())
}

/// A subject with no interrogations — the cheapest thing an assignments row can
/// be keyed on. Nothing in the assignments half of the checker reads
/// interrogation parameters.
fn plain_subject(name: &str, excluded: BTreeSet<PeriodId>) -> Subject {
    Subject {
        parameters: SubjectParameters {
            name: name.into(),
            interrogation_parameters: None,
        },
        excluded_periods: excluded,
        week_pattern: None,
    }
}

fn add_period(app: &mut AppState<Data, Desc>) -> PeriodId {
    match app.apply(Op::Period(PeriodOp::AddFront), desc("Add period")) {
        Ok(Some(NewId::PeriodId(id))) => id,
        other => panic!("adding a period should return a period id, got {other:?}"),
    }
}

fn add_subject(app: &mut AppState<Data, Desc>, subject: Subject) -> SubjectId {
    match app.apply(
        Op::Subject(SubjectOp::AddAfter(None, subject)),
        desc("Add subject"),
    ) {
        Ok(Some(NewId::SubjectId(id))) => id,
        other => panic!("adding a subject should return a subject id, got {other:?}"),
    }
}

fn add_student(app: &mut AppState<Data, Desc>) -> StudentId {
    match app.apply(
        Op::Student(StudentOp::Add(Student::default())),
        desc("Add student"),
    ) {
        Ok(Some(NewId::StudentId(id))) => id,
        other => panic!("adding a student should return a student id, got {other:?}"),
    }
}

/// A `StudentId` that is not live: create one nothing references, then remove
/// it. An integration test has no other route — the id types are opaque and
/// carry no public constructor.
fn dead_student_id(app: &mut AppState<Data, Desc>) -> StudentId {
    let id = add_student(app);
    app.apply(Op::Student(StudentOp::Remove(id)), desc("Remove student"))
        .expect("removing an unreferenced student succeeds");
    id
}

/// Runs `Assign` and returns the [`AssignError`] it produced, failing the test
/// on success or on any other error shape.
fn assign_err(
    app: &mut AppState<Data, Desc>,
    period: PeriodId,
    student: StudentId,
    subject: SubjectId,
) -> AssignError {
    let op = UpdateOp::Assignments(AssignmentsUpdateOp::Assign(period, student, subject, true));
    match op.apply(app) {
        Err(UpdateError::Assignments(AssignmentsUpdateError::Assign(e))) => e,
        other => panic!("expected an Assign error, got {other:?}"),
    }
}

#[test]
fn assigning_a_student_who_does_not_exist_reports_invalid_student_id() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let period = add_period(&mut app);
    let subject = add_subject(&mut app, plain_subject("Math", BTreeSet::new()));
    let dead = dead_student_id(&mut app);

    // The state layer no longer prechecks this: the `SetRow` lands, the checker
    // reports `DanglingFk @ AssignmentsStudent`, and the adapter translates that
    // back into the error this surface has always produced.
    assert_eq!(
        assign_err(&mut app, period, dead, subject),
        AssignError::InvalidStudentId(dead),
    );
}

#[test]
fn assigning_to_a_subject_that_does_not_run_reports_the_subject() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let period = add_period(&mut app);
    let subject = add_subject(&mut app, plain_subject("Math", BTreeSet::from([period])));
    let student = add_student(&mut app);

    // The convergence route, untouched by the tier move — here to show the
    // dangle scan added in front of it did not shadow it.
    assert_eq!(
        assign_err(&mut app, period, student, subject),
        AssignError::SubjectDoesNotRunOnPeriod(subject, period),
    );
}

#[test]
fn a_dead_student_on_a_subject_that_does_not_run_still_reports_the_student() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let period = add_period(&mut app);
    let subject = add_subject(&mut app, plain_subject("Math", BTreeSet::from([period])));
    let dead = dead_student_id(&mut app);

    // Both breaks land in one set — this is the case the old precheck settled
    // before either was visible, and the reason the dangle scan runs first.
    // Reversing the two scans would turn this into
    // `SubjectDoesNotRunOnPeriod`, a different message for the same user
    // mistake.
    assert_eq!(
        assign_err(&mut app, period, dead, subject),
        AssignError::InvalidStudentId(dead),
    );
}
