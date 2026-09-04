//! The global group-list ops: `UpdateGroupList` and the widened
//! `AddNewGroupList`.
//!
//! Both now carry a sealed [`GroupList`] — parameters *and* filling in one
//! payload — instead of one half each. That merge has to hold two lines at
//! once:
//!
//! * the *error* surface grows a student-existence sweep on both ops (the
//!   filling can name students, so `AddNewGroupList` can fail for the first
//!   time), and
//! * the *repair* surface must still fix everything that hangs off the
//!   list — colloscope placements and interrogation cells — while saying
//!   nothing about the payload itself.
//!
//! That second line is where the merge changes behaviour on purpose. The
//! split ops warned when a shrink dropped a prefilled group holding students,
//! because a parameters-only op had to guess what became of a filling its
//! caller could not touch. One payload ends the guessing: the caller describes
//! the whole list, so a group they deleted is their own edit and gets no
//! warning. The colloscope, which they never saw, still does.
//!
//! The tests below pin one case each: out-of-range placements, out-of-range
//! interrogation groups, newly-excluded students, and the non-prefilled →
//! prefilled transition — plus the silence of the shrink. The repairs are the
//! cascade's, so each is read back as the [Fix] it landed.

use collomatique_ops::{
    AddNewGroupListError, CascadeWarning, GroupListsUpdateError, GroupListsUpdateOp, OpCategory,
    UpdateError, UpdateGroupListError, UpdateOp,
};
use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::ids::{GroupListId, PeriodId, StudentId};
use collomatique_state_colloscopes::students::Student;
use collomatique_state_colloscopes::{
    ColloscopeOp, Data, Fix, GroupListOp, NewId, NonEmptyRangeInclusive, Op, PeriodOp, SlotOp,
    StudentOp, Subject, SubjectInterrogationParameters, SubjectOp, SubjectParameters,
    SubjectPeriodicity, TeacherOp, WeekOp,
    group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup},
    slots::Slot,
    teachers::Teacher,
    weeks::WeekDesc,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

type Desc = (OpCategory, String);

fn desc(text: &str) -> Desc {
    (OpCategory::None, text.to_string())
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

/// Same trick for a group list: add one, then take it away.
fn dead_group_list_id(app: &mut AppState<Data, Desc>) -> GroupListId {
    let id = add_group_list(app, params(1), GroupListFilling::default());
    app.apply(
        Op::GroupList(GroupListOp::Remove(id)),
        desc("Remove group list"),
    )
    .expect("removing an unreferenced group list succeeds");
    id
}

/// Group-list parameters with `count` unnamed groups.
fn params(count: usize) -> GroupListParameters {
    GroupListParameters {
        name: "Liste".into(),
        students_per_group: NonEmptyRangeInclusive::new(
            NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
        )
        .expect("statically non-empty"),
        group_names: vec![None; count],
    }
}

fn prefilled(groups: impl IntoIterator<Item = BTreeSet<StudentId>>) -> GroupListFilling {
    GroupListFilling::Prefilled {
        groups: groups
            .into_iter()
            .map(|students| PrefilledGroup { students })
            .collect(),
    }
}

fn automatic(excluded: impl IntoIterator<Item = StudentId>) -> GroupListFilling {
    GroupListFilling::Automatic {
        excluded_students: excluded.into_iter().collect(),
    }
}

fn add_group_list(
    app: &mut AppState<Data, Desc>,
    params: GroupListParameters,
    filling: GroupListFilling,
) -> GroupListId {
    let group_list = GroupList::new(params, filling).expect("test fixture is consistent");
    match app.apply(
        Op::GroupList(GroupListOp::Add(group_list)),
        desc("Add list"),
    ) {
        Ok(Some(NewId::GroupListId(id))) => id,
        other => panic!("adding a group list should return a group list id, got {other:?}"),
    }
}

fn set_placements(
    app: &mut AppState<Data, Desc>,
    id: GroupListId,
    placements: BTreeMap<StudentId, u32>,
) {
    app.apply(
        Op::Colloscope(ColloscopeOp::SetGroupList(id, placements)),
        desc("Place students"),
    )
    .expect("placing students in a live automatic group list succeeds");
}

/// Runs `UpdateGroupList` and returns the error it produced, failing the test
/// on success or on any other error shape.
fn update_err(
    app: &AppState<Data, Desc>,
    id: GroupListId,
    group_list: GroupList,
) -> UpdateGroupListError {
    let op = UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(id, group_list));
    match op.dry_apply(app).map(|_| ()) {
        Err(UpdateError::GroupLists(GroupListsUpdateError::UpdateGroupList(e))) => e,
        other => panic!("expected an UpdateGroupList error, got {other:?}"),
    }
}

/// The repairs a cascade logged, read back as the [Fix] values the fixtures
/// write down.
fn fixes(warnings: &[CascadeWarning]) -> Vec<Fix> {
    warnings.iter().map(|w| w.fix().clone()).collect()
}

