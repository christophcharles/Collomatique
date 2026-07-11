//! Regression tests for the bugs found by the Phase-0 property harness
//! (see the status section of docs/state_consolidation_plan.md).
//!
//! Each test pins one bug deterministically, independent of
//! property-test seed luck. Following the test-first workflow, every
//! test is committed *before* the corresponding fix and was verified to
//! fail against the unfixed code.

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    ColloscopeOp, Data, Error, GroupListError, GroupListOp, NewId, Op, SettingsOp, StudentOp,
    colloscopes::ColloscopeGroupList,
    group_lists::{GroupListFilling, GroupListParameters},
    settings::{Limits, Settings},
    students::Student,
};
use std::collections::{BTreeMap, BTreeSet};

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

/// An automatic→automatic `GroupListOp::SetFilling` must check the new
/// `excluded_students` against students already placed in the colloscope
/// entry of the list, exactly like the prefilled↔automatic transitions
/// and `GroupListOp::Update` do. Before the fix, the op succeeded and
/// left an excluded-but-placed student, panicking the internal
/// invariant check (`ExcludedStudentInGroupList`).
#[test]
fn set_filling_excluding_placed_student_is_rejected() {
    let mut app_state = AppState::<_, String>::new(Data::new());

    let Ok(Some(NewId::StudentId(placed_student))) = app_state.apply(
        Op::Student(StudentOp::Add(Student::default())),
        "Add first student".into(),
    ) else {
        panic!("Unexpected result after adding the first student");
    };
    let Ok(Some(NewId::StudentId(other_student))) = app_state.apply(
        Op::Student(StudentOp::Add(Student::default())),
        "Add second student".into(),
    ) else {
        panic!("Unexpected result after adding the second student");
    };

    // Automatic (default) filling: the list has a colloscope entry
    let Ok(Some(NewId::GroupListId(group_list_id))) = app_state.apply(
        Op::GroupList(GroupListOp::Add(GroupListParameters::default())),
        "Add group list".into(),
    ) else {
        panic!("Unexpected result after adding the group list");
    };

    // Place the first student in group 0 of the colloscope entry
    let Ok(None) = app_state.apply(
        Op::Colloscope(ColloscopeOp::UpdateGroupList(
            group_list_id,
            ColloscopeGroupList {
                groups_for_students: BTreeMap::from([(placed_student, 0)]),
            },
        )),
        "Place student in colloscope".into(),
    ) else {
        panic!("Unexpected result after placing the student");
    };

    // Excluding the placed student must fail
    let result = app_state.apply(
        Op::GroupList(GroupListOp::SetFilling(
            group_list_id,
            GroupListFilling::Automatic {
                excluded_students: BTreeSet::from([placed_student]),
            },
        )),
        "Exclude placed student".into(),
    );
    assert_eq!(
        result,
        Err(Error::GroupList(
            GroupListError::NotCompatibleGroupListInColloscope(group_list_id)
        )),
    );

    // Excluding a student that is not placed still works
    let Ok(None) = app_state.apply(
        Op::GroupList(GroupListOp::SetFilling(
            group_list_id,
            GroupListFilling::Automatic {
                excluded_students: BTreeSet::from([other_student]),
            },
        )),
        "Exclude non-placed student".into(),
    ) else {
        panic!("Excluding a non-placed student should succeed");
    };
}
