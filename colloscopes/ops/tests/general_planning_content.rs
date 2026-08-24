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
//!
//! The contract held across step 7's change of machinery: the same document was
//! cut through the old cleaning path and through the cascade, against the same
//! assertions, until the old path was deleted. Where the two parted company was
//! the *merge* that follows, and deliberately: the old path unassigned the group
//! list before moving the weeks and lost the colles with it, where the cascade
//! moves the weeks first and keeps them (the step's first divergence, which is
//! what the merge fixture below now pins).

use collomatique_ops::{CascadeWarning, GeneralPlanningUpdateOp, OpCategory, UpdateOp};
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    ColloscopeOp, Data, Fix, GroupListOp, NewId, NonEmptyRangeInclusive, Op, PeriodOp, SlotOp,
    Subject, SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity,
    TeacherOp, WeekOp, WeekPatternOp,
    group_lists::{GroupList, GroupListFilling, GroupListParameters},
    ids::{GroupListId, PeriodId, SlotId, SubjectId, WeekId, WeekPatternId},
    slots::Slot,
    teachers::Teacher,
    week_patterns::WeekPattern,
    weeks::WeekDesc,
};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

type Desc = (OpCategory, String);

fn desc(text: &str) -> Desc {
    (OpCategory::None, text.to_string())
}

