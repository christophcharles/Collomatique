//! Regression tests pinning the content-preservation contract of the
//! composite general-planning ops after they were re-cut onto the `WeekOp`
//! family (commit 3 of the WeekId split).
//!
//! Cutting a period must carry the tail weeks' colloscope cells *and*
//! week-pattern bits into the new period — this is what lets a later step
//! delete the `save_then_clean_end_of_period` / `restore_end_of_period` dance
//! and rely on `WeekOp::Move` instead. The global week order is unchanged by a
//! cut (the tail simply changes owner), so a week pattern is byte-identical
//! across the cut and the moved colloscope cell reappears in the new period.

use collomatique_ops::{GeneralPlanningUpdateOp, OpCategory, UpdateOp};
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    ColloscopeOp, Data, GroupListOp, NewId, Op, PeriodOp, SlotOp, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp,
    WeekOp, WeekPatternOp, group_lists::GroupListParameters, ids::PeriodId, periods::WeekDesc,
    slots::Slot, teachers::Teacher, week_patterns::WeekPattern,
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

/// Cutting a period preserves the tail's content: a filled colloscope cell and
/// a non-trivial week-pattern bit both survive into the new period.
#[test]
fn cutting_a_period_preserves_tail_colloscope_and_pattern() {
    let mut app_state = AppState::<_, Desc>::new(Data::new());

    // A four-week period; we will cut after two weeks, so weeks 2 and 3 form
    // the tail that must carry its content into the new period.
    let period_id = add_active_period(&mut app_state, 4);
    // The week at the tail (global position 3) the pattern will exclude; its id
    // is preserved across cut/merge, so the exclusion set stays byte-identical.
    let excluded_week = app_state
        .get_data()
        .get_inner_data()
        .params
        .periods
        .week_id_at(period_id, 3)
        .expect("the period has a fourth week");

    let Ok(Some(NewId::SubjectId(subject_id))) = app_state.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            Subject {
                parameters: SubjectParameters {
                    name: "Math".into(),
                    interrogation_parameters: Some(SubjectInterrogationParameters {
                        students_per_group: NonZeroU32::new(2).unwrap()
                            ..=NonZeroU32::new(3).unwrap(),
                        groups_per_interrogation: NonZeroU32::new(1).unwrap()
                            ..=NonZeroU32::new(1).unwrap(),
                        duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
                        take_duration_into_account: true,
                        periodicity: SubjectPeriodicity::ExactlyPeriodic {
                            periodicity_in_weeks: NonZeroU32::new(1).unwrap(),
                        },
                    }),
                },
                excluded_periods: BTreeSet::new(),
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

    let Ok(Some(NewId::GroupListId(group_list_id))) = app_state.apply(
        Op::GroupList(GroupListOp::Add(GroupListParameters::default())),
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

    // A week pattern that excludes the tail week (global position 3). It is not
    // attached to any slot; we only track that `WeekOp::Move` carries the
    // exclusion unchanged across the cut (membership travels with the week id).
    let Ok(Some(NewId::WeekPatternId(week_pattern_id))) = app_state.apply(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "Impaire".into(),
            excluded_weeks: BTreeSet::from([excluded_week]),
        })),
        desc("Add week pattern"),
    ) else {
        panic!("Unexpected result after adding the week pattern");
    };

    // A non-empty interrogation on week 2 (the first tail week).
    let week2 = app_state
        .get_data()
        .get_inner_data()
        .params
        .periods
        .week_id_at(period_id, 2)
        .expect("the period has a third week");
    let Ok(None) = app_state.apply(
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot_id,
            week2,
            BTreeSet::from([0]),
        )),
        desc("Put an interrogation on the third week"),
    ) else {
        panic!("Unexpected result after placing the interrogation");
    };

    let pattern_before = app_state
        .get_data()
        .get_inner_data()
        .params
        .week_patterns
        .week_pattern_map
        .get(&week_pattern_id)
        .expect("week pattern is live")
        .excluded_weeks
        .clone();
    assert_eq!(pattern_before, BTreeSet::from([excluded_week]));

    // Cut the period after two weeks: weeks 2 and 3 move to a fresh period.
    let new_period_id =
        match UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::CutPeriod(period_id, 2))
            .apply(&mut app_state)
            .expect("cutting a period past filled content must preserve it, not fail")
        {
            Some(NewId::PeriodId(id)) => id,
            other => panic!("Unexpected result after cutting the period: {:?}", other),
        };

    // The two periods now hold two weeks each.
    assert_eq!(
        app_state
            .get_data()
            .get_inner_data()
            .params
            .periods
            .week_count_of(period_id),
        Some(2),
        "the original period should keep its first two weeks",
    );
    assert_eq!(
        app_state
            .get_data()
            .get_inner_data()
            .params
            .periods
            .week_count_of(new_period_id),
        Some(2),
        "the new period should hold the two tail weeks",
    );

    // The week-pattern exclusion is unchanged (the week keeps its id across a
    // cut): the excluded week is still excluded.
    assert_eq!(
        app_state
            .get_data()
            .get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .get(&week_pattern_id)
            .expect("week pattern is still live")
            .excluded_weeks,
        BTreeSet::from([excluded_week]),
        "cutting a period must not disturb week-pattern exclusions",
    );

    // The filled cell moved into the new period at local week 0.
    {
        let inner = app_state.get_data().get_inner_data();
        let moved_week = inner
            .params
            .periods
            .week_id_at(new_period_id, 0)
            .expect("the new period has a first week");
        assert_eq!(
            inner
                .colloscope
                .interrogation(&inner.params.periods, slot_id, moved_week),
            Some(&BTreeSet::from([0])),
            "the interrogation content must travel into the new period",
        );
    }

    // Merging the new period back into the original recombines the weeks and
    // still carries the week-pattern bits. (The colloscope content is dropped
    // on merge — pre-existing behavior: merging unassigns the group list, which
    // clears the cells — so we only assert the structural/pattern preservation
    // here.)
    let merge_result = UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::MergeWithPreviousPeriod(
        new_period_id,
    ))
    .apply(&mut app_state)
    .expect("merging the tail period back must succeed");
    assert!(merge_result.is_none());

    assert_eq!(
        app_state
            .get_data()
            .get_inner_data()
            .params
            .periods
            .period_count(),
        1,
        "the two periods should be merged back into one",
    );
    assert_eq!(
        app_state
            .get_data()
            .get_inner_data()
            .params
            .periods
            .week_count_of(period_id),
        Some(4),
        "the merged period should hold all four weeks again",
    );
    assert_eq!(
        app_state
            .get_data()
            .get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .get(&week_pattern_id)
            .expect("week pattern is still live")
            .excluded_weeks,
        BTreeSet::from([excluded_week]),
        "merging must not disturb week-pattern exclusions either",
    );
}
