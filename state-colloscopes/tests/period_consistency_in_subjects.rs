use collomatique_state::{traits::Manager, AppState};
use collomatique_state_colloscopes::{
    Data, NewId, Op, PeriodOp, Subject, SubjectOp, SubjectParameters, SubjectPeriodicity,
};
use std::{collections::BTreeSet, num::NonZeroU32};

#[test]
fn add_subject_referencing_period_then_remove_period() {
    let mut app_state = AppState::new(Data::new());

    // Prepare periods
    let Ok(Some(NewId::PeriodId(id1))) = app_state.apply(
        Op::Period(PeriodOp::AddFront(vec![true, true, false])),
        "Add first period".into(),
    ) else {
        panic!("Unexpected result after adding first period");
    };
    let Ok(Some(NewId::PeriodId(id2))) = app_state.apply(
        Op::Period(PeriodOp::AddAfter(id1, vec![false, true])),
        "Add second period".into(),
    ) else {
        panic!("Unexpected result after adding second period");
    };

    // Add subject
    let Ok(Some(NewId::SubjectId(subject_id))) = app_state.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            Subject {
                parameters: SubjectParameters {
                    name: "Math".into(),
                    students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                    groups_per_interrogation: NonZeroU32::new(1).unwrap()
                        ..=NonZeroU32::new(1).unwrap(),
                    duration: collomatique_time::NonZeroDurationInMinutes::new(60).unwrap(),
                    take_duration_into_account: true,
                    periodicity: SubjectPeriodicity::ExactlyPeriodic {
                        periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
                    },
                },
                excluded_periods: BTreeSet::from([id2]),
            },
        )),
        "Add subject".into(),
    ) else {
        panic!("Unexpected result after adding the subject");
    };

    // Remove second period
    let Ok(None) = app_state.apply(
        Op::Period(PeriodOp::Remove(id2)),
        "Remove unused period".into(),
    ) else {
        panic!("Unexpected result after removing unused period");
    };

    // Checks that the subject has no excluded periods
    assert!(app_state
        .get_data()
        .get_subjects()
        .find_subject(subject_id)
        .unwrap()
        .excluded_periods
        .is_empty());
}

#[test]
fn add_subject_referencing_period_then_remove_period_and_then_undo() {
    let mut app_state = AppState::new(Data::new());

    // Prepare periods
    let Ok(Some(NewId::PeriodId(id1))) = app_state.apply(
        Op::Period(PeriodOp::AddFront(vec![true, true, false])),
        "Add first period".into(),
    ) else {
        panic!("Unexpected result after adding first period");
    };
    let Ok(Some(NewId::PeriodId(id2))) = app_state.apply(
        Op::Period(PeriodOp::AddAfter(id1, vec![false, true])),
        "Add second period".into(),
    ) else {
        panic!("Unexpected result after adding second period");
    };

    // Add subject
    let Ok(Some(NewId::SubjectId(subject_id))) = app_state.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            Subject {
                parameters: SubjectParameters {
                    name: "Math".into(),
                    students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                    groups_per_interrogation: NonZeroU32::new(1).unwrap()
                        ..=NonZeroU32::new(1).unwrap(),
                    duration: collomatique_time::NonZeroDurationInMinutes::new(60).unwrap(),
                    take_duration_into_account: true,
                    periodicity: SubjectPeriodicity::ExactlyPeriodic {
                        periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
                    },
                },
                excluded_periods: BTreeSet::from([id2]),
            },
        )),
        "Add subject".into(),
    ) else {
        panic!("Unexpected result after adding the subject");
    };

    // Remove second period
    let Ok(None) = app_state.apply(
        Op::Period(PeriodOp::Remove(id2)),
        "Remove unused period".into(),
    ) else {
        panic!("Unexpected result after removing unused period");
    };

    // Undo the op
    app_state.undo().unwrap();

    // Checks that the subject has the correct excluded periods
    let expected = BTreeSet::from([id2]);
    assert_eq!(
        app_state
            .get_data()
            .get_subjects()
            .find_subject(subject_id)
            .unwrap()
            .excluded_periods,
        expected
    );
}
