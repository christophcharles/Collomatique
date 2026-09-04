use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    Data, Error, FixableInvariant, NewId, NonEmptyRangeInclusive, Op, PeriodOp, PeriodRefSite,
    Reference, Subject, SubjectOp, SubjectParameters, SubjectPeriodicity, WeekOp,
    ids::{PeriodId, WeekId},
    subjects::{SubjectInterrogationParameters, WeekBlock},
    weeks::WeekDesc,
};
use std::{collections::BTreeSet, num::NonZeroU32};

/// Creates a front period carrying `weeks` trivially-active weeks (spliced in
/// one at a time — periods are created empty), returning the period id and its
/// week ids in order.
fn add_active_period(app: &mut AppState<Data, String>, weeks: usize) -> (PeriodId, Vec<WeekId>) {
    let period = match app.apply(Op::Period(PeriodOp::AddFront), "Add period".into()) {
        Ok(Some(NewId::PeriodId(id))) => id,
        other => panic!("adding a period should return a period id, got {other:?}"),
    };
    let mut week_ids = Vec::new();
    for _ in 0..weeks {
        let op = match week_ids.last() {
            None => WeekOp::AddFront(period, WeekDesc::new(true)),
            Some(&w) => WeekOp::AddAfter(w, WeekDesc::new(true)),
        };
        match app.apply(Op::Week(op), "Add week".into()) {
            Ok(Some(NewId::WeekId(w))) => week_ids.push(w),
            other => panic!("adding a week should return a week id, got {other:?}"),
        }
    }
    (period, week_ids)
}

#[test]
fn add_subject_referencing_period_then_remove_period() {
    let mut app_state = AppState::<_, String>::new(Data::new());

    // Prepare periods. The second period is left week-empty so the only thing
    // that can block its removal is the subject reference under test.
    let (id1, _) = add_active_period(&mut app_state, 3);
    let Ok(Some(NewId::PeriodId(id2))) = app_state.apply(
        Op::Period(PeriodOp::AddAfter(id1)),
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
                excluded_periods: BTreeSet::from([id2]),
                week_pattern: None,
            },
        )),
        "Add subject".into(),
    ) else {
        panic!("Unexpected result after adding the subject");
    };

    // Remove second period
    let result = app_state.apply(
        Op::Period(PeriodOp::Remove(id2)),
        "Remove unused period".into(),
    );

    assert_eq!(
        result,
        Err(Error::BrokenInvariants(BTreeSet::from([
            FixableInvariant::DanglingFk(Reference::Period {
                target: id2,
                site: PeriodRefSite::SubjectExcludedPeriods(subject_id),
            })
        ]))),
    );
}

#[test]
fn add_subject_referencing_period_then_remove_period_and_then_undo() {
    let mut app_state = AppState::<_, String>::new(Data::new());

    // Prepare periods. The second period is left week-empty so that once the
    // subject reference is removed, nothing else blocks its removal.
    let (id1, _) = add_active_period(&mut app_state, 3);
    let Ok(Some(NewId::PeriodId(id2))) = app_state.apply(
        Op::Period(PeriodOp::AddAfter(id1)),
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
                excluded_periods: BTreeSet::from([id2]),
                week_pattern: None,
            },
        )),
        "Add subject".into(),
    ) else {
        panic!("Unexpected result after adding the subject");
    };

    // Remove reference to second period
    let Ok(None) = app_state.apply(
        Op::Subject(SubjectOp::Update(
            subject_id,
            Subject {
                parameters: SubjectParameters {
                    name: "Math".into(),
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
                week_pattern: None,
            },
        )),
        "Update subject".into(),
    ) else {
        panic!("Unexpected result after updating the subject");
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
    app_state.undo().unwrap();

    // Checks that the subject has the correct excluded periods
    let expected = BTreeSet::from([id2]);
    assert_eq!(
        app_state
            .get_data()
            .get_inner_data()
            .params
            .subjects
            .find_subject(subject_id)
            .unwrap()
            .excluded_periods,
        expected
    );
}

#[test]
fn add_subject_referencing_week_then_shrink_week_count_but_keep_said_week() {
    let mut app_state = AppState::<_, String>::new(Data::new());

    // Prepare a five-week period.
    let (_period_id, week_ids) = add_active_period(&mut app_state, 5);

    // Add subject
    let Ok(Some(NewId::SubjectId(_subject_id))) = app_state.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            Subject {
                parameters: SubjectParameters {
                    name: "Math".into(),
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
                        periodicity: SubjectPeriodicity::AmountForEveryArbitraryBlock {
                            minimum_week_separation: 1,
                            blocks: vec![
                                WeekBlock {
                                    delay_in_weeks: 0,
                                    size_in_weeks: NonZeroU32::new(3).unwrap(),
                                    interrogation_count_in_block: NonEmptyRangeInclusive::new(
                                        1..=1,
                                    )
                                    .expect("statically non-empty"),
                                },
                                WeekBlock {
                                    delay_in_weeks: 0,
                                    size_in_weeks: NonZeroU32::new(2).unwrap(),
                                    interrogation_count_in_block: NonEmptyRangeInclusive::new(
                                        1..=1,
                                    )
                                    .expect("statically non-empty"),
                                },
                            ],
                        },
                    }),
                },
                excluded_periods: BTreeSet::new(),
                week_pattern: None,
            },
        )),
        "Add subject".into(),
    ) else {
        panic!("Unexpected result after adding the subject");
    };

    // Shrink the period by dropping its last week while a subject's blocks
    // still reference the remaining weeks — this must be allowed.
    let Ok(None) = app_state.apply(
        Op::Week(WeekOp::Remove(week_ids[4])),
        "Shrink period".into(),
    ) else {
        panic!("Unexpected result after removing the last week");
    };
}
