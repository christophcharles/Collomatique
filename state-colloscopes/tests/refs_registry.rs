//! Pin tests for the reference registry ([`RefSite`] / [`RefVisitor`]).
//!
//! These lock the exact set and order of reference sites the walker emits for a
//! document that exercises every entity-family relationship. They are the
//! phase-C spec: later commits (dense mirrors, colloscope, and the derive-based
//! reroute) must keep them passing.
//!
//! Commit 1 covers the [`Parameters`]-family sites only (steps 1–11 of the walk
//! order); dense-mirror and colloscope sites arrive in commit 2.

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    BalancingOp, Data, GroupListOp, IncompatOp, NewId, Op, PairingOp, PeriodOp, RefSite,
    RefVisitor, SettingsOp, SlotOp, SlotPairingOp, StudentOp, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp,
    WeekPatternOp,
    balancing::{Balancing, BalancingOptions},
    group_lists::{GroupListFilling, GroupListParameters, PrefilledGroup},
    ids::{GroupListId, PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekPatternId},
    incompats::Incompatibility,
    pairings::{PairingRule, RulePart},
    periods::WeekDesc,
    settings::{Limits, Settings},
    slot_pairings::{SlotPairingRule, SlotRulePart},
    slots::Slot,
    students::Student,
    teachers::Teacher,
    week_patterns::WeekPattern,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

/// Collects every reference emitted by the walker, one vec per referenced kind.
#[derive(Default)]
struct Collect {
    period: Vec<(PeriodId, RefSite)>,
    subject: Vec<(SubjectId, RefSite)>,
    teacher: Vec<(TeacherId, RefSite)>,
    student: Vec<(StudentId, RefSite)>,
    week_pattern: Vec<(WeekPatternId, RefSite)>,
    slot: Vec<(SlotId, RefSite)>,
    group_list: Vec<(GroupListId, RefSite)>,
}

impl RefVisitor for Collect {
    fn period_ref(&mut self, t: PeriodId, s: RefSite) {
        self.period.push((t, s));
    }
    fn subject_ref(&mut self, t: SubjectId, s: RefSite) {
        self.subject.push((t, s));
    }
    fn teacher_ref(&mut self, t: TeacherId, s: RefSite) {
        self.teacher.push((t, s));
    }
    fn student_ref(&mut self, t: StudentId, s: RefSite) {
        self.student.push((t, s));
    }
    fn week_pattern_ref(&mut self, t: WeekPatternId, s: RefSite) {
        self.week_pattern.push((t, s));
    }
    fn slot_ref(&mut self, t: SlotId, s: RefSite) {
        self.slot.push((t, s));
    }
    fn group_list_ref(&mut self, t: GroupListId, s: RefSite) {
        self.group_list.push((t, s));
    }
}

/// Builds a subject that has interrogations (so it can host slots, incompats,
/// balancing overrides…).
fn interrogation_subject(name: &str, excluded_periods: BTreeSet<PeriodId>) -> Subject {
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
        excluded_periods,
    }
}

