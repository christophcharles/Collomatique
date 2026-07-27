//! Regression tests for the bugs found by the Phase-0 property harness
//! (see the status section of docs/state_consolidation_plan.md).
//!
//! Each test pins one bug deterministically, independent of
//! property-test seed luck. Following the test-first workflow, every
//! test is committed *before* the corresponding fix and was verified to
//! fail against the unfixed code.

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    ColloscopeOp, Convergence, Data, Error, FixableInvariant, GroupListOp, GroupListPrecheckError,
    InvalidOp, NewId, NonEmptyRangeInclusive, Op, PeriodOp, PrecheckError, Reference, SettingsOp,
    SlotOp, SlotPrecheckError, StudentOp, StudentRefSite, Subject, SubjectInterrogationParameters,
    SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp, WeekOp,
    group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup},
    ids::PeriodId,
    settings::{Limits, Settings},
    slots::Slot,
    students::Student,
    teachers::Teacher,
    weeks::WeekDesc,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

/// Creates a front period with `weeks` trivially-active weeks, spliced in one
/// at a time via the `WeekOp` family — periods are created empty.
fn add_active_period(app: &mut AppState<Data, String>, weeks: usize) -> PeriodId {
    let period = match app.apply(Op::Period(PeriodOp::AddFront), "Add period".into()) {
        Ok(Some(NewId::PeriodId(id))) => id,
        other => panic!("adding a period should return a period id, got {other:?}"),
    };
    let mut prev = None;
    for _ in 0..weeks {
        let op = match prev {
            None => WeekOp::AddFront(period, WeekDesc::new(true)),
            Some(w) => WeekOp::AddAfter(w, WeekDesc::new(true)),
        };
        match app.apply(Op::Week(op), "Add week".into()) {
            Ok(Some(NewId::WeekId(w))) => prev = Some(w),
            other => panic!("adding a week should return a week id, got {other:?}"),
        }
    }
    period
}

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
            students: BTreeMap::from([(student_id, Limits::default())]).into(),
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
    assert_eq!(
        result,
        Err(Error::BrokenInvariants(BTreeSet::from([
            FixableInvariant::DanglingFk(Reference::Student {
                target: student_id,
                site: StudentRefSite::SettingsStudentKey,
            })
        ]))),
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

/// An automatic→automatic `GroupListOp::Update` (which swaps the filling
/// while keeping the params — the ex-`SetFilling` move) must check the new
/// `excluded_students` against students already placed in the colloscope
/// entry of the list, exactly like the prefilled↔automatic transitions do.
/// Before the fix, the op succeeded and left an excluded-but-placed student,
/// panicking the internal invariant check (`ExcludedStudentInGroupList`).
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
        Op::GroupList(GroupListOp::Add(
            GroupList::new(GroupListParameters::default(), GroupListFilling::default()).unwrap(),
        )),
        "Add group list".into(),
    ) else {
        panic!("Unexpected result after adding the group list");
    };

    // Place the first student in group 0 of the colloscope entry
    let Ok(None) = app_state.apply(
        Op::Colloscope(ColloscopeOp::SetGroupList(
            group_list_id,
            BTreeMap::from([(placed_student, 0)]),
        )),
        "Place student in colloscope".into(),
    ) else {
        panic!("Unexpected result after placing the student");
    };

    // Excluding the placed student must fail (whole-value update keeping params)
    let result = app_state.apply(
        Op::GroupList(GroupListOp::Update(
            group_list_id,
            GroupList::new(
                GroupListParameters::default(),
                GroupListFilling::Automatic {
                    excluded_students: BTreeSet::from([placed_student]),
                },
            )
            .unwrap(),
        )),
        "Exclude placed student".into(),
    );
    assert_eq!(
        result,
        Err(Error::BrokenInvariants(BTreeSet::from([
            FixableInvariant::Convergence(Convergence::ColloscopeStudentExcluded(
                group_list_id,
                placed_student,
            ))
        ]))),
    );

    // Excluding a student that is not placed still works
    let Ok(None) = app_state.apply(
        Op::GroupList(GroupListOp::Update(
            group_list_id,
            GroupList::new(
                GroupListParameters::default(),
                GroupListFilling::Automatic {
                    excluded_students: BTreeSet::from([other_student]),
                },
            )
            .unwrap(),
        )),
        "Exclude non-placed student".into(),
    ) else {
        panic!("Excluding a non-placed student should succeed");
    };
}

