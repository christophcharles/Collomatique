//! Tests for the keyed read interface (`Parameters::lookup` / `resolve`).
//!
//! One entity of every kind is built through the public op API; then each of
//! the ten `Lookup` impls is exercised: `lookup` returns the *live* borrow out
//! of its container (checked by pointer identity, so nothing is cloned or
//! rebuilt), a dangling id resolves to `None`, and `resolve` round-trips on
//! valid ids while panicking on a dangling one.
//!
//! Later phase-C commits (Join views, `all_ids`) extend this file.

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    Data, GroupListOp, IncompatOp, NewId, Op, PairingOp, PeriodOp, SlotOp, SlotPairingOp,
    StudentOp, Subject, SubjectInterrogationParameters, SubjectOp, SubjectParameters,
    SubjectPeriodicity, TeacherOp, WeekPatternOp,
    group_lists::GroupListParameters,
    ids::{
        GroupListId, Id, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
        SubjectId, TeacherId, WeekPatternId,
    },
    incompats::Incompatibility,
    pairings::{PairingRule, RulePart},
    periods::WeekDesc,
    slot_pairings::{SlotPairingRule, SlotRulePart},
    slots::Slot,
    students::Student,
    teachers::Teacher,
    week_patterns::WeekPattern,
};
use std::collections::BTreeSet;
use std::num::NonZeroU32;

/// A subject that has interrogations (so it can host slots, incompats…).
fn interrogation_subject(name: &str) -> Subject {
    Subject {
        parameters: SubjectParameters {
            name: name.into(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: NonZeroU32::new(2).unwrap()..=NonZeroU32::new(3).unwrap(),
                groups_per_interrogation: NonZeroU32::new(1).unwrap()..=NonZeroU32::new(1).unwrap(),
                duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
                },
            }),
        },
        excluded_periods: BTreeSet::new(),
    }
}

fn make_slot(
    subject_id: SubjectId,
    teacher_id: TeacherId,
    week_pattern: Option<WeekPatternId>,
) -> Slot {
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
        week_pattern,
        cost: 0,
    }
}

fn one_group_params(name: &str) -> GroupListParameters {
    GroupListParameters {
        name: name.into(),
        students_per_group: NonZeroU32::new(1).unwrap()..=NonZeroU32::new(3).unwrap(),
        group_names: vec![None],
    }
}

/// Ids captured while building the document, one per entity kind.
struct Built {
    period: PeriodId,
    subject: SubjectId,
    teacher: TeacherId,
    student: StudentId,
    week_pattern: WeekPatternId,
    slot: SlotId,
    incompat: IncompatId,
    group_list: GroupListId,
    pairing: PairingRuleId,
    slot_pairing: SlotPairingRuleId,
}

/// Builds a document holding exactly one entity of each kind.
fn build_document(app: &mut AppState<Data, String>) -> Built {
    macro_rules! apply_new {
        ($op:expr, $variant:path, $msg:expr) => {{
            let Ok(Some($variant(id))) = app.apply($op, $msg.into()) else {
                panic!(concat!("unexpected result: ", $msg));
            };
            id
        }};
    }

    // One one-week period → total week count is 1.
    let period = apply_new!(
        Op::Period(PeriodOp::AddFront(vec![WeekDesc::new(true)])),
        NewId::PeriodId,
        "add period"
    );
    // Week pattern length must match the total week count.
    let week_pattern = apply_new!(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "WP".into(),
            weeks: vec![true],
        })),
        NewId::WeekPatternId,
        "add week pattern"
    );
    // Two interrogation subjects (the pairing rule needs two distinct ones).
    let subject = apply_new!(
        Op::Subject(SubjectOp::AddAfter(None, interrogation_subject("Math"))),
        NewId::SubjectId,
        "add math"
    );
    let phys = apply_new!(
        Op::Subject(SubjectOp::AddAfter(
            Some(subject),
            interrogation_subject("Physics"),
        )),
        NewId::SubjectId,
        "add physics"
    );
    let teacher = apply_new!(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject, phys]),
        })),
        NewId::TeacherId,
        "add teacher"
    );
    let student = apply_new!(
        Op::Student(StudentOp::Add(Student::default())),
        NewId::StudentId,
        "add student"
    );
    // Two slots in Math (the slot pairing needs two of the same subject).
    let slot = apply_new!(
        Op::Slot(SlotOp::AddAfter(
            None,
            make_slot(subject, teacher, Some(week_pattern)),
        )),
        NewId::SlotId,
        "add slot 1"
    );
    let slot2 = apply_new!(
        Op::Slot(SlotOp::AddAfter(
            Some(slot),
            make_slot(subject, teacher, None)
        )),
        NewId::SlotId,
        "add slot 2"
    );
    let incompat = apply_new!(
        Op::Incompat(IncompatOp::Add(Incompatibility {
            subject_id: subject,
            name: "Inc".into(),
            slots: vec![],
            minimum_free_slots: NonZeroU32::new(1).unwrap(),
            week_pattern_id: Some(week_pattern),
        })),
        NewId::IncompatId,
        "add incompat"
    );
    let group_list = apply_new!(
        Op::GroupList(GroupListOp::Add(one_group_params("GL"))),
        NewId::GroupListId,
        "add group list"
    );
    let pairing = apply_new!(
        Op::Pairing(PairingOp::Add(PairingRule {
            antecedent: RulePart {
                subject_id: subject,
                should_have: true,
            },
            consequent: RulePart {
                subject_id: phys,
                should_have: true,
            },
            excluded_periods: BTreeSet::new(),
            soft: false,
        })),
        NewId::PairingRuleId,
        "add pairing"
    );
    let slot_pairing = apply_new!(
        Op::SlotPairing(SlotPairingOp::Add(SlotPairingRule {
            antecedent: SlotRulePart {
                slot_id: slot,
                should_have: true,
            },
            consequent: SlotRulePart {
                slot_id: slot2,
                should_have: true,
            },
            excluded_periods: BTreeSet::new(),
            soft: false,
        })),
        NewId::SlotPairingRuleId,
        "add slot pairing"
    );

    Built {
        period,
        subject,
        teacher,
        student,
        week_pattern,
        slot,
        incompat,
        group_list,
        pairing,
        slot_pairing,
    }
}

