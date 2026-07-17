//! Pin tests for the reference registry ([`RefSite`] / [`RefVisitor`]).
//!
//! These lock the exact set and order of reference sites the walker emits for a
//! document that exercises every relationship — entity families, dense mirrors
//! and the colloscope. They are the phase-C spec: the derive-based reroute
//! (commit 3) must keep them passing.
//!
//! Two things are pinned: the full ordered `walk_refs` output (per referenced
//! kind), and each `references_to_*` reverse lookup on the interesting ids.

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    AssignmentOp, BalancingOp, ColloscopeOp, Data, GroupListOp, IncompatOp, NewId, Op, PairingOp,
    PeriodOp, RefSite, RefVisitor, SettingsOp, SlotOp, SlotPairingOp, StudentOp, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity, TeacherOp,
    WeekOp, WeekPatternOp,
    balancing::{Balancing, BalancingOptions},
    group_lists::{GroupListFilling, GroupListParameters, PrefilledGroup},
    ids::{GroupListId, PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekId, WeekPatternId},
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
    week: Vec<(WeekId, RefSite)>,
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
    fn week_ref(&mut self, t: WeekId, s: RefSite) {
        self.week.push((t, s));
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
fn walk_covers_every_site_in_order() {
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
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "add period 0"
    );
    let w0 = apply_new!(
        Op::Week(WeekOp::AddFront(p0, WeekDesc::new(true))),
        NewId::WeekId,
        "add week to period 0"
    );
    let p1 = apply_new!(
        Op::Period(PeriodOp::AddAfter(p0)),
        NewId::PeriodId,
        "add period 1"
    );
    let w1 = apply_new!(
        Op::Week(WeekOp::AddFront(p1, WeekDesc::new(true))),
        NewId::WeekId,
        "add week to period 1"
    );

    // A week pattern that is trivial on p0 (week kept) but non-trivial on p1
    // (week w1 excluded) — exercises both `WeekPatternLengthCoupling` polarities.
    let wp = apply_new!(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "WP".into(),
            excluded_weeks: std::collections::BTreeSet::from([w1]),
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

    // --- Mirror and colloscope content ---

    // Assign st2 to Math on p0 → the (p0, Math) assignment row appears. Under
    // sparse assignments no other rows exist (nobody else is assigned), so the
    // walker emits exactly this one assignment mirror site.
    apply_none!(
        Op::Assignment(AssignmentOp::Assign(p0, st2, math, true)),
        "assign st2 to math on p0"
    );

    // Associate the automatic group list with Math on p0 (must come before the
    // interrogation so its group number 0 validates against a one-group list).
    apply_none!(
        Op::GroupList(GroupListOp::AssignToSubject(p0, math, Some(gl_auto))),
        "associate gl_auto to math on p0"
    );

    // Place st2 in the colloscope's automatic group list → the list is non-empty.
    apply_none!(
        Op::Colloscope(ColloscopeOp::SetGroupList(
            gl_auto,
            BTreeMap::from([(st2, 0)]),
        )),
        "fill colloscope group list"
    );

    // Assign group 0 to slot1's interrogation on p0 week 0 → that period and slot
    // become non-trivial in the colloscope; p1 and slot2 stay empty.
    apply_none!(
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot1,
            w0,
            BTreeSet::from([0]),
        )),
        "assign group to interrogation"
    );

    // Walk and collect.
    let mut c = Collect::default();
    app.get_data().get_inner_data().walk_refs(&mut c);

    // Convenient site aliases for the mirror/colloscope entries.
    let assign_p0_math = RefSite::AssignmentsKey {
        period: p0,
        subject: math,
        non_trivial: true,
    };
    let association = RefSite::AssociationEntry {
        period: p0,
        subject: math,
        group_list: gl_auto,
    };
    // Row vocabulary: the single non-empty interrogation row is (slot1, w0); it
    // references both a slot and a week. slot2 and week w1 carry no row.
    let collo_int = RefSite::ColloscopeInterrogation {
        slot: slot1,
        week: w0,
    };
    let collo_gl = RefSite::ColloscopeGroupListKey {
        group_list: gl_auto,
    };

    assert_eq!(
        c.period,
        vec![
            // week → period FK (week_map id order: w0 then w1)
            (p0, RefSite::WeekPeriodFk(w0)),
            (p1, RefSite::WeekPeriodFk(w1)),
            // families
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
            // assignments mirror
            (p0, assign_p0_math),
            // association mirror
            (p0, association),
            // colloscope: rows key on (slot, week), so periods are no longer
            // directly referenced by the colloscope.
        ],
    );

    // Weeks are referenced by week-pattern exclusions (step 12: wp excludes w1)
    // and by colloscope interrogation rows (step 15: the (slot1, w0) row).
    assert_eq!(
        c.week,
        vec![(w1, RefSite::WeekPatternExcludedWeek(wp)), (w0, collo_int),],
    );

    assert_eq!(
        c.subject,
        vec![
            // families
            (math, RefSite::TeacherSubjects(teacher)),
            (phys, RefSite::TeacherSubjects(teacher)),
            (math, RefSite::SlotSubject(slot1)),
            (math, RefSite::SlotSubject(slot2)),
            (math, RefSite::IncompatSubject(incompat)),
            (math, RefSite::PairingRulePart(pairing)),
            (phys, RefSite::PairingRulePart(pairing)),
            (phys, RefSite::BalancingSubjectKey),
            // assignments mirror
            (math, assign_p0_math),
            // association mirror
            (math, association),
            // ordering keys mirror (sparse: phys has interrogations but no
            // slots, so it has no ordering row and no site here)
            (math, RefSite::SlotsOrderingKey { non_trivial: true }),
        ],
    );

    // Teachers are never referenced by mirrors or the colloscope.
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
            // families
            (st2, RefSite::GroupListPrefilledStudent(gl_pre)),
            (st1, RefSite::GroupListExcludedStudent(gl_auto)),
            (st2, RefSite::SettingsStudentKey),
            // assignments mirror
            (
                st2,
                RefSite::AssignmentsStudent {
                    period: p0,
                    subject: math,
                },
            ),
            // colloscope group list
            (st2, RefSite::ColloscopeGroupListStudent(gl_auto)),
        ],
    );

    // Week patterns are never referenced by mirrors or the colloscope.
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
            // families
            (slot1, RefSite::SlotPairingRulePart(slot_pairing)),
            (slot2, RefSite::SlotPairingRulePart(slot_pairing)),
            // colloscope: only slot1 carries an interrogation row; slot2 is empty.
            (slot1, collo_int),
        ],
    );

    assert_eq!(
        c.group_list,
        vec![
            // association mirror
            (gl_auto, association),
            // colloscope
            (gl_auto, collo_gl),
        ],
    );

    // --- Reverse lookups (references_to_*) on the interesting ids ---
    let inner = app.get_data().get_inner_data();

    assert_eq!(
        inner.references_to_period(p0),
        vec![
            RefSite::WeekPeriodFk(w0),
            RefSite::WeekPatternLengthCoupling {
                week_pattern: wp,
                non_trivial: false,
            },
            assign_p0_math,
            association,
        ],
    );
    assert_eq!(
        inner.references_to_period(p1),
        vec![
            RefSite::WeekPeriodFk(w1),
            RefSite::SubjectExcludedPeriods(math),
            RefSite::StudentExcludedPeriods(st1),
            RefSite::PairingRuleExcludedPeriods(pairing),
            RefSite::SlotPairingRuleExcludedPeriods(slot_pairing),
            RefSite::WeekPatternLengthCoupling {
                week_pattern: wp,
                non_trivial: true,
            },
        ],
    );

    // Weeks are now ref targets: w0 carries the interrogation row, w1 none.
    assert_eq!(inner.references_to_week(w0), vec![collo_int]);
    assert_eq!(
        inner.references_to_week(w1),
        vec![RefSite::WeekPatternExcludedWeek(wp)],
    );

    assert_eq!(
        inner.references_to_subject(math),
        vec![
            RefSite::TeacherSubjects(teacher),
            RefSite::SlotSubject(slot1),
            RefSite::SlotSubject(slot2),
            RefSite::IncompatSubject(incompat),
            RefSite::PairingRulePart(pairing),
            assign_p0_math,
            association,
            RefSite::SlotsOrderingKey { non_trivial: true },
        ],
    );
    assert_eq!(
        inner.references_to_subject(phys),
        vec![
            RefSite::TeacherSubjects(teacher),
            RefSite::PairingRulePart(pairing),
            RefSite::BalancingSubjectKey,
            // Sparse ordering: phys has no slots, hence no SlotsOrderingKey site.
        ],
    );

    assert_eq!(
        inner.references_to_teacher(teacher),
        vec![RefSite::SlotTeacher(slot1), RefSite::SlotTeacher(slot2),],
    );

    assert_eq!(
        inner.references_to_student(st1),
        vec![RefSite::GroupListExcludedStudent(gl_auto)],
    );
    assert_eq!(
        inner.references_to_student(st2),
        vec![
            RefSite::GroupListPrefilledStudent(gl_pre),
            RefSite::SettingsStudentKey,
            RefSite::AssignmentsStudent {
                period: p0,
                subject: math,
            },
            RefSite::ColloscopeGroupListStudent(gl_auto),
        ],
    );

    assert_eq!(
        inner.references_to_week_pattern(wp),
        vec![
            RefSite::SlotWeekPattern(slot1),
            RefSite::IncompatWeekPattern(incompat),
        ],
    );

    assert_eq!(
        inner.references_to_slot(slot1),
        vec![RefSite::SlotPairingRulePart(slot_pairing), collo_int],
    );
    assert_eq!(
        inner.references_to_slot(slot2),
        vec![RefSite::SlotPairingRulePart(slot_pairing)],
    );

    assert_eq!(
        inner.references_to_group_list(gl_auto),
        vec![association, collo_gl],
    );
    // The prefilled group list is never *referenced* (its students show up as
    // `GroupListPrefilledStudent` student refs, not group-list refs).
    assert_eq!(inner.references_to_group_list(gl_pre), vec![]);
}
