use std::collections::BTreeSet;

use collomatique_core::*;
use collomatique_state::traits::Manager;
use collomatique_state_colloscopes::{Data, NewId, SubjectParameters};

#[test]
fn can_remove_referenced_periods() {
    let mut app_state = collomatique_state::AppState::new(Data::new());

    // Add a first period of five weeks
    let NewId::PeriodId(_period1) =
        ops::UpdateOp::GeneralPlanning(ops::GeneralPlanningUpdateOp::AddNewPeriod(5))
            .apply(&mut app_state)
            .unwrap()
            .unwrap()
    else {
        panic!("Cannot get a period id")
    };

    // Add a second period of four weeks
    let NewId::PeriodId(period2) =
        ops::UpdateOp::GeneralPlanning(ops::GeneralPlanningUpdateOp::AddNewPeriod(4))
            .apply(&mut app_state)
            .unwrap()
            .unwrap()
    else {
        panic!("Cannot get a period id")
    };

    // Add a subject that runs only on period 1
    let NewId::SubjectId(subject) =
        ops::UpdateOp::Subjects(ops::SubjectsUpdateOp::AddNewSubject(SubjectParameters {
            name: "Math".into(),
            ..Default::default()
        }))
        .apply(&mut app_state)
        .unwrap()
        .unwrap()
    else {
        panic!("Cannot get a subject id")
    };
    if ops::UpdateOp::Subjects(ops::SubjectsUpdateOp::UpdatePeriodStatus(
        subject, period2, false,
    ))
    .apply(&mut app_state)
    .unwrap()
    .is_some()
    {
        panic!("Unexpected id");
    }

    // Remove second period entirely
    if ops::UpdateOp::GeneralPlanning(ops::GeneralPlanningUpdateOp::DeletePeriod(period2))
        .apply(&mut app_state)
        .expect("This should not fail: periods should be delisted automagically")
        .is_some()
    {
        panic!("Unexpected id");
    }

    // Check that indeed the subject does not reference any period
    assert!(app_state
        .get_data()
        .get_subjects()
        .find_subject(subject)
        .unwrap()
        .excluded_periods
        .is_empty());
}

#[test]
fn can_remove_referenced_periods_and_they_are_correctly_restored() {
    let mut app_state = collomatique_state::AppState::new(Data::new());

    // Add a first period of five weeks
    let NewId::PeriodId(_period1) =
        ops::UpdateOp::GeneralPlanning(ops::GeneralPlanningUpdateOp::AddNewPeriod(5))
            .apply(&mut app_state)
            .unwrap()
            .unwrap()
    else {
        panic!("Cannot get a period id")
    };

    // Add a second period of four weeks
    let NewId::PeriodId(period2) =
        ops::UpdateOp::GeneralPlanning(ops::GeneralPlanningUpdateOp::AddNewPeriod(4))
            .apply(&mut app_state)
            .unwrap()
            .unwrap()
    else {
        panic!("Cannot get a period id")
    };

    // Add a subject that runs only on period 1
    let NewId::SubjectId(subject) =
        ops::UpdateOp::Subjects(ops::SubjectsUpdateOp::AddNewSubject(SubjectParameters {
            name: "Math".into(),
            ..Default::default()
        }))
        .apply(&mut app_state)
        .unwrap()
        .unwrap()
    else {
        panic!("Cannot get a subject id")
    };
    if ops::UpdateOp::Subjects(ops::SubjectsUpdateOp::UpdatePeriodStatus(
        subject, period2, false,
    ))
    .apply(&mut app_state)
    .unwrap()
    .is_some()
    {
        panic!("Unexpected id");
    }

    // Remove second period entirely
    if ops::UpdateOp::GeneralPlanning(ops::GeneralPlanningUpdateOp::DeletePeriod(period2))
        .apply(&mut app_state)
        .expect("This should not fail: periods should be delisted automagically")
        .is_some()
    {
        panic!("Unexpected id");
    }

    // Now undo last op
    app_state.undo().unwrap();

    // Check that indeed the subject does reference period2
    let expected = BTreeSet::from([period2]);
    let actual = &app_state
        .get_data()
        .get_subjects()
        .find_subject(subject)
        .unwrap()
        .excluded_periods;
    assert_eq!(*actual, expected);
}

#[test]
fn period_status_is_correctly_reproduced_when_cutting() {
    let mut app_state = collomatique_state::AppState::new(Data::new());

    // Add a first period of five weeks
    let NewId::PeriodId(period1) =
        ops::UpdateOp::GeneralPlanning(ops::GeneralPlanningUpdateOp::AddNewPeriod(5))
            .apply(&mut app_state)
            .unwrap()
            .unwrap()
    else {
        panic!("Cannot get a period id")
    };

    // Add a second period of four weeks
    let NewId::PeriodId(period2) =
        ops::UpdateOp::GeneralPlanning(ops::GeneralPlanningUpdateOp::AddNewPeriod(4))
            .apply(&mut app_state)
            .unwrap()
            .unwrap()
    else {
        panic!("Cannot get a period id")
    };

    // Add a subject that runs only on period 1
    let NewId::SubjectId(subject) =
        ops::UpdateOp::Subjects(ops::SubjectsUpdateOp::AddNewSubject(SubjectParameters {
            name: "Math".into(),
            ..Default::default()
        }))
        .apply(&mut app_state)
        .unwrap()
        .unwrap()
    else {
        panic!("Cannot get a subject id")
    };
    if ops::UpdateOp::Subjects(ops::SubjectsUpdateOp::UpdatePeriodStatus(
        subject, period2, false,
    ))
    .apply(&mut app_state)
    .unwrap()
    .is_some()
    {
        panic!("Unexpected id");
    }

    // Cut first period
    let NewId::PeriodId(_period1_bis) =
        ops::UpdateOp::GeneralPlanning(ops::GeneralPlanningUpdateOp::CutPeriod(period1, 2))
            .apply(&mut app_state)
            .expect("This should not fail")
            .unwrap()
    else {
        panic!("Cannot get a period id")
    };

    // Cut second period
    let NewId::PeriodId(period2_bis) =
        ops::UpdateOp::GeneralPlanning(ops::GeneralPlanningUpdateOp::CutPeriod(period2, 2))
            .apply(&mut app_state)
            .expect("This should not fail")
            .unwrap()
    else {
        panic!("Cannot get a period id")
    };

    // Check that indeed the subject does reference period2 and period2_bis
    let expected = BTreeSet::from([period2, period2_bis]);
    let actual = &app_state
        .get_data()
        .get_subjects()
        .find_subject(subject)
        .unwrap()
        .excluded_periods;
    assert_eq!(*actual, expected);
}
