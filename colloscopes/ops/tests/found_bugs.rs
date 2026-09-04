//! Regression tests for bugs found in the `collomatique-ops` crate.
//!
//! Each test pins one bug deterministically. Following the test-first
//! workflow, every test is committed *before* the corresponding fix and
//! was verified to fail against the unfixed code.

use collomatique_ops::{GeneralPlanningUpdateOp, OpCategory, SubjectsUpdateOp, UpdateOp};
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    ColloscopeOp, Data, Fix, GroupListOp, NewId, NonEmptyRangeInclusive, Op, PeriodOp, SlotOp,
    Subject, SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity,
    TeacherOp, WeekOp,
    group_lists::{GroupList, GroupListFilling, GroupListParameters},
    ids::PeriodId,
    slots::Slot,
    teachers::Teacher,
    weeks::WeekDesc,
};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

type Desc = (OpCategory, String);

fn desc(text: &str) -> Desc {
    (OpCategory::None, text.to_string())
}

/// Creates a front period with `weeks` trivially-active weeks, spliced in one
/// at a time via the `WeekOp` family — periods are created empty.
fn add_active_period(app: &mut AppState<Data, Desc>, weeks: usize) -> PeriodId {
    let period = match app.apply(Op::Period(PeriodOp::AddFront), desc("Add period")) {
        Ok(Some(NewId::PeriodId(id))) => id,
        other => panic!("adding a period should return a period id, got {other:?}"),
    };
    let mut prev = None;
    for _ in 0..weeks {
        let op = match prev {
            None => WeekOp::AddFront(period, WeekDesc::new(true)),
            Some(w) => WeekOp::AddAfter(w, WeekDesc::new(true)),
        };
        match app.apply(Op::Week(op), desc("Add week")) {
            Ok(Some(NewId::WeekId(w))) => prev = Some(w),
            other => panic!("adding a week should return a week id, got {other:?}"),
        }
    }
    period
}

/// Shrinking a period must auto-clean the colloscope interrogations that
/// fall on the weeks being removed. The bug this pins was in the old cleaning
/// machinery: the composite's colloscope-scan loop iterated over
/// `old_week_count..week_count`, an always-empty range when shrinking
/// (`week_count < old_week_count`), so the cleaning op never fired and the
/// shrink hit `NotCompatibleSlotInColloscope` and panicked
/// (`Unexpected error for UpdatePeriodWeekCount!`). The composite now simply
/// removes the doomed weeks and the cascade clears what hangs off them, which
/// is what the fix list below reads back.
#[test]
fn shrinking_a_period_cleans_colloscope_on_removed_weeks() {
    let mut app_state = AppState::<_, Desc>::new(Data::new());

    // A three-week period; we will later shrink it down to two weeks and
    // put a non-empty interrogation on the third (doomed) week.
    let period_id = add_active_period(&mut app_state, 3);

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
                excluded_periods: BTreeSet::new(),
                week_pattern: None,
            },
        )),
        desc("Add subject"),
    ) else {
        panic!("Unexpected result after adding the subject");
    };

    let Ok(Some(NewId::TeacherId(teacher_id))) = app_state.apply(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject_id]),
        })),
        desc("Add teacher"),
    ) else {
        panic!("Unexpected result after adding the teacher");
    };

    let Ok(Some(NewId::SlotId(slot_id))) = app_state.apply(
        Op::Slot(SlotOp::AddAfter(
            None,
            Slot {
                subject_id,
                teacher_id,
                start_time: collomatique_time::SlotStart {
                    weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
                    start_time: collomatique_time::WholeMinuteTime::new(
                        chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
                    )
                    .unwrap(),
                },
                extra_info: String::new(),
                week_pattern: None,
                cost: 0,
            },
        )),
        desc("Add slot"),
    ) else {
        panic!("Unexpected result after adding the slot");
    };

    // A group list associated with the subject so that group number 0 is
    // a valid assignment in an interrogation.
    let Ok(Some(NewId::GroupListId(group_list_id))) = app_state.apply(
        Op::GroupList(GroupListOp::Add(
            GroupList::new(GroupListParameters::default(), GroupListFilling::default()).unwrap(),
        )),
        desc("Add group list"),
    ) else {
        panic!("Unexpected result after adding the group list");
    };
    let Ok(None) = app_state.apply(
        Op::GroupList(GroupListOp::AssignToSubject(
            period_id,
            subject_id,
            Some(group_list_id),
        )),
        desc("Assign group list to subject"),
    ) else {
        panic!("Unexpected result after assigning the group list");
    };

    // A non-empty interrogation on the last (third) week — the one that
    // shrinking to two weeks will remove.
    let week2 = app_state
        .get_data()
        .get_inner_data()
        .params
        .weeks
        .week_id_at(period_id, 2)
        .expect("period has a third week");
    let Ok(None) = app_state.apply(
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot_id,
            week2,
            BTreeSet::from([0]),
        )),
        desc("Put an interrogation on the last week"),
    ) else {
        panic!("Unexpected result after placing the interrogation");
    };

    // Shrink the period from three weeks to two. The interrogation on the
    // removed week must be auto-cleaned, not turned into a hard failure.
    let outcome =
        UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::UpdatePeriodWeekCount(period_id, 2))
            .dry_apply(&app_state)
            .expect(
                "shrinking a period past a non-empty interrogation must auto-clean it, not fail",
            );

    // The cascade fired and reported the colloscope loss.
    let fixes: Vec<Fix> = outcome.warnings.iter().map(|w| w.fix().clone()).collect();
    assert_eq!(
        fixes,
        vec![Fix::ClearInterrogationCell {
            slot: slot_id,
            week: week2,
        }],
        "shrinking a period must clear the interrogation on the removed week, \
         and warn about exactly that",
    );

    // The shrink actually went through: the single period is now two weeks
    // long (its only surviving weeks).
    assert_eq!(
        outcome
            .new_state
            .get_data()
            .get_inner_data()
            .params
            .weeks
            .count_weeks(),
        2,
        "the period should have been shrunk to two weeks",
    );
}