/// `GroupListOp::Update` must check the interrogations' `assigned_groups`
/// of every subject associated with the list when shrinking
/// `group_names`, exactly like `AssignToSubject` does. Before the fix,
/// shrinking below an assigned group number succeeded and panicked the
/// internal invariant check (`InvalidGroupNumInInterrogation`).
#[test]
fn update_shrinking_group_names_below_assigned_group_is_rejected() {
    let mut app_state = AppState::<_, String>::new(Data::new());

    let period_id = add_active_period(&mut app_state, 2);

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
            },
        )),
        "Add subject".into(),
    ) else {
        panic!("Unexpected result after adding the subject");
    };

    let Ok(Some(NewId::TeacherId(teacher_id))) = app_state.apply(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject_id]),
        })),
        "Add teacher".into(),
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
        "Add slot".into(),
    ) else {
        panic!("Unexpected result after adding the slot");
    };

    // Group list with 4 groups, associated with the subject
    let Ok(Some(NewId::GroupListId(group_list_id))) = app_state.apply(
        Op::GroupList(GroupListOp::Add(
            GroupList::new(
                GroupListParameters {
                    name: "Liste".into(),
                    students_per_group: NonEmptyRangeInclusive::new(
                        NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                    )
                    .expect("statically non-empty"),
                    group_names: vec![None; 4],
                },
                GroupListFilling::default(),
            )
            .unwrap(),
        )),
        "Add group list".into(),
    ) else {
        panic!("Unexpected result after adding the group list");
    };
    let Ok(None) = app_state.apply(
        Op::GroupList(GroupListOp::AssignToSubject(
            period_id,
            subject_id,
            Some(group_list_id),
        )),
        "Assign group list to subject".into(),
    ) else {
        panic!("Unexpected result after assigning the group list");
    };

    // Assign group number 2 in an interrogation of the slot
    let week0 = app_state
        .get_data()
        .get_inner_data()
        .params
        .weeks
        .week_id_at(period_id, 0)
        .expect("period has a first week");
    let Ok(None) = app_state.apply(
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot_id,
            week0,
            BTreeSet::from([2]),
        )),
        "Assign group 2 in interrogation".into(),
    ) else {
        panic!("Unexpected result after updating the interrogation");
    };

    // Shrinking group_names to 2 groups (max valid group number 1) must fail
    let result = app_state.apply(
        Op::GroupList(GroupListOp::Update(
            group_list_id,
            GroupList::new(
                GroupListParameters {
                    name: "Liste".into(),
                    students_per_group: NonEmptyRangeInclusive::new(
                        NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                    )
                    .expect("statically non-empty"),
                    group_names: vec![None; 2],
                },
                GroupListFilling::default(),
            )
            .unwrap(),
        )),
        "Shrink group list below assigned group".into(),
    );
    assert_eq!(
        result,
        Err(Error::BrokenInvariants(BTreeSet::from([
            FixableInvariant::Convergence(Convergence::InterrogationGroupOutOfBounds(
                slot_id, week0,
            ))
        ]))),
    );

    // Shrinking to 3 groups keeps group number 2 valid and succeeds
    let Ok(None) = app_state.apply(
        Op::GroupList(GroupListOp::Update(
            group_list_id,
            GroupList::new(
                GroupListParameters {
                    name: "Liste".into(),
                    students_per_group: NonEmptyRangeInclusive::new(
                        NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                    )
                    .expect("statically non-empty"),
                    group_names: vec![None; 3],
                },
                GroupListFilling::default(),
            )
            .unwrap(),
        )),
        "Shrink group list above assigned group".into(),
    ) else {
        panic!("Shrinking above the assigned group number should succeed");
    };
}

/// The reverse of a `GroupListOp::Remove` must restore the group list
/// exactly, including its filling kind. Before the fix, the reverse was
/// rebuilt as a plain `Add(id, params)` with the default (automatic,
/// empty) filling: undoing the removal of a prefilled (empty) group
/// list flipped it to automatic and re-registered a colloscope entry.
#[test]
fn remove_prefilled_group_list_round_trips_on_reverse() {
    use collomatique_state::InMemoryData;

    let mut app_state = AppState::<_, String>::new(Data::new());

    let Ok(Some(NewId::GroupListId(group_list_id))) = app_state.apply(
        Op::GroupList(GroupListOp::Add(
            GroupList::new(GroupListParameters::default(), GroupListFilling::default()).unwrap(),
        )),
        "Add group list".into(),
    ) else {
        panic!("Unexpected result after adding the group list");
    };
    let group_count = 16; // GroupListParameters::default() has 16 groups
    let Ok(None) = app_state.apply(
        Op::GroupList(GroupListOp::Update(
            group_list_id,
            GroupList::new(
                GroupListParameters::default(),
                GroupListFilling::Prefilled {
                    groups: vec![PrefilledGroup::default(); group_count],
                },
            )
            .unwrap(),
        )),
        "Make the group list prefilled".into(),
    ) else {
        panic!("Unexpected result after setting the prefilled filling");
    };

    // Same annotate → apply order as Manager::apply
    let mut data: Data = app_state.get_data().clone();
    let before = data.clone();

    let (annotated, _new_id) = data.annotate(Op::GroupList(GroupListOp::Remove(group_list_id)));
    let rev = data
        .apply(&annotated)
        .expect("removing an empty prefilled group list should succeed");
    data.apply(&rev)
        .expect("the reverse of a successfully applied op must apply");

    assert!(
        data == before,
        "undoing the removal must restore the prefilled filling \
         and must not register a colloscope entry",
    );
}