#[test]
fn lookup_borrows_the_live_entity_for_every_kind() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let params = &app.get_data().get_inner_data().params;

    // `lookup` and `resolve` must both hand back the very borrow that lives in
    // the entity's container — proven by pointer identity, not just equality.
    macro_rules! assert_resolves_to {
        ($id:expr, $container:expr) => {{
            let expected = ($container).expect("entity present in its container");
            let via_lookup = params.lookup($id).expect("lookup resolves a live id");
            assert!(
                std::ptr::eq(via_lookup, expected),
                "lookup must borrow the container entity, not a copy",
            );
            assert!(
                std::ptr::eq(params.resolve($id), expected),
                "resolve must return the same borrow as lookup",
            );
        }};
    }

    assert_resolves_to!(ids.period, params.periods.find_period(ids.period));
    assert_resolves_to!(ids.subject, params.subjects.find_subject(ids.subject));
    assert_resolves_to!(ids.teacher, params.teachers.teacher_map.get(&ids.teacher));
    assert_resolves_to!(ids.student, params.students.student_map.get(&ids.student));
    assert_resolves_to!(
        ids.week_pattern,
        params.week_patterns.week_pattern_map.get(&ids.week_pattern)
    );
    assert_resolves_to!(ids.slot, params.slots.find_slot(ids.slot));
    assert_resolves_to!(
        ids.incompat,
        params.incompats.incompat_map.get(&ids.incompat)
    );
    assert_resolves_to!(
        ids.group_list,
        params.group_lists.group_list_map.get(&ids.group_list)
    );
    assert_resolves_to!(
        ids.pairing,
        params.pairings.pairing_rule_map.get(&ids.pairing)
    );
    assert_resolves_to!(
        ids.slot_pairing,
        params
            .slot_pairings
            .slot_pairing_rule_map
            .get(&ids.slot_pairing)
    );
}

#[test]
fn lookup_returns_none_for_a_dangling_id_of_every_kind() {
    let mut app = AppState::<_, String>::new(Data::new());
    let _ = build_document(&mut app);
    let params = &app.get_data().get_inner_data().params;

    // Real ids are issued from 0 and never reach 1 << 40 (cf. the property
    // harness' DANGLING_BASE), so these are guaranteed to dangle.
    macro_rules! assert_dangling_none {
        ($ty:ty) => {{
            let id = unsafe { <$ty>::new(1u64 << 40) };
            assert!(
                params.lookup(id).is_none(),
                concat!("dangling ", stringify!($ty), " must not resolve"),
            );
        }};
    }

    assert_dangling_none!(PeriodId);
    assert_dangling_none!(SubjectId);
    assert_dangling_none!(TeacherId);
    assert_dangling_none!(StudentId);
    assert_dangling_none!(WeekPatternId);
    assert_dangling_none!(SlotId);
    assert_dangling_none!(IncompatId);
    assert_dangling_none!(GroupListId);
    assert_dangling_none!(PairingRuleId);
    assert_dangling_none!(SlotPairingRuleId);
}

#[test]
#[should_panic(expected = "dangling")]
fn resolve_panics_on_a_dangling_id() {
    // An empty document: every id dangles, so `resolve` must panic rather than
    // return. (`lookup` on the same id would simply be `None`.)
    let app = AppState::<_, String>::new(Data::new());
    let params = &app.get_data().get_inner_data().params;

    let dangling = unsafe { TeacherId::new(1u64 << 40) };
    params.resolve(dangling);
}