/// Interrogation parameters for a subject that has no slots and no other
/// references — the minimal shape that reaches the ops-level slot cleaning
/// scans without any earlier (teacher / group-list / assignment) cleaning op
/// firing first.
fn lone_interrogation_parameters(name: &str) -> SubjectParameters {
    SubjectParameters {
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
    }
}

/// Removing the interrogation parameters from a subject that has *no slots*
/// must not panic. Since commit b681cdac made the slots ordering sparse, a
/// subject with interrogations but zero slots has no ordering row, so
/// `slots_for_subject` returns `None`. The `UpdateSubject` cleaning scan
/// (`SubjectsUpdateOp::get_next_cleaning_op`) unwrapped that `None` with a
/// stale `.expect("Subject should have associated slots at this point")`,
/// panicking. The import script hits exactly this shape (interrogation
/// subjects created before any slot).
#[test]
fn removing_interrogations_from_zero_slot_subject_does_not_panic() {
    let mut app_state = AppState::<_, Desc>::new(Data::new());

    let period_id = add_active_period(&mut app_state, 2);
    let _ = period_id;

    let Ok(Some(NewId::SubjectId(subject_id))) = app_state.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            Subject {
                parameters: lone_interrogation_parameters("Math"),
                excluded_periods: BTreeSet::new(),
                week_pattern: None,
            },
        )),
        desc("Add subject"),
    ) else {
        panic!("Unexpected result after adding the subject");
    };

    // Drop the interrogation parameters. No slots, no teacher, no group-list
    // association exist, so the cleaning cascade must fall through to the slot
    // scan and find nothing — not panic.
    let mut new_params = lone_interrogation_parameters("Math");
    new_params.interrogation_parameters = None;

    UpdateOp::Subjects(SubjectsUpdateOp::UpdateSubject(subject_id, new_params))
        .dry_apply(&app_state)
        .expect("removing interrogations from a zero-slot subject must succeed, not panic");
}

/// Excluding a zero-slot interrogation subject from a period must not panic.
/// The `UpdatePeriodStatus` cleaning scan guards on
/// `interrogation_parameters.is_some()` and then walked `slots_for_subject`
/// with a stale `.expect("Subject should have slots at this point")` — which
/// panics under the sparse ordering when the subject has no slots.
#[test]
fn excluding_zero_slot_subject_from_period_does_not_panic() {
    let mut app_state = AppState::<_, Desc>::new(Data::new());

    let period_id = add_active_period(&mut app_state, 2);

    let Ok(Some(NewId::SubjectId(subject_id))) = app_state.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            Subject {
                parameters: lone_interrogation_parameters("Math"),
                excluded_periods: BTreeSet::new(),
                week_pattern: None,
            },
        )),
        desc("Add subject"),
    ) else {
        panic!("Unexpected result after adding the subject");
    };

    // Exclude the subject from the period (new_status = false). No students
    // are assigned, so the scan reaches the interrogation-slot walk, which
    // must find no slots — not panic.
    UpdateOp::Subjects(SubjectsUpdateOp::UpdatePeriodStatus(
        subject_id, period_id, false,
    ))
    .dry_apply(&app_state)
    .expect("excluding a zero-slot subject from a period must succeed, not panic");
}