#[test]
fn replacing_a_group_list_that_does_not_exist_reports_the_id() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let dead = dead_group_list_id(&mut app);

    assert_eq!(
        update_err(
            &app,
            dead,
            GroupList::new(params(2), GroupListFilling::default()).unwrap(),
        ),
        UpdateGroupListError::InvalidGroupListId(dead),
    );
}

#[test]
fn a_prefilled_payload_naming_a_dead_student_reports_the_student() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let id = add_group_list(&mut app, params(2), GroupListFilling::default());
    let dead = dead_student_id(&mut app);

    assert_eq!(
        update_err(
            &app,
            id,
            GroupList::new(
                params(2),
                prefilled([BTreeSet::from([dead]), BTreeSet::new()]),
            )
            .unwrap(),
        ),
        UpdateGroupListError::InvalidStudentId(dead),
    );
}

#[test]
fn an_automatic_payload_excluding_a_dead_student_reports_the_student() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let id = add_group_list(&mut app, params(2), GroupListFilling::default());
    let dead = dead_student_id(&mut app);

    // The other half of the sweep: `GroupListFilling::iter_students` covers the
    // prefilled groups only, so an excluded set that is never walked would let
    // a dangling id reach the state layer.
    assert_eq!(
        update_err(
            &app,
            id,
            GroupList::new(params(2), automatic([dead])).unwrap(),
        ),
        UpdateGroupListError::InvalidStudentId(dead),
    );
}

#[test]
fn shrinking_a_prefilled_list_lands_verbatim_and_says_nothing() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let s0 = add_student(&mut app);
    let s1 = add_student(&mut app);
    let s2 = add_student(&mut app);
    let id = add_group_list(
        &mut app,
        params(3),
        prefilled([
            BTreeSet::from([s0]),
            BTreeSet::from([s1]),
            BTreeSet::from([s2]),
        ]),
    );

    // Two groups instead of three, and the third group's student is simply not
    // in the payload. The split ops warned here, because a parameters-only
    // shrink had to guess what became of a filling its caller could not touch.
    // One payload ends that guessing: dropping the group *is* the caller's
    // edit, so there is nothing to tell them they did not already say.
    let payload = GroupList::new(
        params(2),
        prefilled([BTreeSet::from([s0]), BTreeSet::from([s1])]),
    )
    .unwrap();
    let outcome = UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(id, payload.clone()))
        .dry_apply(&app)
        .expect("shrinking a prefilled list must succeed");

    assert_eq!(fixes(&outcome.warnings), Vec::new());

    let stored = outcome
        .new_state
        .get_data()
        .get_inner_data()
        .params
        .group_lists
        .group_list_map
        .get(&id)
        .expect("the list is still there")
        .clone();
    assert_eq!(stored, payload);
}

#[test]
fn shrinking_removes_the_placements_of_groups_that_disappear() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let s0 = add_student(&mut app);
    let s1 = add_student(&mut app);
    let id = add_group_list(&mut app, params(3), GroupListFilling::default());
    set_placements(&mut app, id, BTreeMap::from([(s0, 0), (s1, 2)]));

    let payload = GroupList::new(params(2), GroupListFilling::default()).unwrap();
    let outcome = UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(id, payload))
        .dry_apply(&app)
        .expect("shrinking past a placement must auto-clean, not fail");

    assert_eq!(
        fixes(&outcome.warnings),
        vec![Fix::RemoveStudentColloscopePlacement {
            group_list: id,
            student: s1,
            rebuilt: BTreeMap::from([(s0, 0)]),
        }],
    );
    assert_eq!(
        outcome
            .new_state
            .get_data()
            .get_inner_data()
            .colloscope
            .group_list(id),
        Some(&BTreeMap::from([(s0, 0)])),
    );
}

#[test]
fn excluding_a_placed_student_removes_the_placement() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let s0 = add_student(&mut app);
    let s1 = add_student(&mut app);
    let id = add_group_list(&mut app, params(2), GroupListFilling::default());
    set_placements(&mut app, id, BTreeMap::from([(s0, 0), (s1, 1)]));

    let payload = GroupList::new(params(2), automatic([s1])).unwrap();
    let outcome = UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(id, payload))
        .dry_apply(&app)
        .expect("excluding a placed student must auto-clean, not fail");

    assert_eq!(
        fixes(&outcome.warnings),
        vec![Fix::RemoveStudentColloscopePlacement {
            group_list: id,
            student: s1,
            rebuilt: BTreeMap::from([(s0, 0)]),
        }],
    );
    assert_eq!(
        outcome
            .new_state
            .get_data()
            .get_inner_data()
            .colloscope
            .group_list(id),
        Some(&BTreeMap::from([(s0, 0)])),
    );
}

