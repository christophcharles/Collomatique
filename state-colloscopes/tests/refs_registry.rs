//! Pin tests for the reference registry (the per-kind `*RefSite` enums /
//! [`RefVisitor`] / [`Reference`]).
//!
//! These lock the exact set and order of reference sites the walker emits for a
//! document that exercises every relationship — entity families, dense mirrors
//! and the colloscope. They are the phase-C spec: the derive-based reroute
//! (commit 3) must keep them passing.
//!
//! Three things are pinned: the full ordered `walk_refs` output (per referenced
//! kind), each `references_to_*` reverse lookup on the interesting ids, and the
//! `for_each_reference` edge stream (faithful forwarding + cross-kind order).

use collomatique_state::{AppState, traits::Manager};
use collomatique_state_colloscopes::{
    AssignmentOp, BalancingOp, ColloscopeOp, Data, GroupListOp, GroupListRefSite, IncompatOp,
    NewId, NonEmptyRangeInclusive, Op, PairingOp, PeriodOp, PeriodRefSite, RefVisitor, Reference,
    SettingsOp, SlotOp, SlotPairingOp, SlotRefSite, StudentOp, StudentRefSite, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity,
    SubjectRefSite, TeacherOp, TeacherRefSite, WeekOp, WeekPatternOp, WeekPatternRefSite,
    WeekRefSite,
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
    period: Vec<(PeriodId, PeriodRefSite)>,
    week: Vec<(WeekId, WeekRefSite)>,
    subject: Vec<(SubjectId, SubjectRefSite)>,
    teacher: Vec<(TeacherId, TeacherRefSite)>,
    student: Vec<(StudentId, StudentRefSite)>,
    week_pattern: Vec<(WeekPatternId, WeekPatternRefSite)>,
    slot: Vec<(SlotId, SlotRefSite)>,
    group_list: Vec<(GroupListId, GroupListRefSite)>,
}

