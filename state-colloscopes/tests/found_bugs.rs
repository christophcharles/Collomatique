//! Regression tests for the bugs found by the Phase-0 property harness
//! (see the status section of docs/state_consolidation_plan.md).
//!
//! Each test pins one bug deterministically, independent of
//! property-test seed luck. Following the test-first workflow, every
//! test is committed *before* the corresponding fix and was verified to
//! fail against the unfixed code.

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    Data, Error, NewId, Op, SettingsOp, StudentOp,
    settings::{Limits, Settings},
    students::Student,
};
use std::collections::BTreeMap;

/// `StudentOp::Remove` must refuse to remove a student that still has
/// per-student settings, exactly like the existing guards for group
/// lists, colloscope group lists and assignments. Before the fix, the
/// removal succeeded and left a dangling `settings.students` entry,
/// panicking the internal invariant check
/// (`InvalidStudentIdInSettings`).
#[test]
fn remove_student_with_settings_is_rejected() {
    let mut app_state = AppState::<_, String>::new(Data::new());

    let Ok(Some(NewId::StudentId(student_id))) = app_state.apply(
        Op::Student(StudentOp::Add(Student::default())),
        "Add student".into(),
    ) else {
        panic!("Unexpected result after adding a student");
    };

    // Per-student settings entry referencing the student
    let Ok(None) = app_state.apply(
        Op::Settings(SettingsOp::Update(Settings {
            global: Limits::default(),
            students: BTreeMap::from([(student_id, Limits::default())]),
        })),
        "Add per-student settings".into(),
    ) else {
        panic!("Unexpected result after updating settings");
    };

    // Removing the student must fail while the settings entry exists
    let result = app_state.apply(
        Op::Student(StudentOp::Remove(student_id)),
        "Remove student".into(),
    );
    assert!(
        matches!(result, Err(Error::Student(_))),
        "removing a student that still has per-student settings must fail, got {result:?}",
    );

    // Once the settings entry is gone, the removal succeeds
    let Ok(None) = app_state.apply(
        Op::Settings(SettingsOp::Update(Settings::default())),
        "Clear per-student settings".into(),
    ) else {
        panic!("Unexpected result after clearing settings");
    };
    let Ok(None) = app_state.apply(
        Op::Student(StudentOp::Remove(student_id)),
        "Remove student".into(),
    ) else {
        panic!("Removing the student should succeed once its settings entry is gone");
    };
}