/// The repairs a cascade logged, read back as the [Fix] values the fixtures
/// write down.
fn fixes(warnings: &[CascadeWarning]) -> Vec<Fix> {
    warnings.iter().map(|w| w.fix().clone()).collect()
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

/// The material both cut fixtures read back.
struct Document {
    period_id: PeriodId,
    subject_id: SubjectId,
    slot_id: SlotId,
    group_list_id: GroupListId,
    week_pattern_id: WeekPatternId,
    /// The week at global position 3 — in the tail, and the one the pattern
    /// excludes.
    excluded_week: WeekId,
    /// The week at global position 2 — the first tail week, and the one
    /// carrying the interrogation.
    filled_week: WeekId,
}

/// A four-week period holding one filled colloscope cell (on the third week)
/// and one week-pattern exclusion (on the fourth): cut after two weeks, both
/// sit in the tail that must carry its content into the new period.
fn build_document() -> (AppState<Data, Desc>, Document) {
    let mut app_state = AppState::<_, Desc>::new(Data::new());

    let period_id = add_active_period(&mut app_state, 4);
    // The week at the tail (global position 3) the pattern will exclude; its id
    // is preserved across cut/merge, so the exclusion set stays byte-identical.
    let excluded_week = app_state
        .get_data()
        .get_inner_data()
        .params
        .weeks
        .week_id_at(period_id, 3)
        .expect("the period has a fourth week");

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
    let filled_week = app_state
        .get_data()
        .get_inner_data()
        .params
        .weeks
        .week_id_at(period_id, 2)
        .expect("the period has a third week");
    let Ok(None) = app_state.apply(
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot_id,
            filled_week,
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

    (
        app_state,
        Document {
            period_id,
            subject_id,
            slot_id,
            group_list_id,
            week_pattern_id,
            excluded_week,
            filled_week,
        },
    )
}

/// What a cut after two weeks must have done, whichever path applied it: the
/// weeks are split evenly, the pattern is untouched, and the filled cell is now
/// the new period's first week.
fn assert_cut_preserved_content(
    app_state: &AppState<Data, Desc>,
    document: &Document,
    new_period_id: PeriodId,
) {
    // The two periods now hold two weeks each.
    assert_eq!(
        app_state
            .get_data()
            .get_inner_data()
            .params
            .weeks
            .week_count_for_period(document.period_id)
            .unwrap_or(0),
        2,
        "the original period should keep its first two weeks",
    );
    assert_eq!(
        app_state
            .get_data()
            .get_inner_data()
            .params
            .weeks
            .week_count_for_period(new_period_id)
            .unwrap_or(0),
        2,
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
            .get(&document.week_pattern_id)
            .expect("week pattern is still live")
            .excluded_weeks,
        BTreeSet::from([document.excluded_week]),
        "cutting a period must not disturb week-pattern exclusions",
    );

    // The filled cell moved into the new period at local week 0.
    {
        let inner = app_state.get_data().get_inner_data();
        let moved_week = inner
            .params
            .weeks
            .week_id_at(new_period_id, 0)
            .expect("the new period has a first week");
        assert_eq!(
            moved_week, document.filled_week,
            "the tail's first week should be the one carrying the interrogation",
        );
        assert_eq!(
            inner.colloscope.interrogation(document.slot_id, moved_week),
            Some(&BTreeSet::from([0])),
            "the interrogation content must travel into the new period",
        );
    }
}

/// What a merge of the tail back into the original must have done: one period
/// again, holding all four weeks, with the pattern untouched. What becomes of
/// the colles is asserted by the caller.
fn assert_merged_structure(app_state: &AppState<Data, Desc>, document: &Document) {
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
            .weeks
            .week_count_for_period(document.period_id)
            .unwrap_or(0),
        4,
        "the merged period should hold all four weeks again",
    );
    assert_eq!(
        app_state
            .get_data()
            .get_inner_data()
            .params
            .week_patterns
            .week_pattern_map
            .get(&document.week_pattern_id)
            .expect("week pattern is still live")
            .excluded_weeks,
        BTreeSet::from([document.excluded_week]),
        "merging must not disturb week-pattern exclusions either",
    );
}

/// Cutting a period preserves the tail's content: a filled colloscope cell and
/// a non-trivial week-pattern bit both survive into the new period, and the
/// cascade repairs nothing on the way — a cut that had to warn about a colle
/// would be a cut that lost one.
///
/// The merge that follows is where the cascade parts company with the old
/// cleaning path: the weeks move first and take their colles with them, so the
/// cell survives here where the old path dropped it. And it says nothing on the
/// way: the only thing the emptied period was keyed on is the group-list
/// association the cut had copied from the very period it merges back into, so
/// dropping it changes nothing. Cut then merge is a silent round trip.
#[test]
fn cutting_a_period_preserves_tail_colloscope_and_pattern() {
    let (mut app_state, document) = build_document();

    let cut = UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::CutPeriod(document.period_id, 2))
        .dry_apply(&app_state)
        .expect("cutting a period past filled content must preserve it, not fail");
    let new_period_id = match cut.new_id {
        Some(NewId::PeriodId(id)) => id,
        other => panic!("Unexpected result after cutting the period: {:?}", other),
    };
    assert_eq!(
        fixes(&cut.warnings),
        Vec::new(),
        "a cut carries its content over untouched: there is nothing to repair",
    );
    app_state = cut.new_state;

    assert_cut_preserved_content(&app_state, &document, new_period_id);

    let merge = UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::MergeWithPreviousPeriod(
        new_period_id,
    ))
    .dry_apply(&app_state)
    .expect("merging the tail period back must succeed");
    assert!(merge.new_id.is_none());
    assert_eq!(
        fixes(&merge.warnings),
        Vec::new(),
        "a cut followed by a merge is a round trip: the tail period's association \
         is the copy the cut made of the head's, so dropping it changes nothing \
         and the merge says nothing",
    );
    app_state = merge.new_state;

    assert_merged_structure(&app_state, &document);

    // The divergence: the colles came back with their weeks.
    let inner = app_state.get_data().get_inner_data();
    assert_eq!(
        inner
            .params
            .weeks
            .week_id_at(document.period_id, 2)
            .expect("the merged period has a third week"),
        document.filled_week,
        "the merged weeks should be appended in order, so the filled one is third again",
    );
    assert_eq!(
        inner
            .colloscope
            .interrogation(document.slot_id, document.filled_week),
        Some(&BTreeSet::from([0])),
        "merging must carry the colles of the moved weeks, not erase them",
    );
    assert_eq!(
        inner
            .params
            .group_lists
            .subjects_associations
            .get(&(document.period_id, document.subject_id)),
        Some(&document.group_list_id),
        "the surviving period keeps the association its colles are read against",
    );
}

/// The same cut once more through [UpdateOp::apply], the variant that installs
/// the new state itself and drops the warnings — the one the scripting api
/// calls. What it must still hand back is the created id.
#[test]
fn apply_installs_the_cut_in_place() {
    let (mut app_state, document) = build_document();

    let new_period_id =
        match UpdateOp::GeneralPlanning(GeneralPlanningUpdateOp::CutPeriod(document.period_id, 2))
            .apply(&mut app_state)
            .expect("cutting a period past filled content must preserve it, not fail")
        {
            Some(NewId::PeriodId(id)) => id,
            other => panic!("Unexpected result after cutting the period: {:?}", other),
        };

    assert_cut_preserved_content(&app_state, &document, new_period_id);
}