impl RefVisitor for Collect {
    fn period_ref(&mut self, t: PeriodId, s: PeriodRefSite) {
        self.period.push((t, s));
    }
    fn week_ref(&mut self, t: WeekId, s: WeekRefSite) {
        self.week.push((t, s));
    }
    fn subject_ref(&mut self, t: SubjectId, s: SubjectRefSite) {
        self.subject.push((t, s));
    }
    fn teacher_ref(&mut self, t: TeacherId, s: TeacherRefSite) {
        self.teacher.push((t, s));
    }
    fn student_ref(&mut self, t: StudentId, s: StudentRefSite) {
        self.student.push((t, s));
    }
    fn week_pattern_ref(&mut self, t: WeekPatternId, s: WeekPatternRefSite) {
        self.week_pattern.push((t, s));
    }
    fn slot_ref(&mut self, t: SlotId, s: SlotRefSite) {
        self.slot.push((t, s));
    }
    fn group_list_ref(&mut self, t: GroupListId, s: GroupListRefSite) {
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
        students_per_group: NonEmptyRangeInclusive::new(
            NonZeroU32::new(1).unwrap()..=NonZeroU32::new(3).unwrap(),
        )
        .expect("statically non-empty"),
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

    // A week pattern that excludes w1 — exercises `WeekPatternExcludedWeek`
    // (pattern → week). The pattern → period edge is transitive and no longer
    // materialized as a reference site.
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

    // Convenient site aliases for the mirror/colloscope entries. Each site now
    // carries only the *complement* of its target within the referencing row.
    let assign_period_site = PeriodRefSite::AssignmentsKey { subject: math };
    let assign_subject_site = SubjectRefSite::AssignmentsKey { period: p0 };
    let assoc_period_site = PeriodRefSite::AssociationEntry { subject: math };
    let assoc_subject_site = SubjectRefSite::AssociationEntry { period: p0 };
    let assoc_group_list_site = GroupListRefSite::AssociationEntry {
        period: p0,
        subject: math,
    };
    // Row vocabulary: the single non-empty interrogation row is (slot1, w0); it
    // references both a slot and a week, each site carrying the other component.
    // slot2 and week w1 carry no row.
    let collo_int_slot_site = SlotRefSite::ColloscopeInterrogation { week: w0 };
    let collo_int_week_site = WeekRefSite::ColloscopeInterrogation { slot: slot1 };
    let collo_gl_site = GroupListRefSite::ColloscopeGroupListKey;

    assert_eq!(
        c.period,
        vec![
            // week → period FK (week_map id order: w0 then w1)
            (p0, PeriodRefSite::WeekPeriodFk(w0)),
            (p1, PeriodRefSite::WeekPeriodFk(w1)),
            // families
            (p1, PeriodRefSite::SubjectExcludedPeriods(math)),
            (p1, PeriodRefSite::StudentExcludedPeriods(st1)),
            (p1, PeriodRefSite::PairingRuleExcludedPeriods(pairing)),
            (
                p1,
                PeriodRefSite::SlotPairingRuleExcludedPeriods(slot_pairing)
            ),
            // (the pattern → period edge is transitive, not a site)
            // assignments mirror
            (p0, assign_period_site),
            // association mirror
            (p0, assoc_period_site),
            // colloscope: rows key on (slot, week), so periods are no longer
            // directly referenced by the colloscope.
        ],
    );

    // Weeks are referenced by week-pattern exclusions (step 12: wp excludes w1)
    // and by colloscope interrogation rows (step 14: the (slot1, w0) row).
    assert_eq!(
        c.week,
        vec![
            (w1, WeekRefSite::WeekPatternExcludedWeek(wp)),
            (w0, collo_int_week_site),
        ],
    );

    assert_eq!(
        c.subject,
        vec![
            // families
            (math, SubjectRefSite::TeacherSubjects(teacher)),
            (phys, SubjectRefSite::TeacherSubjects(teacher)),
            (math, SubjectRefSite::SlotSubject(slot1)),
            (math, SubjectRefSite::SlotSubject(slot2)),
            (math, SubjectRefSite::IncompatSubject(incompat)),
            // pairing: math is the antecedent, phys the consequent (distinct sites)
            (math, SubjectRefSite::PairingRuleAntecedent(pairing)),
            (phys, SubjectRefSite::PairingRuleConsequent(pairing)),
            (phys, SubjectRefSite::BalancingSubjectKey),
            // assignments mirror
            (math, assign_subject_site),
            // association mirror
            (math, assoc_subject_site),
            // (no ordering-key site: those keys mirror `SlotSubject` and are excluded)
        ],
    );

    // Teachers are never referenced by mirrors or the colloscope.
    assert_eq!(
        c.teacher,
        vec![
            (teacher, TeacherRefSite::SlotTeacher(slot1)),
            (teacher, TeacherRefSite::SlotTeacher(slot2)),
        ],
    );

    assert_eq!(
        c.student,
        vec![
            // families
            (st2, StudentRefSite::GroupListPrefilledStudent(gl_pre)),
            (st1, StudentRefSite::GroupListExcludedStudent(gl_auto)),
            (st2, StudentRefSite::SettingsStudentKey),
            // assignments mirror
            (
                st2,
                StudentRefSite::AssignmentsStudent {
                    period: p0,
                    subject: math,
                },
            ),
            // colloscope group list
            (st2, StudentRefSite::ColloscopeGroupListStudent(gl_auto)),
        ],
    );

    // Week patterns are never referenced by mirrors or the colloscope.
    assert_eq!(
        c.week_pattern,
        vec![
            (wp, WeekPatternRefSite::SlotWeekPattern(slot1)),
            (wp, WeekPatternRefSite::IncompatWeekPattern(incompat)),
        ],
    );

    assert_eq!(
        c.slot,
        vec![
            // families: slot1 is the antecedent, slot2 the consequent (distinct sites)
            (slot1, SlotRefSite::SlotPairingRuleAntecedent(slot_pairing)),
            (slot2, SlotRefSite::SlotPairingRuleConsequent(slot_pairing)),
            // colloscope: only slot1 carries an interrogation row; slot2 is empty.
            (slot1, collo_int_slot_site),
        ],
    );

    assert_eq!(
        c.group_list,
        vec![
            // association mirror
            (gl_auto, assoc_group_list_site),
            // colloscope
            (gl_auto, collo_gl_site),
        ],
    );

    // --- Reverse lookups (references_to_*) on the interesting ids ---
    let inner = app.get_data().get_inner_data();

    assert_eq!(
        inner.references_to_period(p0),
        vec![
            PeriodRefSite::WeekPeriodFk(w0),
            assign_period_site,
            assoc_period_site,
        ],
    );
    assert_eq!(
        inner.references_to_period(p1),
        vec![
            PeriodRefSite::WeekPeriodFk(w1),
            PeriodRefSite::SubjectExcludedPeriods(math),
            PeriodRefSite::StudentExcludedPeriods(st1),
            PeriodRefSite::PairingRuleExcludedPeriods(pairing),
            PeriodRefSite::SlotPairingRuleExcludedPeriods(slot_pairing),
        ],
    );

    // Weeks are now ref targets: w0 carries the interrogation row, w1 none.
    assert_eq!(inner.references_to_week(w0), vec![collo_int_week_site]);
    assert_eq!(
        inner.references_to_week(w1),
        vec![WeekRefSite::WeekPatternExcludedWeek(wp)],
    );

    assert_eq!(
        inner.references_to_subject(math),
        vec![
            SubjectRefSite::TeacherSubjects(teacher),
            SubjectRefSite::SlotSubject(slot1),
            SubjectRefSite::SlotSubject(slot2),
            SubjectRefSite::IncompatSubject(incompat),
            SubjectRefSite::PairingRuleAntecedent(pairing),
            assign_subject_site,
            assoc_subject_site,
        ],
    );
    assert_eq!(
        inner.references_to_subject(phys),
        vec![
            SubjectRefSite::TeacherSubjects(teacher),
            SubjectRefSite::PairingRuleConsequent(pairing),
            SubjectRefSite::BalancingSubjectKey,
        ],
    );

    assert_eq!(
        inner.references_to_teacher(teacher),
        vec![
            TeacherRefSite::SlotTeacher(slot1),
            TeacherRefSite::SlotTeacher(slot2),
        ],
    );

    assert_eq!(
        inner.references_to_student(st1),
        vec![StudentRefSite::GroupListExcludedStudent(gl_auto)],
    );
    assert_eq!(
        inner.references_to_student(st2),
        vec![
            StudentRefSite::GroupListPrefilledStudent(gl_pre),
            StudentRefSite::SettingsStudentKey,
            StudentRefSite::AssignmentsStudent {
                period: p0,
                subject: math,
            },
            StudentRefSite::ColloscopeGroupListStudent(gl_auto),
        ],
    );

    assert_eq!(
        inner.references_to_week_pattern(wp),
        vec![
            WeekPatternRefSite::SlotWeekPattern(slot1),
            WeekPatternRefSite::IncompatWeekPattern(incompat),
        ],
    );

    assert_eq!(
        inner.references_to_slot(slot1),
        vec![
            SlotRefSite::SlotPairingRuleAntecedent(slot_pairing),
            collo_int_slot_site,
        ],
    );
    assert_eq!(
        inner.references_to_slot(slot2),
        vec![SlotRefSite::SlotPairingRuleConsequent(slot_pairing)],
    );

    assert_eq!(
        inner.references_to_group_list(gl_auto),
        vec![assoc_group_list_site, collo_gl_site],
    );
    // The prefilled group list is never *referenced* (its students show up as
    // `GroupListPrefilledStudent` student refs, not group-list refs).
    assert_eq!(inner.references_to_group_list(gl_pre), vec![]);

    // --- Edge stream (for_each_reference) consistency with the visitor walk ---
    // The funnel must forward every callback faithfully and preserve cross-kind
    // interleaving: projecting the flat stream per kind reproduces each `Collect`
    // vector, and no edge is lost or duplicated across kinds.
    let mut flat = Vec::new();
    inner.for_each_reference(&mut |r| flat.push(r));

    let flat_period: Vec<_> = flat
        .iter()
        .filter_map(|r| match *r {
            Reference::Period { target, site } => Some((target, site)),
            _ => None,
        })
        .collect();
    let flat_week: Vec<_> = flat
        .iter()
        .filter_map(|r| match *r {
            Reference::Week { target, site } => Some((target, site)),
            _ => None,
        })
        .collect();
    let flat_subject: Vec<_> = flat
        .iter()
        .filter_map(|r| match *r {
            Reference::Subject { target, site } => Some((target, site)),
            _ => None,
        })
        .collect();
    let flat_teacher: Vec<_> = flat
        .iter()
        .filter_map(|r| match *r {
            Reference::Teacher { target, site } => Some((target, site)),
            _ => None,
        })
        .collect();
    let flat_student: Vec<_> = flat
        .iter()
        .filter_map(|r| match *r {
            Reference::Student { target, site } => Some((target, site)),
            _ => None,
        })
        .collect();
    let flat_week_pattern: Vec<_> = flat
        .iter()
        .filter_map(|r| match *r {
            Reference::WeekPattern { target, site } => Some((target, site)),
            _ => None,
        })
        .collect();
    let flat_slot: Vec<_> = flat
        .iter()
        .filter_map(|r| match *r {
            Reference::Slot { target, site } => Some((target, site)),
            _ => None,
        })
        .collect();
    let flat_group_list: Vec<_> = flat
        .iter()
        .filter_map(|r| match *r {
            Reference::GroupList { target, site } => Some((target, site)),
            _ => None,
        })
        .collect();

    assert_eq!(flat_period, c.period);
    assert_eq!(flat_week, c.week);
    assert_eq!(flat_subject, c.subject);
    assert_eq!(flat_teacher, c.teacher);
    assert_eq!(flat_student, c.student);
    assert_eq!(flat_week_pattern, c.week_pattern);
    assert_eq!(flat_slot, c.slot);
    assert_eq!(flat_group_list, c.group_list);

    assert_eq!(
        flat.len(),
        c.period.len()
            + c.week.len()
            + c.subject.len()
            + c.teacher.len()
            + c.student.len()
            + c.week_pattern.len()
            + c.slot.len()
            + c.group_list.len(),
        "for_each_reference lost or duplicated an edge across kinds",
    );
}