/// `GroupListOp::AssignToSubject` with a dangling group-list id must
/// return `InvalidGroupListId`. Before the fix, it panicked on an
/// `.expect("Group list ID should be valid")` placed before the actual
/// id check (which was therefore dead code).
#[test]
fn assign_to_subject_with_dangling_group_list_id_errors() {
    let mut app_state = AppState::<_, String>::new(Data::new());

    let period_id = add_active_period(&mut app_state, 2);

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
            },
        )),
        "Add subject".into(),
    ) else {
        panic!("Unexpected result after adding the subject");
    };

    // A removed group list leaves a dangling id
    let Ok(Some(NewId::GroupListId(group_list_id))) = app_state.apply(
        Op::GroupList(GroupListOp::Add(
            GroupList::new(GroupListParameters::default(), GroupListFilling::default()).unwrap(),
        )),
        "Add group list".into(),
    ) else {
        panic!("Unexpected result after adding the group list");
    };
    let Ok(None) = app_state.apply(
        Op::GroupList(GroupListOp::Remove(group_list_id)),
        "Remove group list".into(),
    ) else {
        panic!("Unexpected result after removing the group list");
    };

    let result = app_state.apply(
        Op::GroupList(GroupListOp::AssignToSubject(
            period_id,
            subject_id,
            Some(group_list_id),
        )),
        "Assign dangling group list".into(),
    );
    assert_eq!(
        result,
        Err(Error::InvalidOp(InvalidOp::Precheck(
            PrecheckError::GroupList(GroupListPrecheckError::InvalidGroupListId(group_list_id))
        ))),
    );
}

/// `SlotOp::Update` must reject an update that moves the slot to a
/// different subject: `slot.subject_id` is authoritative and the slots
/// backend groups slots per subject. Since the flat-table restructure
/// (phase B commit 3), the subject is carried by the slot itself; without
/// this guard a changed `subject_id` would desynchronize the slot table
/// from the per-subject ordering sidecar.
#[test]
fn slot_update_changing_subject_is_rejected() {
    let mut app_state = AppState::<_, String>::new(Data::new());

    let _period_id = add_active_period(&mut app_state, 2);

    let interrogation_subject = |name: &str| Subject {
        parameters: SubjectParameters {
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
        },
        excluded_periods: BTreeSet::new(),
    };

    let Ok(Some(NewId::SubjectId(subject_a))) = app_state.apply(
        Op::Subject(SubjectOp::AddAfter(None, interrogation_subject("Math"))),
        "Add subject A".into(),
    ) else {
        panic!("Unexpected result after adding subject A");
    };
    let Ok(Some(NewId::SubjectId(subject_b))) = app_state.apply(
        Op::Subject(SubjectOp::AddAfter(
            Some(subject_a),
            interrogation_subject("Physics"),
        )),
        "Add subject B".into(),
    ) else {
        panic!("Unexpected result after adding subject B");
    };

    // A teacher that teaches in both subjects, so the update would pass
    // teacher validation and only the subject-change guard can reject it.
    let Ok(Some(NewId::TeacherId(teacher_id))) = app_state.apply(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject_a, subject_b]),
        })),
        "Add teacher".into(),
    ) else {
        panic!("Unexpected result after adding the teacher");
    };

    let slot = |subject_id| Slot {
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
    };

    let Ok(Some(NewId::SlotId(slot_id))) = app_state.apply(
        Op::Slot(SlotOp::AddAfter(None, slot(subject_a))),
        "Add slot to subject A".into(),
    ) else {
        panic!("Unexpected result after adding the slot");
    };

    // Updating the slot with a different (but otherwise valid) subject must fail.
    let result = app_state.apply(
        Op::Slot(SlotOp::Update(slot_id, slot(subject_b))),
        "Move slot to subject B".into(),
    );
    assert_eq!(
        result,
        Err(Error::InvalidOp(InvalidOp::Precheck(PrecheckError::Slot(
            SlotPrecheckError::CannotChangeSubject(slot_id, subject_a, subject_b)
        )))),
    );
}