#[test]
fn becoming_prefilled_empties_the_colloscope_placement_row() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let s0 = add_student(&mut app);
    let s1 = add_student(&mut app);
    let id = add_group_list(&mut app, params(2), GroupListFilling::default());
    set_placements(&mut app, id, BTreeMap::from([(s0, 0), (s1, 1)]));

    // A prefilled list holds no colloscope row at all, so the whole row goes at
    // once: for a prefilled list there is no single element to blame, so the
    // cascade clears the row and says so in one sentence (the old cleaning path
    // took the students out one at a time and warned once per student).
    let payload = GroupList::new(
        params(2),
        prefilled([BTreeSet::from([s0]), BTreeSet::from([s1])]),
    )
    .unwrap();
    let outcome = UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(id, payload))
        .dry_apply(&app)
        .expect("prefilling a placed list must auto-clean, not fail");

    assert_eq!(
        fixes(&outcome.warnings),
        vec![Fix::ClearColloscopeGroupListRow { group_list: id }],
    );
    assert_eq!(
        outcome
            .new_state
            .get_data()
            .get_inner_data()
            .colloscope
            .group_list(id),
        None,
    );
}

/// A period holding `weeks` active weeks, spliced in one at a time — periods
/// are created empty.
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

#[test]
fn shrinking_trims_out_of_range_interrogation_groups() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let period = add_active_period(&mut app, 1);

    let Ok(Some(NewId::SubjectId(subject_id))) = app.apply(
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
                            NonZeroU32::new(1).unwrap()..=NonZeroU32::new(2).unwrap(),
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

    let Ok(Some(NewId::TeacherId(teacher_id))) = app.apply(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject_id]),
        })),
        desc("Add teacher"),
    ) else {
        panic!("Unexpected result after adding the teacher");
    };

    let Ok(Some(NewId::SlotId(slot_id))) = app.apply(
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

    let id = add_group_list(&mut app, params(3), GroupListFilling::default());
    app.apply(
        Op::GroupList(GroupListOp::AssignToSubject(period, subject_id, Some(id))),
        desc("Assign group list to subject"),
    )
    .expect("assigning a live group list to a running subject succeeds");

    let week0 = app
        .get_data()
        .get_inner_data()
        .params
        .weeks
        .week_id_at(period, 0)
        .expect("the period has a first week");
    app.apply(
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot_id,
            week0,
            BTreeSet::from([0, 2]),
        )),
        desc("Put an interrogation on the first week"),
    )
    .expect("interrogating groups 0 and 2 of a three-group list succeeds");

    // Down to two groups: group 2 no longer exists, so the interrogation cell
    // that names it has to be trimmed before the replacement lands.
    let payload = GroupList::new(params(2), GroupListFilling::default()).unwrap();
    let outcome = UpdateOp::GroupLists(GroupListsUpdateOp::UpdateGroupList(id, payload))
        .dry_apply(&app)
        .expect("shrinking past an interrogation group must auto-clean, not fail");

    assert_eq!(
        fixes(&outcome.warnings),
        vec![Fix::RemoveGroupsFromInterrogationCell {
            slot: slot_id,
            week: week0,
            groups: BTreeSet::from([2]),
            rebuilt: BTreeSet::from([0]),
        }],
    );
    assert_eq!(
        outcome
            .new_state
            .get_data()
            .get_inner_data()
            .colloscope
            .interrogation(slot_id, week0),
        Some(&BTreeSet::from([0])),
    );
}

#[test]
fn adding_a_list_whose_filling_names_a_dead_student_reports_the_student() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let dead = dead_student_id(&mut app);

    // `AddNewGroupList` could not fail at all while its payload was the
    // parameters only. Without this sweep the dangling id would reach the state
    // layer and blow up the translator's `.expect`.
    let payload = GroupList::new(
        params(2),
        prefilled([BTreeSet::from([dead]), BTreeSet::new()]),
    )
    .unwrap();
    let op = UpdateOp::GroupLists(GroupListsUpdateOp::AddNewGroupList(payload));
    match op.dry_apply(&app).map(|_| ()) {
        Err(UpdateError::GroupLists(GroupListsUpdateError::AddNewGroupList(e))) => {
            assert_eq!(e, AddNewGroupListError::InvalidStudentId(dead))
        }
        other => panic!("expected an AddNewGroupList error, got {other:?}"),
    }
}

#[test]
fn adding_a_list_keeps_the_prefilled_filling_it_was_given() {
    let mut app = AppState::<Data, Desc>::new(Data::new());
    let s0 = add_student(&mut app);
    let s1 = add_student(&mut app);

    let payload = GroupList::new(
        params(2),
        prefilled([BTreeSet::from([s0]), BTreeSet::from([s1])]),
    )
    .unwrap();
    let outcome = UpdateOp::GroupLists(GroupListsUpdateOp::AddNewGroupList(payload.clone()))
        .dry_apply(&app)
        .expect("adding a prefilled list must succeed");

    let Some(NewId::GroupListId(new_id)) = outcome.new_id else {
        panic!(
            "adding a group list should return its id, got {:?}",
            outcome.new_id
        );
    };
    // The point of the widened payload: the filling survives the trip, where
    // the old op forced every new list to be automatic.
    assert_eq!(
        outcome
            .new_state
            .get_data()
            .get_inner_data()
            .params
            .group_lists
            .group_list_map
            .get(&new_id),
        Some(&payload),
    );
}