fn slot(subject_id: SubjectId, teacher_id: TeacherId, week_pattern: Option<WeekPatternId>) -> Slot {
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

/// A one-group group list (so a prefilled filling needs exactly one group).
fn one_group_params(name: &str) -> GroupListParameters {
    GroupListParameters {
        name: name.into(),
        students_per_group: NonZeroU32::new(1).unwrap()..=NonZeroU32::new(3).unwrap(),
        group_names: vec![None],
    }
}

#[test]
fn walk_covers_every_family_site_in_order() {
    let mut app = AppState::<_, String>::new(Data::new());

    macro_rules! apply_new {
        ($op:expr, $variant:path, $msg:expr) => {{
            let Ok(Some($variant(id))) = app.apply($op, $msg.into()) else {
                panic!(concat!("unexpected result: ", $msg));
            };
            id
        }};
    }
    macro_rules! apply_none {
        ($op:expr, $msg:expr) => {{
            let Ok(None) = app.apply($op, $msg.into()) else {
                panic!(concat!("unexpected result: ", $msg));
            };
        }};
    }

    // Two one-week periods.
    let p0 = apply_new!(
        Op::Period(PeriodOp::AddFront(vec![WeekDesc::new(true)])),
        NewId::PeriodId,
        "add period 0"
    );
    let p1 = apply_new!(
        Op::Period(PeriodOp::AddAfter(p0, vec![WeekDesc::new(true)])),
        NewId::PeriodId,
        "add period 1"
    );

    // A week pattern that is trivial on p0 (week kept) but non-trivial on p1
    // (week dropped) — exercises both `WeekPatternLengthCoupling` polarities.
    let wp = apply_new!(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "WP".into(),
            weeks: vec![true, false],
        })),
        NewId::WeekPatternId,
        "add week pattern"
    );

    // Math excludes p1; Physics runs everywhere.
    let math = apply_new!(
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::from([p1])),
        )),
        NewId::SubjectId,
        "add math"
    );
    let phys = apply_new!(
        Op::Subject(SubjectOp::AddAfter(
            Some(math),
            interrogation_subject("Physics", BTreeSet::new()),
        )),
        NewId::SubjectId,
        "add physics"
    );

    // One teacher teaching both subjects.
    let teacher = apply_new!(
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([math, phys]),
        })),
        NewId::TeacherId,
        "add teacher"
    );

    // Two slots in Math (needed for the slot pairing); slot1 carries a week pattern.
    let slot1 = apply_new!(
        Op::Slot(SlotOp::AddAfter(None, slot(math, teacher, Some(wp)))),
        NewId::SlotId,
        "add slot 1"
    );
    let slot2 = apply_new!(
        Op::Slot(SlotOp::AddAfter(Some(slot1), slot(math, teacher, None))),
        NewId::SlotId,
        "add slot 2"
    );

    // Two students: st1 excludes p1, st2 is used by prefilled group + settings.
    let st1 = apply_new!(
        Op::Student(StudentOp::Add(Student {
            desc: Default::default(),
            excluded_periods: BTreeSet::from([p1]),
        })),
        NewId::StudentId,
        "add student 1"
    );
    let st2 = apply_new!(
        Op::Student(StudentOp::Add(Student::default())),
        NewId::StudentId,
        "add student 2"
    );

    // Incompat on Math with a week pattern.
    let incompat = apply_new!(
        Op::Incompat(IncompatOp::Add(Incompatibility {
            subject_id: math,
            name: "Inc".into(),
            slots: vec![],
            minimum_free_slots: NonZeroU32::new(1).unwrap(),
            week_pattern_id: Some(wp),
        })),
        NewId::IncompatId,
        "add incompat"
    );

    // Pairing rule Math => Physics, excluded on p1.
    let pairing = apply_new!(
        Op::Pairing(PairingOp::Add(PairingRule {
            antecedent: RulePart {
                subject_id: math,
                should_have: true,
            },
            consequent: RulePart {
                subject_id: phys,
                should_have: true,
            },
            excluded_periods: BTreeSet::from([p1]),
            soft: false,
        })),
        NewId::PairingRuleId,
        "add pairing"
    );

    // Slot pairing rule slot1 => slot2 (same subject), excluded on p1.
    let slot_pairing = apply_new!(
        Op::SlotPairing(SlotPairingOp::Add(SlotPairingRule {
            antecedent: SlotRulePart {
                slot_id: slot1,
                should_have: true,
            },
            consequent: SlotRulePart {
                slot_id: slot2,
                should_have: true,
            },
            excluded_periods: BTreeSet::from([p1]),
            soft: false,
        })),
        NewId::SlotPairingRuleId,
        "add slot pairing"
    );

    // Prefilled group list referencing st2.
    let gl_pre = apply_new!(
        Op::GroupList(GroupListOp::Add(one_group_params("Prefilled"))),
        NewId::GroupListId,
        "add prefilled group list"
    );
    apply_none!(
        Op::GroupList(GroupListOp::SetFilling(
            gl_pre,
            GroupListFilling::Prefilled {
                groups: vec![PrefilledGroup {
                    students: BTreeSet::from([st2]),
                }],
            },
        )),
        "prefill group list"
    );

    // Automatic group list excluding st1.
    let gl_auto = apply_new!(
        Op::GroupList(GroupListOp::Add(one_group_params("Automatic"))),
        NewId::GroupListId,
        "add automatic group list"
    );
    apply_none!(
        Op::GroupList(GroupListOp::SetFilling(
            gl_auto,
            GroupListFilling::Automatic {
                excluded_students: BTreeSet::from([st1]),
            },
        )),
        "set automatic exclusion"
    );

    // Per-student settings entry for st2.
    apply_none!(
        Op::Settings(SettingsOp::Update(Settings {
            global: Limits::default(),
            students: BTreeMap::from([(st2, Limits::default())]).into(),
        })),
        "add per-student settings"
    );

    // Per-subject balancing override for Physics.
    apply_none!(
        Op::Balancing(BalancingOp::Update(Balancing {
            global: BalancingOptions::default(),
            subjects: BTreeMap::from([(phys, BalancingOptions::default())]).into(),
        })),
        "add per-subject balancing"
    );

    // Walk and collect.
    let mut c = Collect::default();
    let params = &app.get_data().get_inner_data().params;
    collomatique_state_colloscopes::refs::walk_params_refs_for_tests(params, &mut c);

    assert_eq!(
        c.period,
        vec![
            (p1, RefSite::SubjectExcludedPeriods(math)),
            (p1, RefSite::StudentExcludedPeriods(st1)),
            (p1, RefSite::PairingRuleExcludedPeriods(pairing)),
            (p1, RefSite::SlotPairingRuleExcludedPeriods(slot_pairing)),
            (
                p0,
                RefSite::WeekPatternLengthCoupling {
                    week_pattern: wp,
                    non_trivial: false,
                },
            ),
            (
                p1,
                RefSite::WeekPatternLengthCoupling {
                    week_pattern: wp,
                    non_trivial: true,
                },
            ),
        ],
    );

    assert_eq!(
        c.subject,
        vec![
            (math, RefSite::TeacherSubjects(teacher)),
            (phys, RefSite::TeacherSubjects(teacher)),
            (math, RefSite::SlotSubject(slot1)),
            (math, RefSite::SlotSubject(slot2)),
            (math, RefSite::IncompatSubject(incompat)),
            (math, RefSite::PairingRulePart(pairing)),
            (phys, RefSite::PairingRulePart(pairing)),
            (phys, RefSite::BalancingSubjectKey),
        ],
    );

    assert_eq!(
        c.teacher,
        vec![
            (teacher, RefSite::SlotTeacher(slot1)),
            (teacher, RefSite::SlotTeacher(slot2)),
        ],
    );

    assert_eq!(
        c.student,
        vec![
            (st2, RefSite::GroupListPrefilledStudent(gl_pre)),
            (st1, RefSite::GroupListExcludedStudent(gl_auto)),
            (st2, RefSite::SettingsStudentKey),
        ],
    );

    assert_eq!(
        c.week_pattern,
        vec![
            (wp, RefSite::SlotWeekPattern(slot1)),
            (wp, RefSite::IncompatWeekPattern(incompat)),
        ],
    );

    assert_eq!(
        c.slot,
        vec![
            (slot1, RefSite::SlotPairingRulePart(slot_pairing)),
            (slot2, RefSite::SlotPairingRulePart(slot_pairing)),
        ],
    );

    // No group-list references are emitted by family walkers (associations and
    // colloscope sites arrive in commit 2).
    assert_eq!(c.group_list, vec![]);
}
