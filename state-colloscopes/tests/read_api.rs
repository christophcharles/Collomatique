//! Tests for the keyed read interface (`Parameters::lookup` / `resolve`).
//!
//! One entity of every kind is built through the public op API; then each of
//! the ten `Lookup` impls is exercised: `lookup` returns the *live* borrow out
//! of its container (checked by pointer identity, so nothing is cloned or
//! rebuilt), a dangling id resolves to `None`, and `resolve` round-trips on
//! valid ids while panicking on a dangling one.
//!
//! Two further phase-C sections extend this file: the `Join`-view tests (C3b)
//! and the `all_ids` table-enumeration test (C3c).

use collomatique_state::{AppState, Join, traits::Manager};
use collomatique_state_colloscopes::{
    Data, GroupListOp, IncompatOp, JoinedRulePart, NewId, NonEmptyRangeInclusive, Op, PairingOp,
    PeriodOp, SlotOp, SlotPairingOp, StudentOp, Subject, SubjectInterrogationParameters, SubjectOp,
    SubjectParameters, SubjectPeriodicity, TeacherOp, WeekOp, WeekPatternOp,
    group_lists::GroupListParameters,
    ids::{
        GroupListId, Id, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
        SubjectId, TeacherId, WeekId, WeekPatternId,
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
        students_per_group: NonEmptyRangeInclusive::new(
            NonZeroU32::new(1).unwrap()..=NonZeroU32::new(3).unwrap(),
        )
        .expect("statically non-empty"),
        group_names: vec![None],
    }
}

/// Ids captured while building the document, one per entity kind.
struct Built {
    period: PeriodId,
    week: WeekId,
    subject: SubjectId,
    phys: SubjectId,
    teacher: TeacherId,
    student: StudentId,
    week_pattern: WeekPatternId,
    slot: SlotId,
    slot2: SlotId,
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
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "add period"
    );
    let week = apply_new!(
        Op::Week(WeekOp::AddFront(period, WeekDesc::new(true))),
        NewId::WeekId,
        "add week"
    );
    // Trivial pattern: excludes no week.
    let week_pattern = apply_new!(
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "WP".into(),
            excluded_weeks: Default::default(),
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
        week,
        subject,
        phys,
        teacher,
        student,
        week_pattern,
        slot,
        slot2,
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

    // A period resolves to the unit entity (existence only): pointer identity
    // is meaningless for a ZST, so the period pin is a value check.
    assert_eq!(params.lookup(ids.period), Some(&()));
    assert_eq!(params.resolve(ids.period), &());
    // The pointer-identity pin for the periods module lives on the week entity,
    // which is the borrowable [Week] out of the week table.
    assert_resolves_to!(ids.week, params.periods.find_week(ids.week));
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

// --- Join views (C3b) ---------------------------------------------------

#[test]
fn join_borrows_every_entity_it_resolves() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let params = &app.get_data().get_inner_data().params;

    // The slot's scalar FKs (subject, teacher) and its `Some` week pattern must
    // each resolve to the very borrow `resolve` hands back — the join copies no
    // entity, it only rewrites ids into borrows out of `params`.
    let slot = params.resolve(ids.slot);
    let joined = slot.join(params).expect("validated slot joins");
    assert!(std::ptr::eq(joined.subject, params.resolve(ids.subject)));
    assert!(std::ptr::eq(joined.teacher, params.resolve(ids.teacher)));
    let wp = joined.week_pattern.expect("slot 1 has a week pattern");
    assert!(std::ptr::eq(wp, params.resolve(ids.week_pattern)));
    // Non-FK fields appear as plain borrows of the source value.
    assert!(std::ptr::eq(joined.extra_info, &slot.extra_info));
}

#[test]
fn btree_set_fk_joins_to_id_sorted_borrows() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let params = &app.get_data().get_inner_data().params;

    // The teacher teaches {Math, Physics}; the set-of-ids FK lifts to a Vec of
    // borrows in id order (Math was created first, so it sorts first).
    let teacher = params.resolve(ids.teacher);
    let joined = teacher.join(params).expect("validated teacher joins");
    assert_eq!(joined.subjects.len(), 2);
    assert!(std::ptr::eq(
        joined.subjects[0],
        params.resolve(ids.subject)
    ));
    assert!(std::ptr::eq(joined.subjects[1], params.resolve(ids.phys)));
}

