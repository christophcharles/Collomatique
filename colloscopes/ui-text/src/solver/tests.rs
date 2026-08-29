//! What [`fixed_pin_violation_text`] says, word for word.
//!
//! The four sentences are the whole point of the function, so they are pinned
//! exactly, against a document small enough to read: one subject with one slot,
//! one student, one group list whose first group is named and whose second is
//! not.
//!
//! The `Debug` fallbacks are not tested here — nothing in this crate can build
//! a [`GroupNum`] for a group list the parameters do not hold, which is the only
//! id a caller could plausibly get wrong.

use super::*;

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::group_lists::{
    GroupList, GroupListFilling, GroupListParameters,
};
use collomatique_state_colloscopes::slots::Slot;
use collomatique_state_colloscopes::students::Student;
use collomatique_state_colloscopes::teachers::Teacher;
use collomatique_state_colloscopes::{
    Data, GroupListOp, NewId, NonEmptyRangeInclusive, Op, SlotOp, StudentOp, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp,
};
use collomatique_state_colloscopes::{PersonWithContact, SlotId, StudentId};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

/// The ids of the little document [`document`] builds.
struct Built {
    data: Data,
    slot: SlotId,
    student: StudentId,
    group_list: GroupListId,
}

impl Built {
    fn params(&self) -> &Parameters {
        &self.data.get_inner_data().params
    }

    fn group(&self, index: usize) -> GroupNum {
        GroupNum::new(self.params(), self.group_list, index).expect("group index is in range")
    }
}

/// « Potions » held on Monday at 14h00, Harry Potter, and « Sixième année »
/// with a named group 1 and an unnamed group 2.
fn document() -> Built {
    let mut state = AppState::<Data, String>::new(Data::default());

    macro_rules! apply_new {
        ($op:expr, $variant:path, $what:expr) => {{
            let Ok(Some($variant(id))) = state.apply($op, $what.to_string()) else {
                panic!(concat!("adding the ", $what, " should land"));
            };
            id
        }};
    }

    let subject = apply_new!(
        Op::Subject(SubjectOp::AddAfter(
            None,
            Subject {
                parameters: SubjectParameters {
                    name: "Potions".into(),
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
        NewId::SubjectId,
        "subject"
    );

    let teacher = apply_new!(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: PersonWithContact {
                surname: "Rogue".into(),
                firstname: "Séverus".into(),
                ..Default::default()
            },
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "teacher"
    );

    let slot = apply_new!(
        Op::Slot(SlotOp::AddAfter(
            None,
            Slot {
                subject_id: subject,
                teacher_id: teacher,
                start_time: collomatique_time::SlotStart {
                    weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
                    start_time: collomatique_time::WholeMinuteTime::new(
                        chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap(),
                    )
                    .unwrap(),
                },
                extra_info: String::new(),
                week_pattern: None,
                cost: 0,
            },
        )),
        NewId::SlotId,
        "slot"
    );

    let student = apply_new!(
        Op::Student(StudentOp::Add(Student {
            desc: PersonWithContact {
                surname: "Potter".into(),
                firstname: "Harry".into(),
                ..Default::default()
            },
            excluded_periods: BTreeSet::new(),
        })),
        NewId::StudentId,
        "student"
    );

    let group_list = apply_new!(
        Op::GroupList(GroupListOp::Add(
            GroupList::new(
                GroupListParameters {
                    name: "Sixième année".into(),
                    students_per_group: NonEmptyRangeInclusive::new(
                        NonZeroU32::new(1).unwrap()..=NonZeroU32::new(3).unwrap(),
                    )
                    .expect("statically non-empty"),
                    group_names: vec![Some("Gryffondor".try_into().expect("non-empty")), None,],
                },
                GroupListFilling::Automatic {
                    excluded_students: BTreeSet::new(),
                },
            )
            .expect("an automatic group list has nothing to check")
        )),
        NewId::GroupListId,
        "group list"
    );

    Built {
        data: state.get_data().clone(),
        slot,
        student,
        group_list,
    }
}

#[test]
fn a_group_pinned_into_an_interrogation() {
    let built = document();
    let var = Var::GroupInInterrogation {
        slot: built.slot,
        week: collomatique_constraints_colloscopes::ids::GlobalWeek(4),
        group: built.group(0),
    };

    assert_eq!(
        fixed_pin_violation_text(&var, 1.0, built.params()),
        "La configuration de résolution imposait que le groupe 1 (Gryffondor) passe en colle sur \
         le créneau Potions (lundi 14h00) la semaine 5, mais le colloscope ne le respecte pas."
    );
}

#[test]
fn a_group_pinned_out_of_an_interrogation() {
    let built = document();
    // The unnamed group, so its rendering is the bare number.
    let var = Var::GroupInInterrogation {
        slot: built.slot,
        week: collomatique_constraints_colloscopes::ids::GlobalWeek(0),
        group: built.group(1),
    };

    assert_eq!(
        fixed_pin_violation_text(&var, 0.0, built.params()),
        "La configuration de résolution imposait que le groupe 2 ne passe pas en colle sur le \
         créneau Potions (lundi 14h00) la semaine 1, mais le colloscope ne le respecte pas."
    );
}

#[test]
fn a_student_pinned_into_a_group() {
    let built = document();
    let var = Var::StudentInGroup {
        group_list: built.group_list,
        student: built.student,
        group: built.group(0),
    };

    assert_eq!(
        fixed_pin_violation_text(&var, 1.0, built.params()),
        "La configuration de résolution imposait que l'élève Harry Potter soit dans le groupe 1 \
         (Gryffondor) de la liste Sixième année, mais le colloscope ne le respecte pas."
    );
}

#[test]
fn a_student_pinned_out_of_a_group() {
    let built = document();
    let var = Var::StudentInGroup {
        group_list: built.group_list,
        student: built.student,
        group: built.group(1),
    };

    assert_eq!(
        fixed_pin_violation_text(&var, 0.0, built.params()),
        "La configuration de résolution imposait que l'élève Harry Potter ne soit pas dans le \
         groupe 2 de la liste Sixième année, mais le colloscope ne le respecte pas."
    );
}