#[test]
fn option_fk_joins_both_ways() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let params = &app.get_data().get_inner_data().params;

    // Some(week_pattern) → Some(borrow).
    let with_wp = make_slot(ids.subject, ids.teacher, Some(ids.week_pattern));
    let joined = with_wp
        .join(params)
        .expect("candidate with a valid wp joins");
    assert!(std::ptr::eq(
        joined.week_pattern.expect("Some maps to Some"),
        params.resolve(ids.week_pattern)
    ));

    // None → None (the join never touches the lookup for an absent FK).
    let without_wp = make_slot(ids.subject, ids.teacher, None);
    let joined = without_wp
        .join(params)
        .expect("candidate without a wp joins");
    assert!(joined.week_pattern.is_none());
}

#[test]
fn nested_rule_part_composes_through_its_own_view() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let params = &app.get_data().get_inner_data().params;

    // A PairingRule joins by first joining each RulePart into a JoinedRulePart,
    // whose `subject` field is itself a borrow out of `params`.
    let pairing = params.resolve(ids.pairing);
    let joined = pairing.join(params).expect("validated pairing joins");
    let antecedent: &JoinedRulePart = &joined.antecedent;
    let consequent: &JoinedRulePart = &joined.consequent;
    assert!(std::ptr::eq(
        antecedent.subject,
        params.resolve(ids.subject)
    ));
    assert!(std::ptr::eq(consequent.subject, params.resolve(ids.phys)));
}

#[test]
fn dangling_fk_join_returns_the_new_id_error() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let params = &app.get_data().get_inner_data().params;

    // A candidate value never inserted into the document: its subject FK is
    // valid but its teacher FK dangles. The join is fail-fast in field order
    // (subject resolves, teacher does not), so the error names the teacher id.
    let dangling_teacher = unsafe { TeacherId::new(1u64 << 40) };
    let bogus = make_slot(ids.subject, dangling_teacher, Some(ids.week_pattern));
    assert!(matches!(
        bogus.join(params),
        Err(NewId::TeacherId(id)) if id == dangling_teacher
    ));
}

// --- Table enumeration (C3c) --------------------------------------------

#[test]
fn all_ids_lists_every_table_in_canonical_order() {
    let mut app = AppState::<_, String>::new(Data::new());
    let ids = build_document(&mut app);
    let params = &app.get_data().get_inner_data().params;

    // `all_ids` is the single declared enumeration of the tables. The order is
    // fixed: students, periods, weeks (each period's weeks, in walk order),
    // subjects (in OrderedTable order: Math then Physics), teachers, week
    // patterns, slots (id order: slot then slot2), incompats, group lists,
    // pairing rules, slot pairing rules. The document's single period holds one
    // week.
    let week = params
        .periods
        .week_ids()
        .next()
        .expect("the period has one week");
    let expected = vec![
        NewId::StudentId(ids.student),
        NewId::PeriodId(ids.period),
        NewId::from(week),
        NewId::SubjectId(ids.subject),
        NewId::SubjectId(ids.phys),
        NewId::TeacherId(ids.teacher),
        NewId::WeekPatternId(ids.week_pattern),
        NewId::SlotId(ids.slot),
        NewId::SlotId(ids.slot2),
        NewId::IncompatId(ids.incompat),
        NewId::GroupListId(ids.group_list),
        NewId::PairingRuleId(ids.pairing),
        NewId::SlotPairingRuleId(ids.slot_pairing),
    ];
    assert_eq!(params.all_ids().collect::<Vec<_>>(), expected);

    // `NewId::inner` strips the typed wrapper to the raw `u64` the numeric
    // `ids()` view (and duplicate scanning) runs on.
    assert_eq!(
        NewId::SlotId(ids.slot).inner(),
        <SlotId as Id>::inner(&ids.slot)
    );
}
