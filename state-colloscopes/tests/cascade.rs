//! Colloscope cascade fixtures (step-6 commits 7 and 7.6, plan §9 and §9ter).
//!
//! Each fixture builds a document through the public op surface
//! (`AppState` + `Manager::apply`), then takes `app.get_data().clone()`,
//! annotates the target op itself through `Data::annotate`, and drives
//! [collomatique_state::apply_cascade] on that [Data] directly. What is under
//! test is the resolution map (`src/resolution.rs`) driven by the real engine,
//! not the `AppState` surface.
//!
//! What a fixture reads is the cascade's [collomatique_state::CascadeReceipt]:
//! the fixes it had to apply, each as the [Fix] value the map answered, with
//! the target held apart. So the expected lists here are `Fix` values, not ops
//! — the meaning of each repair, which is what a consumer sees. That a `Fix`
//! translates to the right op is pinned once and for all by the attribution
//! tests in `src/resolution/attribution_tests.rs`, so it is not re-asserted per
//! fixture; and the target is not a fix, so it never appears in these lists —
//! its landing is what the `Ok` means.
//!
//! Two rules govern the assertions in this file.
//!
//! **The expected fix list is derived on paper first.** Every fixture asserts
//! something about the fixes that landed, and that list comes from the plan's
//! §8.1 / §8.2 arm tables *before* the test is ever run. A difference between
//! the hand-derived list and what the engine produced is a finding to explain —
//! possibly a map bug — never a value to paste back in.
//!
//! **Sequence versus content.** Asserting the *order* of the landed fixes is only
//! meaningful where the engine actually made a choice, i.e. where one failing
//! apply reported more than one broken invariant and the engine picked
//! `set.first()` out of the `BTreeSet`. Fixture `1a` is the one that pins that
//! canonical pick order; every later fixture asserts content (length plus
//! `contains`) and is deliberately blind to order.
//!
//! The `fixture_*` tests are commit 7's (plan §9): the cascade repairs the
//! document and the target lands, so they all assert `Ok`. The `rejection_*`
//! tests at the end are commit 7.6's (plan §9ter): the op's payload is bad on
//! its own terms, no state fix can help, and the target is convicted — so they
//! assert `Err`, plus the document unchanged. They read the engine's
//! `None if is_target` branch (`state/src/cascade.rs:114-119`), which is the
//! production-visible half of the design doc's frame point 5: if an arm
//! answered `Some` there, the cascade would quietly repair the state and the
//! user would be told an edit succeeded that was in fact refused.

use collomatique_state::{AppState, CascadeReceipt, InMemoryData, apply_cascade, traits::Manager};
use collomatique_state_colloscopes::{
    AssignmentOp, BalancingOp, ColloscopeOp, Convergence, Data, Error, Fix, FixableInvariant,
    GroupListOp, IncompatOp, NewId, NonEmptyRangeInclusive, Op, PairingOp, PeriodOp, Reference,
    SettingsOp, SlotOp, SlotPairingOp, StudentOp, StudentRefSite, Subject,
    SubjectInterrogationParameters, SubjectOp, SubjectParameters, SubjectPeriodicity,
    SubjectRefSite, TeacherOp, TeacherRefSite, WeekOp, WeekPatternOp,
    balancing::BalancingOptions,
    group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup},
    ids::{
        GroupListId, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
        SubjectId, TeacherId, WeekId, WeekPatternId,
    },
    incompats::Incompatibility,
    ops::{AnnotatedOp, AnnotatedSlotOp},
    pairings::{PairingRule, RulePart},
    settings::Limits,
    slot_pairings::{SlotPairingRule, SlotRulePart},
    slots::Slot,
    soft_param::SoftParam,
    students::Student,
    teachers::Teacher,
    week_patterns::WeekPattern,
    weeks::WeekDesc,
};
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;

/// Applies an op that must create a fresh entity, and returns its new id.
macro_rules! apply_new {
    ($app:expr, $op:expr, $variant:path, $what:expr) => {
        match $app.apply($op, $what.into()) {
            Ok(Some($variant(id))) => id,
            other => panic!("{} should return a fresh id, got {:?}", $what, other),
        }
    };
}

/// Applies an op that creates nothing.
fn apply_ok(app: &mut AppState<Data, String>, op: Op, what: &str) {
    if let Err(e) = app.apply(op, what.into()) {
        panic!("{what} should apply, got {e:?}");
    }
}

/// A subject with no interrogations, excluding exactly `excluded`.
///
/// A subject without interrogations cannot host a slot, an association or a
/// balancing entry, so it stays inert in a document: it is the cheapest way to
/// own a `SubjectExcludedPeriods` reference and nothing else.
fn plain_subject(name: &str, excluded: BTreeSet<PeriodId>) -> Subject {
    Subject {
        parameters: SubjectParameters {
            name: name.into(),
            interrogation_parameters: None,
        },
        excluded_periods: excluded,
    }
}

/// A subject that runs interrogations (so it can host slots, an association and
/// colloscope cells), excluding exactly `excluded`.
///
/// Its interrogations last an hour. The rejection fixtures need to vary that,
/// so the body lives in [interrogation_subject_lasting].
fn interrogation_subject(name: &str, excluded: BTreeSet<PeriodId>) -> Subject {
    interrogation_subject_lasting(name, excluded, 60)
}

/// [interrogation_subject], with the interrogation duration spelled out.
fn interrogation_subject_lasting(
    name: &str,
    excluded: BTreeSet<PeriodId>,
    minutes: u32,
) -> Subject {
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
                duration: collomatique_time::NonZeroMinutes::new(minutes).unwrap(),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: NonZeroU32::new(2).unwrap(),
                },
            }),
        },
        excluded_periods: excluded,
    }
}

/// A student with default identity, excluding exactly `excluded`.
fn plain_student(excluded: BTreeSet<PeriodId>) -> Student {
    Student {
        desc: Default::default(),
        excluded_periods: excluded,
    }
}

/// A slot starting at `hour:00`, well clear of the end of the day (the
/// subjects above last an hour, so `SlotOverflowsDay` never fires).
fn make_slot(
    subject_id: SubjectId,
    teacher_id: TeacherId,
    week_pattern: Option<WeekPatternId>,
    hour: u32,
) -> Slot {
    make_slot_at(subject_id, teacher_id, week_pattern, hour, 0)
}

/// [make_slot], to the minute.
///
/// Only the `SlotOverflowsDay` fixtures need this: every other fixture in the
/// file wants a slot that is merely somewhere in the day, and says so with an
/// hour alone.
fn make_slot_at(
    subject_id: SubjectId,
    teacher_id: TeacherId,
    week_pattern: Option<WeekPatternId>,
    hour: u32,
    minute: u32,
) -> Slot {
    Slot {
        subject_id,
        teacher_id,
        start_time: collomatique_time::SlotStart {
            weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
            start_time: collomatique_time::WholeMinuteTime::new(
                chrono::NaiveTime::from_hms_opt(hour, minute, 0).unwrap(),
            )
            .unwrap(),
        },
        extra_info: String::new(),
        week_pattern,
        cost: 0,
    }
}

fn pairing_rule(
    antecedent: SubjectId,
    consequent: SubjectId,
    excluded_periods: BTreeSet<PeriodId>,
) -> PairingRule {
    PairingRule::new(
        RulePart {
            subject_id: antecedent,
            should_have: true,
        },
        RulePart {
            subject_id: consequent,
            should_have: true,
        },
        excluded_periods,
        false,
    )
    .expect("the two parts name distinct subjects")
}

fn slot_pairing_rule(
    antecedent: SlotId,
    consequent: SlotId,
    excluded_periods: BTreeSet<PeriodId>,
) -> SlotPairingRule {
    SlotPairingRule::new(
        SlotRulePart {
            slot_id: antecedent,
            should_have: true,
        },
        SlotRulePart {
            slot_id: consequent,
            should_have: true,
        },
        excluded_periods,
        false,
    )
    .expect("the two parts name distinct slots")
}

/// An automatically-filled group list with `groups` unnamed groups.
fn automatic_group_list(
    name: &str,
    groups: usize,
    excluded_students: BTreeSet<StudentId>,
) -> GroupList {
    GroupList::new(
        GroupListParameters {
            name: name.into(),
            group_names: vec![None; groups],
            ..Default::default()
        },
        GroupListFilling::Automatic { excluded_students },
    )
    .expect("`GroupList::new` validates the prefilled branch only")
}

/// A prefilled group list holding exactly `groups`, one `group_names` entry per
/// group (the count match is `GroupList::new`'s first value-internal invariant).
fn prefilled_group_list(name: &str, groups: Vec<BTreeSet<StudentId>>) -> GroupList {
    GroupList::new(
        GroupListParameters {
            name: name.into(),
            group_names: vec![None; groups.len()],
            ..Default::default()
        },
        GroupListFilling::Prefilled {
            groups: groups
                .into_iter()
                .map(|students| PrefilledGroup { students })
                .collect(),
        },
    )
    .expect("the group count matches and no student sits in two groups")
}

/// The meaning of every landed fix, in order. The target is not a fix, so it
/// is not here.
fn landed_fixes(receipt: &CascadeReceipt<Data>) -> Vec<Fix> {
    receipt
        .fixes()
        .iter()
        .map(|(_op, fix)| fix.clone())
        .collect()
}

/// Content, not sequence: the same fixes landed, in any order.
///
/// Length plus `contains` catches an extra, a missing and a wrong fix. The one
/// case it misses — a duplicate paired with an omission — cannot occur, since
/// the same fix landing twice would be a perfect no-op and the engine panics on
/// that.
fn assert_same_fixes(actual: &[Fix], expected: &[Fix]) {
    for fix in expected {
        assert!(
            actual.contains(fix),
            "expected fix never landed: {fix:#?}\nlanded: {actual:#?}"
        );
    }
    assert_eq!(
        actual.len(),
        expected.len(),
        "landed fix count\nlanded: {actual:#?}"
    );
}

/// The document holds no broken invariant at all — checked by calling the
/// checker directly rather than by inferring it from the cascade's success.
fn assert_clean(data: &Data) {
    assert_eq!(
        data.get_inner_data().broken_invariants(),
        Ok(BTreeSet::new()),
        "the final state must be fully valid"
    );
}

/// The target was convicted, of exactly `expected` and nothing else.
///
/// The set is compared whole rather than with a `contains`, per this file's
/// first rule: an extra break would mean the fixture's document is not the one
/// its doc comment describes, and that is worth failing on.
fn assert_convicted_of(err: Error, expected: BTreeSet<FixableInvariant>, why: &str) {
    match err {
        Error::BrokenInvariants(set) => assert_eq!(set, expected, "{why}"),
        other => panic!("expected BrokenInvariants, got {other:?}"),
    }
}

/// Fixture `1a` — **order**. The minimal document in which the engine genuinely
/// *chooses*.
///
/// A period `P` excluded by one subject and by one student, and referenced by
/// nothing else. `PeriodOp::Remove(P)` therefore fails its first apply with
/// **two** broken invariants at once — `DanglingFk(Period { P,
/// SubjectExcludedPeriods(S) })` and `DanglingFk(Period { P,
/// StudentExcludedPeriods(St) })` — whose fixes are two *different* ops.
///
/// The expected sequence, derived from the tables and not from a run: both
/// breaks are `Reference::Period` on the same target, so the `BTreeSet` order
/// is decided by `PeriodRefSite`'s declaration order, where
/// `SubjectExcludedPeriods` precedes `StudentExcludedPeriods` (`refs.rs`). So
/// round 1 picks the subject site and lands `Subject(Update(S, minus P))`;
/// round 2 retries the target, now sees only the student break, and lands
/// `Student(Update(St, minus P))`; round 3 retries the target and it lands.
///
/// This fixture, and only this one, pins that canonical pick order — it is the
/// tripwire on `FixableInvariant`'s derived `Ord`. A reorder of the enums, or a
/// new variant inserted in the middle, changes the picks and fails here, on two
/// ops, where the diff is readable at a glance.
#[test]
fn fixture_1a_two_simultaneous_breaks_are_fixed_in_canonical_order() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let period: PeriodId = match app.apply(Op::Period(PeriodOp::AddFront), "add period".into()) {
        Ok(Some(NewId::PeriodId(id))) => id,
        other => panic!("adding a period should return a period id, got {other:?}"),
    };
    let excluding_period = BTreeSet::from([period]);

    let subject: SubjectId = match app.apply(
        Op::Subject(SubjectOp::AddAfter(
            None,
            plain_subject("Math", excluding_period.clone()),
        )),
        "add subject".into(),
    ) {
        Ok(Some(NewId::SubjectId(id))) => id,
        other => panic!("adding a subject should return a subject id, got {other:?}"),
    };
    let student: StudentId = match app.apply(
        Op::Student(StudentOp::Add(plain_student(excluding_period.clone()))),
        "add student".into(),
    ) {
        Ok(Some(NewId::StudentId(id))) => id,
        other => panic!("adding a student should return a student id, got {other:?}"),
    };

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::Period(PeriodOp::Remove(period)));

    let receipt = apply_cascade(&mut data, target).expect("the cascade resolves both breaks");

    assert_eq!(
        landed_fixes(&receipt),
        vec![
            Fix::RemoveSubjectPeriodExclusion {
                subject,
                period,
                rebuilt: plain_subject("Math", BTreeSet::new()),
            },
            Fix::RemoveStudentPeriodExclusion {
                student,
                period,
                rebuilt: plain_student(BTreeSet::new()),
            },
        ],
    );

    let params = &data.get_inner_data().params;
    assert!(
        params.periods.find_period_position(period).is_none(),
        "the target period is gone"
    );
    assert_eq!(
        params.subjects.find_subject(subject),
        Some(&plain_subject("Math", BTreeSet::new())),
        "the subject survives, minus the removed period"
    );
    assert_eq!(
        params.students.student_map.get(&student),
        Some(&plain_student(BTreeSet::new())),
        "the student survives, minus the removed period"
    );
}

/// Fixture `1b` — **depth**. A fix of a fix of the target.
///
/// A period `P` with exactly one week `w`, one slot carrying one colloscope
/// cell on `w`, and no week pattern excluding `w`. The chain the fixture is
/// for:
///
/// - round 1, `PeriodOp::Remove(P)` dangles `Week::period_id` → fix
///   `Week(Remove(w))`;
/// - round 2, that fix itself fails: the colloscope row keyed `(slot, w)`
///   dangles on the week → fix `Colloscope(SetInterrogation(slot, w, ∅))`;
/// - round 3 onward, the clear lands, the week removal is retried and lands,
///   the target is retried and lands.
///
/// That is depth three — a fix of a fix of the target — which no engine test
/// reaches (the toy tests in `cascade.rs` stop at depth two).
///
/// **One deviation from the plan's §9 description, found while deriving the
/// expected list.** The plan asks for a document where every round reports
/// exactly one break. That is not constructible: a colloscope cell must be
/// non-empty (an empty one is canonical-absent), and every group number in it
/// is checked against the group count of the group list associated to
/// `(period of the week, subject of the slot)` — with no association the bound
/// is `0` and *any* group number is out of bounds. So the cell forces an
/// association on `(P, subject)`, which is itself a seventh period reference
/// site. Round 1 therefore reports two breaks, not one, and the cascade lands
/// four ops rather than three. The extra break changes nothing about the depth
/// chain: `PeriodRefSite::WeekPeriodFk` is declared before
/// `PeriodRefSite::AssociationEntry`, so the week is still picked first, and
/// the association is cleared only once the chain has run to completion.
#[test]
fn fixture_1b_a_fix_of_a_fix_of_the_target_lands_in_order() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let period: PeriodId = apply_new!(
        app,
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "adding a period"
    );
    let week: WeekId = apply_new!(
        app,
        Op::Week(WeekOp::AddFront(period, WeekDesc::default())),
        NewId::WeekId,
        "adding a week"
    );
    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding a subject"
    );
    let teacher: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "adding a teacher"
    );
    let slot: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(None, make_slot(subject, teacher, None, 8))),
        NewId::SlotId,
        "adding a slot"
    );
    // Forced by the group-number bound: without an association the bound is 0
    // and the cell below cannot be filled at all (see the note above).
    let group_list: GroupListId = apply_new!(
        app,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Liste",
            2,
            BTreeSet::new()
        ))),
        NewId::GroupListId,
        "adding a group list"
    );
    apply_ok(
        &mut app,
        Op::GroupList(GroupListOp::AssignToSubject(
            period,
            subject,
            Some(group_list),
        )),
        "associating the group list",
    );
    apply_ok(
        &mut app,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot,
            week,
            BTreeSet::from([0]),
        )),
        "filling the colloscope cell",
    );

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::Period(PeriodOp::Remove(period)));

    let receipt = apply_cascade(&mut data, target).expect("the cascade resolves the chain");

    assert_eq!(
        landed_fixes(&receipt),
        vec![
            Fix::ClearInterrogationCell { slot, week },
            Fix::DeleteWeek { week },
            Fix::UnassignGroupList { period, subject },
        ],
    );

    assert_clean(&data);
    let inner = data.get_inner_data();
    assert!(
        inner.params.periods.find_period_position(period).is_none(),
        "the target period is gone"
    );
    assert!(
        inner.params.weeks.find_week(week).is_none(),
        "its only week went with it"
    );
    assert!(
        inner.colloscope.interrogation(slot, week).is_none(),
        "and the cell that hung off that week"
    );
    assert!(
        inner.params.slots.find_slot(slot).is_some(),
        "the slot itself is innocent and survives"
    );
    assert!(
        inner
            .params
            .group_lists
            .group_list_map
            .get(&group_list)
            .is_some(),
        "unassigning leaves the group list in place"
    );
}

/// The seven-site period document shared by fixtures `1c` and `1e`.
struct PeriodDocument {
    period: PeriodId,
    /// One week for `1c`, two for `1e` (the first carries the colloscope cell).
    weeks: Vec<WeekId>,
    /// Runs interrogations and runs on the period: it owns the slots, the
    /// assignments row and the association.
    subject: SubjectId,
    /// Excludes the period — the `SubjectExcludedPeriods` site.
    excluded_subject: SubjectId,
    slots: Vec<SlotId>,
    pairing: PairingRuleId,
    slot_pairing: SlotPairingRuleId,
    /// Excludes the period — the `StudentExcludedPeriods` site.
    excluded_student: StudentId,
    /// Present for the period, and the one held by the assignments row.
    assigned_student: StudentId,
    group_list: GroupListId,
    /// `1e` only: excludes the *second* week, and is worn by the first slot.
    week_pattern: Option<WeekPatternId>,
}

/// Builds the document both `1c` and `1e` use.
///
/// With `depth == false` this is exactly `1c`: the period is referenced from
/// **all seven** of its sites at once (`WeekPeriodFk`,
/// `SubjectExcludedPeriods`, `StudentExcludedPeriods`,
/// `PairingRuleExcludedPeriods`, `SlotPairingRuleExcludedPeriods`,
/// `AssignmentsKey`, `AssociationEntry`) and every fix hangs flat off the
/// target.
///
/// With `depth == true` it is `1e`: a second week, a week pattern excluding
/// that second week and worn by the first slot, and a colloscope cell on the
/// first week. Two of the seven fixes then open sub-cascades of their own.
fn build_period_document(app: &mut AppState<Data, String>, depth: bool) -> PeriodDocument {
    let period: PeriodId = apply_new!(
        app,
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "adding a period"
    );

    let mut weeks = vec![apply_new!(
        app,
        Op::Week(WeekOp::AddFront(period, WeekDesc::default())),
        NewId::WeekId,
        "adding the first week"
    )];
    if depth {
        weeks.push(apply_new!(
            app,
            Op::Week(WeekOp::AddAfter(weeks[0], WeekDesc::default())),
            NewId::WeekId,
            "adding the second week"
        ));
    }

    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the running subject"
    );
    let excluded_subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            Some(subject),
            plain_subject("Sport", BTreeSet::from([period]))
        )),
        NewId::SubjectId,
        "adding the excluding subject"
    );
    let teacher: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "adding a teacher"
    );

    // `1e` only: the pattern excludes the second week, so removing that week
    // has to repair the pattern first. It is worn by the first slot, whose
    // colloscope cell sits on the *first* week — which the pattern leaves
    // active, so the cell is legal.
    let week_pattern: Option<WeekPatternId> = depth.then(|| {
        apply_new!(
            app,
            Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
                name: "skip the second week".into(),
                excluded_weeks: BTreeSet::from([weeks[1]]),
            })),
            NewId::WeekPatternId,
            "adding a week pattern"
        )
    });

    // Two slots on the same subject: the slot pairing rule below needs them
    // (`PairedSlotsNotInSameSubject`).
    let slots: Vec<SlotId> = vec![
        apply_new!(
            app,
            Op::Slot(SlotOp::AddAfter(
                None,
                make_slot(subject, teacher, week_pattern, 8)
            )),
            NewId::SlotId,
            "adding the first slot"
        ),
        apply_new!(
            app,
            Op::Slot(SlotOp::AddAfter(
                None,
                make_slot(subject, teacher, None, 10)
            )),
            NewId::SlotId,
            "adding the second slot"
        ),
    ];

    let pairing: PairingRuleId = apply_new!(
        app,
        Op::Pairing(PairingOp::Add(pairing_rule(
            subject,
            excluded_subject,
            BTreeSet::from([period])
        ))),
        NewId::PairingRuleId,
        "adding a pairing rule"
    );
    let slot_pairing: SlotPairingRuleId = apply_new!(
        app,
        Op::SlotPairing(SlotPairingOp::Add(slot_pairing_rule(
            slots[0],
            slots[1],
            BTreeSet::from([period])
        ))),
        NewId::SlotPairingRuleId,
        "adding a slot pairing rule"
    );

    let excluded_student: StudentId = apply_new!(
        app,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::from([period])))),
        NewId::StudentId,
        "adding the excluding student"
    );
    // A second student, present for the period: the assignments row needs one
    // (an assigned student who excluded the period would break
    // `AssignedStudentNotPresentForPeriod` instead).
    let assigned_student: StudentId = apply_new!(
        app,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::new()))),
        NewId::StudentId,
        "adding the assigned student"
    );
    apply_ok(
        &mut *app,
        Op::Assignment(collomatique_state_colloscopes::AssignmentOp::SetRow(
            period,
            subject,
            BTreeSet::from([assigned_student]),
        )),
        "filling the assignments row",
    );

    let group_list: GroupListId = apply_new!(
        app,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Liste",
            2,
            BTreeSet::new()
        ))),
        NewId::GroupListId,
        "adding a group list"
    );
    apply_ok(
        &mut *app,
        Op::GroupList(GroupListOp::AssignToSubject(
            period,
            subject,
            Some(group_list),
        )),
        "associating the group list",
    );

    if depth {
        apply_ok(
            &mut *app,
            Op::Colloscope(ColloscopeOp::SetInterrogation(
                slots[0],
                weeks[0],
                BTreeSet::from([0]),
            )),
            "filling the colloscope cell",
        );
    }

    PeriodDocument {
        period,
        weeks,
        subject,
        excluded_subject,
        slots,
        pairing,
        slot_pairing,
        excluded_student,
        assigned_student,
        group_list,
        week_pattern,
    }
}

/// The seven flat fixes a period removal draws off `build_period_document`,
/// hand-derived from the §8.1 table — one per `PeriodRefSite` variant, in
/// declaration order. `1c` lands exactly these; `1e` lands these plus the
/// fixes of the two sub-cascades.
fn seven_flat_period_fixes(doc: &PeriodDocument, week: WeekId) -> Vec<Fix> {
    vec![
        Fix::DeleteWeek { week },
        Fix::RemoveSubjectPeriodExclusion {
            subject: doc.excluded_subject,
            period: doc.period,
            rebuilt: plain_subject("Sport", BTreeSet::new()),
        },
        Fix::RemoveStudentPeriodExclusion {
            student: doc.excluded_student,
            period: doc.period,
            rebuilt: plain_student(BTreeSet::new()),
        },
        Fix::RemovePairingRulePeriodExclusion {
            rule: doc.pairing,
            period: doc.period,
            rebuilt: pairing_rule(doc.subject, doc.excluded_subject, BTreeSet::new()),
        },
        Fix::RemoveSlotPairingRulePeriodExclusion {
            rule: doc.slot_pairing,
            period: doc.period,
            rebuilt: slot_pairing_rule(doc.slots[0], doc.slots[1], BTreeSet::new()),
        },
        Fix::ClearAssignmentRow {
            period: doc.period,
            subject: doc.subject,
        },
        Fix::UnassignGroupList {
            period: doc.period,
            subject: doc.subject,
        },
    ]
}

/// Fixture `1c` — **breadth**. A period referenced from all seven of its sites
/// at once, every fix hanging flat off the target.
///
/// Note what this does *not* catch: an arm missing from the map is a compile
/// error already, since `fix_period_ref` matches `PeriodRefSite` totally with
/// no wildcard. What it catches is an arm that wrongly answers `None`, or
/// answers the wrong fix — including the two sealed rebuilds (`PairingRule` and
/// `SlotPairingRule`), whose `.expect` is exercised here for the first time.
#[test]
fn fixture_1c_all_seven_period_sites_are_repaired() {
    let mut app = AppState::<Data, String>::new(Data::new());
    let doc = build_period_document(&mut app, false);

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::Period(PeriodOp::Remove(doc.period)));

    let receipt = apply_cascade(&mut data, target).expect("the cascade resolves all seven sites");

    let expected = seven_flat_period_fixes(&doc, doc.weeks[0]);
    assert_same_fixes(&landed_fixes(&receipt), &expected);

    assert_clean(&data);
    let inner = data.get_inner_data();
    assert!(
        inner
            .params
            .periods
            .find_period_position(doc.period)
            .is_none(),
        "the target period is gone"
    );
    assert!(
        inner.params.weeks.find_week(doc.weeks[0]).is_none(),
        "so is its week"
    );
    assert!(
        inner
            .params
            .assignments
            .students(doc.period, doc.subject)
            .is_none(),
        "the assignments row is gone"
    );
    assert!(
        !inner
            .params
            .group_lists
            .subjects_associations
            .contains(&(doc.period, doc.subject)),
        "the association is gone"
    );
    // Every row that merely *mentioned* the period is updated, not deleted.
    assert!(
        inner
            .params
            .subjects
            .find_subject(doc.excluded_subject)
            .is_some(),
        "the excluding subject survives"
    );
    assert!(
        inner
            .params
            .students
            .student_map
            .get(&doc.excluded_student)
            .is_some(),
        "the excluding student survives"
    );
    assert!(
        inner
            .params
            .pairings
            .pairing_rule_map
            .get(&doc.pairing)
            .is_some(),
        "the pairing rule survives"
    );
    assert!(
        inner
            .params
            .slot_pairings
            .slot_pairing_rule_map
            .get(&doc.slot_pairing)
            .is_some(),
        "the slot pairing rule survives"
    );
    assert!(
        inner
            .params
            .group_lists
            .group_list_map
            .get(&doc.group_list)
            .is_some(),
        "the group list itself survives, merely unassigned"
    );
    assert!(
        inner
            .params
            .students
            .student_map
            .get(&doc.assigned_student)
            .is_some(),
        "and so does the student who was assigned"
    );
}

/// Fixture `1d` — **confluence on one op**. Two different broken invariants
/// whose arms emit the *same* fix.
///
/// `invariants.rs` runs two independent `if`s over the same placement with no
/// `else`, so one `GroupListOp::Update` that both shrinks `group_names` **and**
/// adds a student to `excluded_students` makes both
/// `ColloscopeStudentExcluded(gl, st)` and
/// `ColloscopeStudentGroupOutOfBounds(gl, st, 1)` fire against a live
/// colloscope row placing `st` at group 1. Per §8.2 both arms answer the same
/// [Fix::RemoveStudentColloscopePlacement] — the collapse the vocabulary makes
/// explicit.
///
/// The point of the fixture is the *second* half: whichever break is picked,
/// the one fix kills both, the retry succeeds, and no second fix is ever
/// requested. A redundant second fix would apply as a perfect no-op and the
/// engine would panic — so "exactly one fix landed" is the real assertion here.
#[test]
fn fixture_1d_two_invariants_resolved_by_one_fix() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let student: StudentId = apply_new!(
        app,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::new()))),
        NewId::StudentId,
        "adding a student"
    );
    let group_list: GroupListId = apply_new!(
        app,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Liste",
            2,
            BTreeSet::new()
        ))),
        NewId::GroupListId,
        "adding a two-group list"
    );
    apply_ok(
        &mut app,
        Op::Colloscope(ColloscopeOp::SetGroupList(
            group_list,
            BTreeMap::from([(student, 1u32)]),
        )),
        "placing the student in the second group",
    );

    // One group left (so group 1 is out of bounds) *and* the student excluded.
    let shrunk = automatic_group_list("Liste", 1, BTreeSet::from([student]));

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::GroupList(GroupListOp::Update(
        group_list,
        shrunk.clone(),
    )));

    let receipt = apply_cascade(&mut data, target).expect("the cascade resolves both breaks");

    assert_same_fixes(
        &landed_fixes(&receipt),
        &[Fix::RemoveStudentColloscopePlacement {
            group_list,
            student,
            rebuilt: BTreeMap::new(),
        }],
    );

    assert_clean(&data);
    let inner = data.get_inner_data();
    assert!(
        inner.colloscope.group_list(group_list).is_none(),
        "the placement row emptied, so it is gone"
    );
    assert_eq!(
        inner.params.group_lists.group_list_map.get(&group_list),
        Some(&shrunk),
        "the target landed exactly as written"
    );
}

/// Fixture `1e` — **the flagship**. `1c`'s document plus depth: the weeks carry
/// a colloscope cell and a week pattern, so two of the seven flat fixes open
/// sub-cascades of their own.
///
/// The hand-derived trace. The target reports eight breaks in round 1 (two
/// `WeekPeriodFk`, one per week, plus the other six sites), and
/// `PeriodRefSite::WeekPeriodFk` is declared first, so the weeks go first:
///
/// - `Week(Remove(first))` fails on the colloscope row keyed on that week →
///   `Colloscope(SetInterrogation(slot, first, ∅))`, then the removal lands;
/// - `Week(Remove(second))` fails on the week pattern that excludes it →
///   `WeekPattern(Update(pattern, minus second))`, then the removal lands;
/// - the remaining six sites are then repaired flat, and the target lands.
///
/// Ten fixes, then the target. Content, not sequence: round 1 makes a genuine
/// pick and pinning that pick is `1a`'s job.
#[test]
fn fixture_1e_the_flagship_period_removal() {
    let mut app = AppState::<Data, String>::new(Data::new());
    let doc = build_period_document(&mut app, true);
    let pattern = doc.week_pattern.expect("built with depth");

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::Period(PeriodOp::Remove(doc.period)));

    let receipt =
        apply_cascade(&mut data, target).expect("the cascade resolves the whole document");

    let mut expected = seven_flat_period_fixes(&doc, doc.weeks[0]);
    expected.extend([
        // The first week's sub-cascade.
        Fix::ClearInterrogationCell {
            slot: doc.slots[0],
            week: doc.weeks[0],
        },
        // The second week, and its own sub-cascade.
        Fix::RemoveWeekPatternExclusion {
            pattern,
            week: doc.weeks[1],
            rebuilt: WeekPattern {
                name: "skip the second week".into(),
                excluded_weeks: BTreeSet::new(),
            },
        },
        Fix::DeleteWeek { week: doc.weeks[1] },
    ]);
    assert_same_fixes(&landed_fixes(&receipt), &expected);

    assert_clean(&data);
    let inner = data.get_inner_data();
    assert!(
        inner
            .params
            .periods
            .find_period_position(doc.period)
            .is_none(),
        "the target period is gone"
    );
    for week in &doc.weeks {
        assert!(
            inner.params.weeks.find_week(*week).is_none(),
            "every week of the period went with it"
        );
        assert!(
            inner
                .colloscope
                .interrogation(doc.slots[0], *week)
                .is_none(),
            "and every colloscope cell that hung off one"
        );
    }
    assert!(
        inner
            .params
            .assignments
            .students(doc.period, doc.subject)
            .is_none(),
        "the assignments row is gone"
    );
    assert!(
        !inner
            .params
            .group_lists
            .subjects_associations
            .contains(&(doc.period, doc.subject)),
        "the association is gone"
    );
    // The slots, the pattern and the two rules are all updated or untouched —
    // never deleted. A period removal must not cost the timetable its slots.
    for slot in &doc.slots {
        assert!(
            inner.params.slots.find_slot(*slot).is_some(),
            "the slots survive"
        );
    }
    assert_eq!(
        inner
            .params
            .week_patterns
            .week_pattern_map
            .get(&pattern)
            .map(|p| p.excluded_weeks.clone()),
        Some(BTreeSet::new()),
        "the pattern survives, emptied of the week it excluded"
    );
}

/// Fixture `2` — **breadth below the root**, and full site coverage for two
/// more target kinds.
///
/// A teacher `t1` owning two slots. `TeacherRefSite` has the single variant
/// `SlotTeacher(SlotId)`, so `TeacherOp::Remove(t1)` breaks it twice at once
/// and each fix is the map's most destructive one, `Slot(Remove(..))`. Each of
/// those two slot removals then opens its *own* sub-cascade — which is what
/// separates this fixture from `1c`. `1c` is breadth at the root: seven fixes
/// hanging directly off the target, every one of them flat. Here the stack gets
/// wider at depth two, which neither `1b` (one break per round) nor `1c` (all
/// fixes flat) ever does.
///
/// `SlotRefSite` has exactly three variants and the document covers all three:
/// `slot_a` is the antecedent of one slot pairing rule and carries the
/// colloscope cell, `slot_b` is the consequent of a second rule. With `1c`'s
/// seven period sites that gives the suite full site coverage for three target
/// kinds — period, teacher and slot.
///
/// Two construction choices, both load-bearing and both taken from §9.
///
/// **A second teacher `t2`, with a slot of their own.** When a fix is as
/// explosive as "delete a whole teacher's timetable", what it leaves alone is
/// worth as much as what it removes, and the arm's identity test
/// (`slot.teacher_id != teacher` → `None`) is invisible to a document where
/// every slot belongs to the target. `slot_c` comes almost free, because it is
/// also the far end of both pairing rules.
///
/// **The two rules do not pair `t1`'s slots with each other.** They each pair
/// one of them with `slot_c`. Pairing two slots of the same teacher is
/// perfectly legal — only pairing a slot with *itself* is refused — so this is
/// a coverage choice, not a validity one. Were `slot_a` and `slot_b` paired
/// together by a single rule, the first slot removal would take that rule with
/// it, the second would find no rule left to break, and the two-arm coverage
/// would silently collapse to one arm with the test still green. Two rules are
/// what keep both arms reachable.
///
/// The trace, derived from the tables. Round 1 reports two `SlotTeacher`
/// breaks; the engine picks one — say `slot_a` — and its removal fails with two
/// dangling slot references. `SlotRefSite` declares `SlotPairingRuleAntecedent`
/// before `ColloscopeInterrogation`, so the rule goes first, then the cell,
/// then the slot; then the target is retried, `slot_b` is picked, its rule
/// goes, the slot goes, and the teacher lands. Five fixes. Which of the two slots
/// is picked first is not asserted — that is `1a`'s job — so this fixture
/// checks content, not sequence.
#[test]
fn fixture_2_teacher_removal_fans_out_below_the_root() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let period: PeriodId = apply_new!(
        app,
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "adding a period"
    );
    let week: WeekId = apply_new!(
        app,
        Op::Week(WeekOp::AddFront(period, WeekDesc::default())),
        NewId::WeekId,
        "adding a week"
    );
    // One subject for all three slots: a slot pairing rule whose two slots sit
    // on different subjects breaks `PairedSlotsNotInSameSubject`.
    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding a subject"
    );
    let teacher: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "adding the teacher to remove"
    );
    let other_teacher: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "adding the innocent teacher"
    );
    let slot_a: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(None, make_slot(subject, teacher, None, 8))),
        NewId::SlotId,
        "adding the first slot of the removed teacher"
    );
    let slot_b: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(
            Some(slot_a),
            make_slot(subject, teacher, None, 10)
        )),
        NewId::SlotId,
        "adding the second slot of the removed teacher"
    );
    let slot_c: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(
            Some(slot_b),
            make_slot(subject, other_teacher, None, 12)
        )),
        NewId::SlotId,
        "adding the innocent teacher's slot"
    );
    // `slot_a` on the antecedent side, `slot_b` on the consequent side: one
    // rule per arm of `SlotRefSite`, and `slot_c` on the far end of both.
    let rule_ac: SlotPairingRuleId = apply_new!(
        app,
        Op::SlotPairing(SlotPairingOp::Add(slot_pairing_rule(
            slot_a,
            slot_c,
            BTreeSet::new()
        ))),
        NewId::SlotPairingRuleId,
        "adding the antecedent-side rule"
    );
    let rule_cb: SlotPairingRuleId = apply_new!(
        app,
        Op::SlotPairing(SlotPairingOp::Add(slot_pairing_rule(
            slot_c,
            slot_b,
            BTreeSet::new()
        ))),
        NewId::SlotPairingRuleId,
        "adding the consequent-side rule"
    );
    // Forced by the group-number bound, exactly as in `1b`: with no association
    // on `(period, subject)` the bound is 0 and no cell can be filled at all.
    let group_list: GroupListId = apply_new!(
        app,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Liste",
            2,
            BTreeSet::new()
        ))),
        NewId::GroupListId,
        "adding a group list"
    );
    apply_ok(
        &mut app,
        Op::GroupList(GroupListOp::AssignToSubject(
            period,
            subject,
            Some(group_list),
        )),
        "associating the group list",
    );
    apply_ok(
        &mut app,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot_a,
            week,
            BTreeSet::from([0]),
        )),
        "filling the colloscope cell",
    );

    let innocent_slot = app
        .get_data()
        .get_inner_data()
        .params
        .slots
        .find_slot(slot_c)
        .expect("the innocent slot is there")
        .clone();

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::Teacher(TeacherOp::Remove(teacher)));

    let receipt = apply_cascade(&mut data, target).expect("the cascade resolves both fan-outs");

    let expected = vec![
        Fix::DeleteSlotPairingRule { rule: rule_ac },
        Fix::ClearInterrogationCell { slot: slot_a, week },
        Fix::DeleteSlot { slot: slot_a },
        Fix::DeleteSlotPairingRule { rule: rule_cb },
        Fix::DeleteSlot { slot: slot_b },
    ];
    assert_same_fixes(&landed_fixes(&receipt), &expected);

    assert_clean(&data);
    let inner = data.get_inner_data();
    assert!(
        inner.params.teachers.teacher_map.get(&teacher).is_none(),
        "the target teacher is gone"
    );
    for slot in [slot_a, slot_b] {
        assert!(
            inner.params.slots.find_slot(slot).is_none(),
            "both of the teacher's slots went with them"
        );
    }
    for rule in [rule_ac, rule_cb] {
        assert!(
            inner
                .params
                .slot_pairings
                .slot_pairing_rule_map
                .get(&rule)
                .is_none(),
            "each rule went with the slot it named"
        );
    }
    assert!(
        inner.colloscope.interrogation(slot_a, week).is_none(),
        "the cell that hung off the removed slot is gone"
    );
    // The identity test in the `SlotTeacher` arm: the other teacher's timetable
    // is not collateral damage.
    assert!(
        inner
            .params
            .teachers
            .teacher_map
            .get(&other_teacher)
            .is_some(),
        "the innocent teacher is untouched"
    );
    assert_eq!(
        inner.params.slots.find_slot(slot_c),
        Some(&innocent_slot),
        "and their slot is byte-identical, not merely present"
    );
}

/// Fixture `3` — a **fix that is itself rejected and cascades further**, along a
/// chain of `Convergence` breaks.
///
/// Every fixture before this one walks `DanglingFk` sites: something is removed
/// and the references to it are repaired. This one walks a different axis of the
/// map. Nothing dangles here — every reference resolves — and what is wrong is a
/// *relation* between two live rows.
///
/// A subject `S` with interrogations enabled, a teacher `t` who teaches it, a
/// slot on `S` taught by `t`, a group-list association on `(P, S)` and a
/// balancing override on `S`. Target:
/// `SubjectOp::Update(S, the same subject with interrogations off)`.
///
/// The trace, derived from §8.2's table and not from a run. `Convergence`'s
/// declaration order is the canonical pick order.
///
/// - **Round 1.** The target applies and breaks **four** invariants at once:
///   `TeacherSubjectWithoutInterrogations`, `SlotForSubjectWithoutInterrogations`,
///   `AssociationForSubjectWithoutInterrogations` and
///   `BalancingForSubjectWithoutInterrogations`.
///   `SlotTeacherDoesNotTeachSubject` — declared before all of them — does *not*
///   fire: the teacher still teaches the subject. So the pick is
///   `TeacherSubjectWithoutInterrogations` and the fix is
///   `Teacher(Update(t, minus S))`.
/// - **Round 2.** That fix is applied over the rolled-back document, and **fails
///   the gate itself**: with `S` gone from the teacher, the slot's teacher no
///   longer teaches the slot's subject, so `SlotTeacherDoesNotTeachSubject`
///   breaks. Its arm removes the slot.
/// - **Round 3 onward.** The slot removal lands; the teacher trim is retried and
///   lands; the target is retried and now reports only the association and the
///   balancing breaks, which clear one per round in declaration order; then the
///   target lands.
///
/// Four fixes, not three — the extra one is the slot removal, attributed to
/// `SlotTeacherDoesNotTeachSubject` rather than to the arm one would expect. The
/// intermediate state that produces it — a teacher who has dropped a subject
/// while a slot of theirs still runs on it — is one no user action can reach
/// directly, which is exactly where a map bug would hide.
///
/// Note what is *not* in the landed set: `SlotForSubjectWithoutInterrogations`
/// is never picked, even though it fires in round 1. Its slot is already gone,
/// removed by a different arm. That is structural rather than an artefact of
/// this document — §8.2's row 3 carries the argument — and this fixture is where
/// it shows.
///
/// **A second subject `S2`, also taught by `t`**, is the innocent bystander. It
/// keeps its interrogations, so it takes no part in the trace; what it buys is
/// the shape of the teacher fix. `Teacher::subjects` is a set, so §8.2's row 2
/// claims the offending *element* leaves and the teacher survives — and with a
/// single-subject teacher the resulting empty set is indistinguishable from an
/// arm that cleared the whole thing. The fix carries the rebuilt teacher and is
/// compared whole, so `S2` still being in it is the assertion that separates the
/// two.
///
/// Content, not sequence: round 1 has four simultaneous breaks, so the engine
/// genuinely picks, and pinning that pick is `1a`'s job.
#[test]
fn fixture_3_a_rejected_fix_cascades_through_convergence_breaks() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let period: PeriodId = apply_new!(
        app,
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "adding a period"
    );
    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the subject to turn off"
    );
    // The innocent bystander: it keeps its interrogations, and its only job is
    // to be still there in the teacher the fix rebuilds.
    let other_subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            Some(subject),
            interrogation_subject("Physique", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the innocent subject"
    );
    let teacher: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject, other_subject]),
        })),
        NewId::TeacherId,
        "adding the teacher"
    );
    let slot: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(None, make_slot(subject, teacher, None, 8))),
        NewId::SlotId,
        "adding the slot"
    );
    let group_list: GroupListId = apply_new!(
        app,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Liste",
            2,
            BTreeSet::new()
        ))),
        NewId::GroupListId,
        "adding a group list"
    );
    apply_ok(
        &mut app,
        Op::GroupList(GroupListOp::AssignToSubject(
            period,
            subject,
            Some(group_list),
        )),
        "associating the group list",
    );
    // Deliberately *not* the default options: the entry has to be visibly
    // different from the global ones, so that its removal is a real change and
    // the global ones can be checked untouched at the end.
    let override_options = BalancingOptions {
        avoid_twice_in_a_row: false,
        year_teacher_rotation: true,
        ..Default::default()
    };
    apply_ok(
        &mut app,
        Op::Balancing(BalancingOp::SetSubject(
            subject,
            Some(override_options.clone()),
        )),
        "setting the balancing override",
    );

    let global_balancing = app
        .get_data()
        .get_inner_data()
        .params
        .balancing
        .global
        .clone();

    let subject_off = plain_subject("Math", BTreeSet::new());
    let mut data = app.get_data().clone();
    let (target, _new_info) =
        data.annotate(Op::Subject(SubjectOp::Update(subject, subject_off.clone())));

    let receipt = apply_cascade(&mut data, target).expect("the cascade resolves the chain");

    let expected = vec![
        Fix::DeleteSlot { slot },
        Fix::RemoveTeacherSubject {
            teacher,
            subject,
            rebuilt: Teacher {
                desc: Default::default(),
                subjects: BTreeSet::from([other_subject]),
            },
        },
        Fix::UnassignGroupList { period, subject },
        Fix::ClearSubjectBalancing { subject },
    ];
    assert_same_fixes(&landed_fixes(&receipt), &expected);

    assert_clean(&data);
    let inner = data.get_inner_data();
    assert_eq!(
        inner.params.subjects.find_subject(subject),
        Some(&subject_off),
        "the target landed: the subject runs no interrogations any more"
    );
    assert!(
        inner.params.slots.find_slot(slot).is_none(),
        "the slot went with the teacher's trim, not with the target"
    );
    // §8.2 row 2: the offending element leaves, the teacher stays — and `S2` is
    // what tells that apart from a cleared set.
    assert_eq!(
        inner.params.teachers.teacher_map.get(&teacher),
        Some(&Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([other_subject]),
        }),
        "the teacher survives, having lost exactly the one subject"
    );
    assert!(
        inner
            .params
            .group_lists
            .subjects_associations
            .get(&(period, subject))
            .is_none(),
        "the association is cleared"
    );
    assert!(
        inner
            .params
            .group_lists
            .group_list_map
            .get(&group_list)
            .is_some(),
        "unassigning leaves the group list itself in place"
    );
    assert!(
        inner.params.balancing.subjects.get(&subject).is_none(),
        "the balancing override is cleared"
    );
    assert_eq!(
        inner.params.balancing.global, global_balancing,
        "and the global balancing options are untouched"
    );
    assert_eq!(
        inner.params.balancing.options_for(subject),
        &global_balancing,
        "so the subject falls back to the global options"
    );
    assert_eq!(
        inner.params.subjects.find_subject(other_subject),
        Some(&interrogation_subject("Physique", BTreeSet::new())),
        "the innocent subject is untouched"
    );
}

/// Fixture `4` — **full site coverage for the student**, and the only fixture
/// that reaches a group list at all.
///
/// `StudentRefSite` has five variants (`refs.rs:154-169`) and this document
/// holds all five at once, so `StudentOp::Remove(st)` breaks five references in
/// a single round and every fix hangs flat off the target. Five fixes land,
/// then the target.
///
/// Covering the five needs **three** group lists, because a filling is either
/// `Prefilled` or `Automatic` and one list cannot play two of the roles:
///
/// - `gl1`, **prefilled**, one of whose groups holds `st` →
///   `GroupListPrefilledStudent`;
/// - `gl2`, **automatic**, excluding `st` → `GroupListExcludedStudent`;
/// - `gl3`, **automatic**, *not* excluding `st`, carrying a colloscope row that
///   places them → `ColloscopeGroupListStudent`. It has to be a third list:
///   placing `st` in `gl2` would break `ColloscopeStudentExcluded`, and in `gl1`
///   would break `ColloscopeGroupListPrefilled`, so either would be testing
///   something else by accident;
///
/// plus a per-student settings override (`SettingsStudentKey`) and an
/// assignments row holding `st` (`AssignmentsStudent { period, subject }`).
///
/// **Why the fixture is worth its weight: `gl1` and `gl2`.** Their two arms are
/// the sealed `GroupList::new` rebuilds of §8.1, each carrying an `.expect`.
/// Commit 7.5 tests only their `None` branch, and no other fixture in this file
/// reaches a group list at all — so without this one those two `.expect`s are
/// never executed by any test in the suite. (`1c` covers the other two sealed
/// rebuilds, `PairingRule` and `SlotPairingRule`.) It is also the only fixture
/// exercising `SettingsOp::SetStudent`, the elementary op split out in commit
/// 5.98.
///
/// **A second student `st2`**, sitting in the *same* prefilled group, the *same*
/// assignments row and the *same* colloscope row. Three of the five fixes carry
/// a rebuilt collection, and the fixes are compared whole, so `st2` still being
/// in each of them is what separates "the offending element left" from "the
/// collection was cleared". The other two fixes need no bystander: the settings
/// fix names its key, and `gl2`'s excluded set has only `st` in it by
/// construction.
///
/// Content, not sequence: five simultaneous breaks means the engine genuinely
/// picks, and pinning that pick is `1a`'s job.
#[test]
fn fixture_4_student_removal_covers_all_five_student_sites() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let period: PeriodId = apply_new!(
        app,
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "adding a period"
    );
    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding a subject"
    );
    let student: StudentId = apply_new!(
        app,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::new()))),
        NewId::StudentId,
        "adding the student to remove"
    );
    let other_student: StudentId = apply_new!(
        app,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::new()))),
        NewId::StudentId,
        "adding the innocent student"
    );

    // Site 1: a prefilled group holding the student — and the bystander with
    // them, so the rebuilt list has something to keep.
    let gl1: GroupListId = apply_new!(
        app,
        Op::GroupList(GroupListOp::Add(prefilled_group_list(
            "Prérempli",
            vec![BTreeSet::from([student, other_student]), BTreeSet::new()]
        ))),
        NewId::GroupListId,
        "adding the prefilled group list"
    );
    // Site 2: an automatic list excluding the student.
    let gl2: GroupListId = apply_new!(
        app,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Exclusions",
            2,
            BTreeSet::from([student])
        ))),
        NewId::GroupListId,
        "adding the excluding group list"
    );
    // Site 5: an automatic list that excludes nobody, so it can carry a
    // colloscope row placing the student.
    let gl3: GroupListId = apply_new!(
        app,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Colloscope",
            2,
            BTreeSet::new()
        ))),
        NewId::GroupListId,
        "adding the placed group list"
    );
    apply_ok(
        &mut app,
        Op::Colloscope(ColloscopeOp::SetGroupList(
            gl3,
            BTreeMap::from([(student, 0), (other_student, 1)]),
        )),
        "placing both students in the colloscope group list",
    );

    // Site 3: a per-student settings override. Deliberately not the default
    // `Limits`, so that clearing it is a visible change and the global limits
    // can be asserted untouched at the end.
    let override_limits = Limits {
        interrogations_per_week_max: Some(SoftParam {
            soft: false,
            value: 3,
        }),
        ..Default::default()
    };
    apply_ok(
        &mut app,
        Op::Settings(SettingsOp::SetStudent(
            student,
            Some(override_limits.clone()),
        )),
        "setting the per-student limits",
    );
    // Site 4: an assignments row holding the student, and the bystander.
    apply_ok(
        &mut app,
        Op::Assignment(AssignmentOp::SetRow(
            period,
            subject,
            BTreeSet::from([student, other_student]),
        )),
        "filling the assignments row",
    );

    let global_limits = app
        .get_data()
        .get_inner_data()
        .params
        .settings
        .global
        .clone();
    let innocent_student = app
        .get_data()
        .get_inner_data()
        .params
        .students
        .student_map
        .get(&other_student)
        .expect("the innocent student is there")
        .clone();

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::Student(StudentOp::Remove(student)));

    let receipt = apply_cascade(&mut data, target).expect("the cascade repairs all five sites");

    let expected = vec![
        Fix::RemoveStudentFromGroupListPrefill {
            group_list: gl1,
            student,
            rebuilt: prefilled_group_list(
                "Prérempli",
                vec![BTreeSet::from([other_student]), BTreeSet::new()],
            ),
        },
        Fix::RemoveStudentGroupListExclusion {
            group_list: gl2,
            student,
            rebuilt: automatic_group_list("Exclusions", 2, BTreeSet::new()),
        },
        Fix::ClearStudentSettings { student },
        Fix::RemoveStudentFromAssignmentRow {
            period,
            subject,
            student,
            rebuilt: BTreeSet::from([other_student]),
        },
        Fix::RemoveStudentColloscopePlacement {
            group_list: gl3,
            student,
            rebuilt: BTreeMap::from([(other_student, 1)]),
        },
    ];
    assert_same_fixes(&landed_fixes(&receipt), &expected);

    assert_clean(&data);
    let inner = data.get_inner_data();
    assert!(
        inner.params.students.student_map.get(&student).is_none(),
        "the target student is gone"
    );
    assert_eq!(
        inner.params.group_lists.group_list_map.get(&gl1),
        Some(&prefilled_group_list(
            "Prérempli",
            vec![BTreeSet::from([other_student]), BTreeSet::new()]
        )),
        "the prefilled group lost the student and kept the other one"
    );
    assert_eq!(
        inner.params.group_lists.group_list_map.get(&gl2),
        Some(&automatic_group_list("Exclusions", 2, BTreeSet::new())),
        "the excluding list no longer excludes them"
    );
    assert!(
        inner.params.settings.students.get(&student).is_none(),
        "the per-student settings override is gone"
    );
    assert_eq!(
        inner.params.settings.global, global_limits,
        "and the global limits are untouched"
    );
    assert_eq!(
        inner.params.assignments.students(period, subject),
        Some(&BTreeSet::from([other_student])),
        "the assignments row lost the student and kept the other one"
    );
    assert_eq!(
        inner.colloscope.group_list(gl3),
        Some(&BTreeMap::from([(other_student, 1)])),
        "and so did the colloscope row, the other student keeping their group"
    );
    assert_eq!(
        inner
            .params
            .students
            .student_map
            .get(&other_student)
            .cloned(),
        Some(innocent_student),
        "the innocent student is byte-identical, not merely present"
    );
}

/// The document shared by fixtures `5a` and `5b`.
struct WeekPatternDocument {
    /// The pattern both targets act on. Excludes `blocked_week`.
    pattern: WeekPatternId,
    /// The innocent bystander pattern, worn by `other_slot` and
    /// `other_incompat`.
    other_pattern: WeekPatternId,
    /// Carries the colloscope cell. `pattern` allows it.
    cell_week: WeekId,
    /// Excluded by `pattern`: the week `slot` gets back when `pattern` dies.
    blocked_week: WeekId,
    /// Wears `pattern`, and carries the cell on `cell_week`.
    slot: SlotId,
    /// Wears `other_pattern`.
    other_slot: SlotId,
    /// Wears `pattern`.
    incompat: IncompatId,
    /// Wears `other_pattern`.
    other_incompat: IncompatId,
}

/// Builds the document both `5a` and `5b` use.
///
/// A pattern `WP` excluding one week, worn by one slot and one
/// incompatibility; the slot carries a colloscope cell on a week `WP` allows;
/// and a second pattern `WP2` with its own slot and its own incompatibility.
///
/// The slot and the incompatibility are given **non-default field values**
/// (`extra_info`, `cost`, a name, a `minimum_free_slots` of 2). Both arms
/// rebuild the whole row to clear one field, and `5a` compares the resulting
/// ops whole — but a rebuild that reset a field to its default would be
/// invisible against a row whose fields were already at their defaults.
fn build_week_pattern_document(app: &mut AppState<Data, String>) -> WeekPatternDocument {
    let period: PeriodId = apply_new!(
        app,
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "adding a period"
    );
    let cell_week: WeekId = apply_new!(
        app,
        Op::Week(WeekOp::AddFront(period, WeekDesc::default())),
        NewId::WeekId,
        "adding the week that carries the cell"
    );
    let blocked_week: WeekId = apply_new!(
        app,
        Op::Week(WeekOp::AddAfter(cell_week, WeekDesc::default())),
        NewId::WeekId,
        "adding the week the pattern blocks"
    );
    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding a subject"
    );
    let teacher: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "adding a teacher"
    );

    let pattern: WeekPatternId = apply_new!(
        app,
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "Semaines A".into(),
            excluded_weeks: BTreeSet::from([blocked_week]),
        })),
        NewId::WeekPatternId,
        "adding the target pattern"
    );
    let other_pattern: WeekPatternId = apply_new!(
        app,
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "Semaines B".into(),
            excluded_weeks: BTreeSet::from([cell_week]),
        })),
        NewId::WeekPatternId,
        "adding the innocent pattern"
    );

    let mut slot_value = make_slot(subject, teacher, Some(pattern), 8);
    slot_value.extra_info = "salle 12".into();
    slot_value.cost = 7;
    let slot: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(None, slot_value)),
        NewId::SlotId,
        "adding the slot wearing the target pattern"
    );
    let mut other_slot_value = make_slot(subject, teacher, Some(other_pattern), 10);
    other_slot_value.extra_info = "salle 4".into();
    other_slot_value.cost = -3;
    let other_slot: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(Some(slot), other_slot_value)),
        NewId::SlotId,
        "adding the innocent slot"
    );

    let incompat: IncompatId = apply_new!(
        app,
        Op::Incompat(IncompatOp::Add(Incompatibility {
            subject_id: subject,
            name: "Sport".into(),
            slots: vec![],
            minimum_free_slots: NonZeroU32::new(2).unwrap(),
            week_pattern_id: Some(pattern),
        })),
        NewId::IncompatId,
        "adding the incompatibility wearing the target pattern"
    );
    let other_incompat: IncompatId = apply_new!(
        app,
        Op::Incompat(IncompatOp::Add(Incompatibility {
            subject_id: subject,
            name: "Musique".into(),
            slots: vec![],
            minimum_free_slots: NonZeroU32::new(3).unwrap(),
            week_pattern_id: Some(other_pattern),
        })),
        NewId::IncompatId,
        "adding the innocent incompatibility"
    );

    // Forced by the group-number bound, exactly as in `1b`: with no association
    // on `(period, subject)` the bound is 0 and no cell can be filled at all.
    let group_list: GroupListId = apply_new!(
        app,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Liste",
            2,
            BTreeSet::new()
        ))),
        NewId::GroupListId,
        "adding a group list"
    );
    apply_ok(
        app,
        Op::GroupList(GroupListOp::AssignToSubject(
            period,
            subject,
            Some(group_list),
        )),
        "associating the group list",
    );
    apply_ok(
        app,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot,
            cell_week,
            BTreeSet::from([0]),
        )),
        "filling the colloscope cell",
    );

    WeekPatternDocument {
        pattern,
        other_pattern,
        cell_week,
        blocked_week,
        slot,
        other_slot,
        incompat,
        other_incompat,
    }
}

/// Fixture `5a` — **the deliberate divergence from the legacy cleaning**
/// (design-doc D5.4).
///
/// Target: `WeekPatternOp::Remove(WP)`. Both sites hold the pattern in an
/// `Option` whose `None` is a legal, documented value, so the reference can go
/// alone and the row stays. The legacy cleaning deleted both rows
/// (`ops/src/week_patterns.rs:229-256`); the map clears the field instead.
///
/// One round, two breaks — `SlotWeekPattern(slot)` and
/// `IncompatWeekPattern(incompat)` — whose fixes are independent. Content, not
/// sequence: the order here would teach nothing `1a` does not already pin.
/// Two fixes, and that length is the concrete form of §8.1's argument that
/// clearing to `None` can only ever *remove* instances of
/// `InterrogationOnInactiveWeek`, never create one. If a future change made
/// widening break something, the length is where it would surface.
///
/// The two fixes are compared **whole**. Each carries an entire rebuilt row, so
/// the exact fix pins that *only* `week_pattern` moved — an arm that rebuilt the
/// row from something else, or reset another field on the way, is caught here.
/// "The row survives intact" is the whole claim of the divergence, so the test
/// checks the whole row rather than one field. The builder gives both rows
/// non-default fields precisely so that this comparison has teeth.
///
/// **The semantic assertion is a before/after flip, not a final value.**
/// Asserting `slot.week_pattern == None` would say a field moved; it would not
/// say the slot got *wider*, which is what the divergence claims. So the fixture
/// takes `blocked_week` — a week `WP` excluded — and checks that
/// `is_interrogation_possible(slot, blocked_week)` is `false` before the cascade
/// and `true` after. A bare "true at the end" could pass for the wrong reason;
/// the flip cannot.
///
/// (Note what `None` does *not* mean. `is_week_active` is a conjunction: the
/// week must run interrogations **and** not be excluded by the slot's pattern.
/// Clearing the pattern drops the second conjunct only. Whether the first
/// conjunct still gates is `colloscope_params`' business, not the cascade's, so
/// this fixture does not test it.)
///
/// `WP2`, its slot and its incompatibility are the innocent bystanders. Both
/// arms test `slot.week_pattern == Some(WP)` (resp. `incompat.week_pattern_id`)
/// before clearing. If every pattern-bearing row in the document pointed at
/// `WP`, that comparison would pass trivially and a map that cleared *every*
/// row's pattern would pass the fixture. This is the same move as scenario 2's
/// second teacher and scenario 4's second student.
#[test]
fn fixture_5a_week_pattern_removal_widens_its_rows_instead_of_deleting_them() {
    let mut app = AppState::<Data, String>::new(Data::new());
    let doc = build_week_pattern_document(&mut app);

    let before = app.get_data().get_inner_data();
    let slot_before = before
        .params
        .slots
        .find_slot(doc.slot)
        .expect("the slot is there")
        .clone();
    let incompat_before = before
        .params
        .incompats
        .incompat_map
        .get(&doc.incompat)
        .expect("the incompatibility is there")
        .clone();
    let other_pattern_before = before
        .params
        .week_patterns
        .week_pattern_map
        .get(&doc.other_pattern)
        .expect("the innocent pattern is there")
        .clone();
    let other_slot_before = before
        .params
        .slots
        .find_slot(doc.other_slot)
        .expect("the innocent slot is there")
        .clone();
    let other_incompat_before = before
        .params
        .incompats
        .incompat_map
        .get(&doc.other_incompat)
        .expect("the innocent incompatibility is there")
        .clone();
    // The first half of the flip: while `WP` lives, it blocks the slot here.
    assert!(
        !before
            .params
            .is_interrogation_possible(doc.slot, doc.blocked_week),
        "the pattern blocks the slot on that week to begin with"
    );

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::WeekPattern(WeekPatternOp::Remove(doc.pattern)));

    let receipt = apply_cascade(&mut data, target).expect("the cascade clears both references");

    let mut widened_slot = slot_before.clone();
    widened_slot.week_pattern = None;
    let mut widened_incompat = incompat_before.clone();
    widened_incompat.week_pattern_id = None;
    let expected = vec![
        Fix::ClearSlotWeekPattern {
            slot: doc.slot,
            rebuilt: widened_slot.clone(),
        },
        Fix::ClearIncompatWeekPattern {
            incompat: doc.incompat,
            rebuilt: widened_incompat.clone(),
        },
    ];
    assert_same_fixes(&landed_fixes(&receipt), &expected);

    assert_clean(&data);
    let inner = data.get_inner_data();
    assert!(
        inner
            .params
            .week_patterns
            .week_pattern_map
            .get(&doc.pattern)
            .is_none(),
        "the target pattern is gone"
    );
    // The divergence itself: legacy deleted these two rows.
    assert_eq!(
        inner.params.slots.find_slot(doc.slot),
        Some(&widened_slot),
        "the slot survives, with only its pattern cleared"
    );
    assert_eq!(
        inner.params.incompats.incompat_map.get(&doc.incompat),
        Some(&widened_incompat),
        "and so does the incompatibility"
    );
    // The second half of the flip: the week the dead pattern used to block is
    // available again.
    assert!(
        inner
            .params
            .is_interrogation_possible(doc.slot, doc.blocked_week),
        "clearing the pattern widens the slot onto the week it used to block"
    );
    assert!(
        inner
            .colloscope
            .interrogation(doc.slot, doc.cell_week)
            .is_some(),
        "widening destroys nothing: the cell is still there"
    );
    assert_eq!(
        inner
            .params
            .week_patterns
            .week_pattern_map
            .get(&doc.other_pattern),
        Some(&other_pattern_before),
        "the innocent pattern is byte-identical"
    );
    assert_eq!(
        inner.params.slots.find_slot(doc.other_slot),
        Some(&other_slot_before),
        "and so is its slot — the arm's identity test is doing its job"
    );
    assert_eq!(
        inner.params.incompats.incompat_map.get(&doc.other_incompat),
        Some(&other_incompat_before),
        "and so is its incompatibility"
    );
}

/// Fixture `5b` — the **legacy-agreement** case, and the only commit-7 fixture
/// that reaches §8.2 row 12.
///
/// Target: `WeekPatternOp::Update(WP, excluded_weeks + cell_week)`, with the
/// slot's colloscope cell sitting on `cell_week`. One break,
/// `InterrogationOnInactiveWeek(slot, cell_week)`; the fix clears the cell; then
/// the update lands. One fix, then the target.
///
/// It sits next to `5a` on purpose. When the pattern *narrows*, the map does
/// what the legacy cleaning did — `UpdateWeekPattern`
/// (`ops/src/week_patterns.rs:200-226`) clears exactly the newly excluded cells,
/// one at a time. When the pattern *disappears*, the map deliberately does not
/// (`5a`). A reader who finds `5a` strange gets the contrast here.
///
/// `1b` and `1e` also clear colloscope cells, but through the
/// `ColloscopeInterrogation` dangling-FK arm on week removal, which is a
/// different arm entirely. Without this fixture §8.2 row 12 is covered only by
/// commit 7.5's `None` branch and by whatever commit 8's random walk happens to
/// hit.
///
/// The mirror of `5a`'s flip closes the pair: narrowing makes the cell's week
/// impossible for the slot, where removing the pattern made a blocked week
/// possible.
#[test]
fn fixture_5b_week_pattern_update_clears_the_newly_inactive_cell() {
    let mut app = AppState::<Data, String>::new(Data::new());
    let doc = build_week_pattern_document(&mut app);

    let before = app.get_data().get_inner_data();
    let slot_before = before
        .params
        .slots
        .find_slot(doc.slot)
        .expect("the slot is there")
        .clone();
    assert!(
        before
            .params
            .is_interrogation_possible(doc.slot, doc.cell_week),
        "the cell's week is possible for the slot to begin with"
    );

    let narrowed = WeekPattern {
        name: "Semaines A".into(),
        excluded_weeks: BTreeSet::from([doc.blocked_week, doc.cell_week]),
    };
    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::WeekPattern(WeekPatternOp::Update(
        doc.pattern,
        narrowed.clone(),
    )));

    let receipt = apply_cascade(&mut data, target).expect("the cascade clears the stranded cell");

    let expected = vec![Fix::ClearInterrogationCell {
        slot: doc.slot,
        week: doc.cell_week,
    }];
    assert_same_fixes(&landed_fixes(&receipt), &expected);

    assert_clean(&data);
    let inner = data.get_inner_data();
    assert_eq!(
        inner
            .params
            .week_patterns
            .week_pattern_map
            .get(&doc.pattern),
        Some(&narrowed),
        "the target landed: the pattern now excludes the cell's week too"
    );
    assert!(
        inner
            .colloscope
            .interrogation(doc.slot, doc.cell_week)
            .is_none(),
        "the cell the narrowing stranded is gone"
    );
    assert_eq!(
        inner.params.slots.find_slot(doc.slot),
        Some(&slot_before),
        "the slot itself is untouched: the map cleared the cell, not the row"
    );
    assert!(
        !inner
            .params
            .is_interrogation_possible(doc.slot, doc.cell_week),
        "and the week is now impossible for the slot — the mirror of 5a's flip"
    );
}

/// Fixture `6` — **a no-op target lands, and does not panic**.
///
/// Every other fixture in this file drives the resolution map. This one does
/// not: the target breaks nothing, so the map is never consulted. What it
/// guards is a deliberate carve-out in the engine that nothing else touches.
///
/// `cascade.rs:81-83` computes the no-op snapshot for fix ops only:
///
/// ```ignore
/// // Snapshot for the no-op-fix panic; only fix ops are held to it (a
/// // no-op *target* is a legitimate perfect no-op, G.2).
/// let before = (!is_target).then(|| data.clone());
/// ```
///
/// A fix that applies as a perfect no-op is a map-contract violation and the
/// engine panics on it — the map owes a `None` when there is nothing to repair.
/// A *target* that applies as a perfect no-op is legitimate, because the apply
/// gate accepts perfect no-ops (the G.2 widening), so the target is exempt from
/// that check on purpose. Drop the `(!is_target)` guard and make the snapshot
/// unconditional, and every no-op target starts panicking — and without this
/// fixture no test in the suite would notice. The toy tests in `cascade.rs` do
/// not cover it, and `property_apply_gate.rs` exercises the gate rather than the
/// cascade.
///
/// An identical `Slot` is the right op for the job: unambiguously a no-op, and
/// clear of the canonical-absent rules that would make an emptying colloscope or
/// assignments write a *real* change. The expected value is read from the live
/// document, so "identical" is guaranteed rather than reconstructed.
///
/// This fixture replaces a "clean target lands alone" one — a benign edit
/// cascading to exactly `[itself]` — dropped at the July 28 2026 review as
/// testing nothing new: the engine's fast path is already toy test 3, and "an
/// ordinary edit does not trip the checker" is what the rest of the suite does
/// all day.
#[test]
fn fixture_6_a_no_op_target_lands_alone_and_does_not_panic() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding a subject"
    );
    let teacher: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "adding a teacher"
    );
    let slot: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(None, make_slot(subject, teacher, None, 8))),
        NewId::SlotId,
        "adding a slot"
    );

    let mut data = app.get_data().clone();
    let before = data.get_inner_data().clone();
    // Read back rather than rebuild: this is the identical value by
    // construction, which is the whole point of the op.
    let identical_slot = before
        .params
        .slots
        .find_slot(slot)
        .expect("the slot is there")
        .clone();

    let (target, _new_info) = data.annotate(Op::Slot(SlotOp::Update(slot, identical_slot.clone())));

    let receipt = apply_cascade(&mut data, target).expect("a no-op target is accepted, not fixed");

    assert!(
        receipt.fixes().is_empty(),
        "nothing broke, so the map was never consulted: {:#?}",
        landed_fixes(&receipt)
    );
    let landed: Vec<AnnotatedOp> = receipt
        .into_aggregated_op()
        .inner()
        .iter()
        .map(|step| step.inner().clone())
        .collect();
    assert_eq!(
        landed,
        vec![AnnotatedOp::from(AnnotatedSlotOp::Update(
            slot,
            identical_slot
        ))],
        "the target lands alone"
    );

    assert_clean(&data);
    assert_eq!(
        data.get_inner_data(),
        &before,
        "and the document is byte-identical — the no-op really was one"
    );
}

// ---------------------------------------------------------------------------
// Commit 7.6 (plan §9ter) — the rejection fixtures.
// ---------------------------------------------------------------------------

/// The document both `SlotOverflowsDay` fixtures start from: a subject whose
/// interrogations last an hour, a teacher who teaches it, and one slot at
/// 23:00.
///
/// The document is **valid**. 23:00 plus 60 minutes ends exactly at midnight,
/// and a slot ending exactly at midnight does not overflow —
/// `SlotWithDuration::new` accepts it, and its doctest pins `22:00 + 2h =
/// 00:00` as `Some` (`time/src/lib.rs:632-643`).
///
/// The 23:00 start is load-bearing for the **pair**, not just for `1a`. `1b`
/// reuses this document and grows the interrogation to 90 minutes: from 22:00
/// that would end at 23:30, overflow nothing, and `1b` would pass while testing
/// nothing at all. From 23:00 both halves overflow — 23:30 + 60 for `1a`,
/// 23:00 + 90 for `1b`.
fn document_with_a_slot_ending_at_midnight()
-> (AppState<Data, String>, SubjectId, TeacherId, SlotId) {
    let mut app = AppState::<Data, String>::new(Data::new());

    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the subject"
    );
    let teacher: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "adding the teacher"
    );
    let slot: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(
            None,
            make_slot_at(subject, teacher, None, 23, 0)
        )),
        NewId::SlotId,
        "adding the slot that ends exactly at midnight"
    );

    (app, subject, teacher, slot)
}

/// Rejection `1a` — `SlotOverflowsDay`, **rejected**. The op moves the slot.
///
/// Target: `SlotOp::Update(slot, the same slot at 23:30)`. The op applies,
/// `SlotWithDuration::new(23:30, 60)` is `None`, and the checker reports
/// `SlotOverflowsDay { slot, start: 23:30, duration: 60 }`
/// (`invariants.rs:451-462`). The apply gate rolls the op back, so the arm is
/// asked about that invariant on the **restored** document — where the slot
/// still starts at 23:00. `23:00 != 23:30`, the shape test fails, the arm
/// answers `None`, and since the failing op is the target the engine restores
/// its entry snapshot and returns the break.
///
/// The expected set is derived by hand and is a single element. The slot's
/// teacher teaches its subject and the subject runs interrogations, so neither
/// `SlotTeacherDoesNotTeachSubject` nor `SlotForSubjectWithoutInterrogations`
/// fires, and the document holds nothing else at all.
///
/// **What the failure would look like.** Without the `start` comparison the arm
/// has only one possible answer, `Some(Slot::Remove(slot))`: the user asks to
/// move a slot half an hour later, and the application deletes it and reports
/// success. This fixture is the end-to-end reason commit 5.97 put `start` in
/// the payload — before that enrichment the variant carried a `SlotId` alone
/// and the arm had no second conjunct to test.
///
/// The op is unwritable through `ops/`, which guards it with
/// `UpdateSlotError::SlotOverlapsWithNextDay` (`ops/src/slots.rs:481`) — but
/// that guard reads a `BrokenInvariants` error, so this is the trace it reads.
#[test]
fn rejection_1a_a_slot_moved_past_midnight_is_convicted_not_deleted() {
    let (app, subject, teacher, slot) = document_with_a_slot_ending_at_midnight();

    let mut data = app.get_data().clone();
    let before = data.get_inner_data().clone();

    let moved_slot = make_slot_at(subject, teacher, None, 23, 30);
    let (target, _new_info) = data.annotate(Op::Slot(SlotOp::Update(slot, moved_slot.clone())));

    let err = apply_cascade(&mut data, target)
        .expect_err("moving the slot past midnight must be refused, not repaired");

    assert_convicted_of(
        err,
        BTreeSet::from([FixableInvariant::Convergence(
            Convergence::SlotOverflowsDay {
                slot,
                start: moved_slot.start_time.clone(),
                duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
            },
        )]),
        "the target is convicted of exactly the overflow its own payload causes",
    );

    assert_eq!(
        data.get_inner_data(),
        &before,
        "a convicted target leaves the document bit-identical"
    );
    assert_eq!(
        data.get_inner_data().params.slots.find_slot(slot),
        Some(&make_slot_at(subject, teacher, None, 23, 0)),
        "and the innocent slot is still there, still at 23:00 — the arm did not delete it"
    );
}

/// Rejection `1b` — `SlotOverflowsDay`, **accepted**. The op lengthens the
/// interrogation.
///
/// The mirror of `1a`, on the same document, and the pair is worth more than
/// either half. Target: `SubjectOp::Update(subject, interrogations of 90
/// minutes)`, with the slot left at 23:00 — 90 minutes from 23:00 ends at
/// 00:30, so the *same* invariant fires on the *same* slot. But this time the
/// slot's `start_time` is exactly what the invariant names: the shape test
/// passes, the arm answers `Some(Slot::Remove(slot))`, the fix lands, and the
/// target lands after it.
///
/// Alone, `1a` shows only that the arm says `None` somewhere. Together the two
/// show that the `start` field *discriminates*: same invariant, same arm,
/// opposite verdict, and the only difference is which of the two operands the
/// op moved.
///
/// One fix lands, then the target; content rather than sequence is asserted per
/// this file's second rule: every round here reports exactly one break, so the
/// order is forced by the data and pinning it would pin depth, not choice.
///
/// This is also §8.2 row 4's only pin. That row has **no legacy behaviour to
/// compare against**: `ops/src/subjects.rs` does not read `BrokenInvariants` at
/// all on this route — it applies the update under
/// `.expect("All data should be valid at this point")` (`subjects.rs:758`, and
/// again at `:895`), so lengthening an interrogation over a late slot aborts
/// the process today. This fixture is what states the new answer. (The
/// neighbouring `ops/src/slots.rs` *does* match on the overflow, at `:481`,
/// which is `1a`'s route and a different story.)
#[test]
fn rejection_1b_a_lengthened_interrogation_removes_the_slot_it_overflows() {
    let (app, subject, _teacher, slot) = document_with_a_slot_ending_at_midnight();

    let mut data = app.get_data().clone();
    let longer_subject = interrogation_subject_lasting("Math", BTreeSet::new(), 90);

    let (target, _new_info) = data.annotate(Op::Subject(SubjectOp::Update(
        subject,
        longer_subject.clone(),
    )));

    let receipt = apply_cascade(&mut data, target)
        .expect("the arm removes the slot the lengthened interrogation overflows");

    // The overflow arm has its own meaning — the slot goes *because it would
    // spill over into the next day* — even though it emits the same op as
    // [Fix::DeleteSlot]. That distinction is what this fixture is about, so it
    // is asserted here rather than left to the translation.
    assert_same_fixes(
        &landed_fixes(&receipt),
        &[Fix::DeleteOverflowingSlot { slot }],
    );

    assert_clean(&data);
    let params = &data.get_inner_data().params;
    assert!(
        params.slots.find_slot(slot).is_none(),
        "the slot is gone — this is the arm's `Some` branch, the one `1a` refuses to take"
    );
    assert_eq!(
        params.subjects.find_subject(subject),
        Some(&longer_subject),
        "and the target landed: the interrogation really is 90 minutes now"
    );
}

/// Rejection `2` — `AssignedStudentNotPresentForPeriod`. The op assigns a
/// student who is not there.
///
/// A period `P`, a subject `S` that runs on it, an assignments row at `(P, S)`
/// already holding student `A`, and a student `B` who excludes `P`. Target:
/// `AssignmentOp::SetRow(P, S, {A, B})`. The op applies, the checker reports
/// `AssignedStudentNotPresentForPeriod { P, S, B }` (`invariants.rs:495-506`),
/// the gate rolls it back, and the arm is asked on the restored row — which
/// holds `A` alone. `row.contains(B)` is false, the arm answers `None`, and the
/// target is convicted.
///
/// **The construction obeys §9ter.2 — fail on the last conjunct.** The arm is a
/// two-step chain: the row exists, *and* it holds the named student. The row is
/// made to pre-exist, with a legitimate member in it, so the first step passes
/// and the second is the one that decides. Written the lazy way — no row at all
/// before the target — the *presence* step would fail instead and the test
/// would go green even for a map that had dropped the membership test entirely.
///
/// **The other trap is the subject.** `S` must genuinely run on `P`. If it
/// excluded `P`, `AssignmentForSubjectNotRunningOnPeriod` would fire too, and
/// it is declared *before* `AssignedStudentNotPresentForPeriod`
/// (`invariants.rs:483-506`), so the engine would pick it instead — its own
/// shape test passes on the pre-op state, a fix would land, and this fixture
/// would be testing a different trace while staying green.
///
/// `S` deliberately runs no interrogations: nothing in the assignments half of
/// the checker looks at them, and the row costs a subject and no more.
///
/// **What the failure would look like.** Not a silent repair, here. On today's
/// map the comparison above *is* how this arm honours the strict-monotonicity
/// contract: it finds no `B` to remove and returns `None`, so no fix op is
/// produced at all. Take the comparison away and the arm would fall through to
/// its `Some` branch, which rebuilds the row without the named student — and on
/// the restored row that rebuild is `{A}` again. That would be a perfect no-op
/// fix, which the engine treats as a map-contract violation and panics on
/// (`state/src/cascade.rs:87-95`). So an arm that lost this comparison would
/// crash rather than corrupt. That is the shape of every arm whose fix removes
/// one named element from a row, and it is why the assertion below is worth
/// having even though the damage would be loud.
#[test]
fn rejection_2_a_student_absent_from_the_period_convicts_the_assignment() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let period: PeriodId = apply_new!(
        app,
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "adding the period"
    );
    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            plain_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the subject, which runs on the period"
    );
    let present_student: StudentId = apply_new!(
        app,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::new()))),
        NewId::StudentId,
        "adding the student who is present for the period"
    );
    let absent_student: StudentId = apply_new!(
        app,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::from([period])))),
        NewId::StudentId,
        "adding the student who excludes the period"
    );
    apply_ok(
        &mut app,
        Op::Assignment(AssignmentOp::SetRow(
            period,
            subject,
            BTreeSet::from([present_student]),
        )),
        "filling the assignments row with the legitimate student",
    );

    let mut data = app.get_data().clone();
    let before = data.get_inner_data().clone();

    let (target, _new_info) = data.annotate(Op::Assignment(AssignmentOp::SetRow(
        period,
        subject,
        BTreeSet::from([present_student, absent_student]),
    )));

    let err = apply_cascade(&mut data, target)
        .expect_err("assigning an absent student must be refused, not repaired");

    assert_convicted_of(
        err,
        BTreeSet::from([FixableInvariant::Convergence(
            Convergence::AssignedStudentNotPresentForPeriod {
                period,
                subject,
                student: absent_student,
            },
        )]),
        "the target is convicted of exactly the absence its own payload introduces",
    );

    assert_eq!(
        data.get_inner_data(),
        &before,
        "a convicted target leaves the document bit-identical"
    );
    assert_eq!(
        data.get_inner_data()
            .params
            .assignments
            .students(period, subject),
        Some(&BTreeSet::from([present_student])),
        "and the innocent row is untouched — the present student is still assigned"
    );
}

/// Rejection `3` — `InterrogationGroupOutOfBounds`. The op writes a group
/// number no group list has.
///
/// A group list of **three** groups associated to `(P, S)`, and a colloscope
/// cell at `(slot, week)` already holding group `0`. Target:
/// `ColloscopeOp::SetInterrogation(slot, week, {0, 7})`. The op applies, the
/// bound read from the association is `3`, and the checker reports
/// `InterrogationGroupOutOfBounds(slot, week, 7)` (`invariants.rs:599-621`).
/// The gate rolls it back, and the arm is asked on the restored cell — which
/// holds `0` alone. `cell.contains(7)` is false, the arm answers `None`, and
/// the target is convicted.
///
/// §9ter.2 again, and it is the reason the cell pre-exists holding `0`: the arm
/// looks the cell up first and only then asks whether the named group is in it.
/// Writing `{7}` into a cell that did not exist would fail at the lookup and
/// prove nothing about the second step. Group `0` is also what keeps the
/// pre-op document valid, since a cell must be non-empty to exist at all.
///
/// The enrichment this fixture needs is **commit 5** — the offending group
/// number in the payload. Without it the invariant names only `(slot, week)`,
/// and an arm that can merely see a cell there has no way to tell this trace
/// from a legitimate one.
///
/// The counterfactual is the same as rejection `2`'s: were the `contains` test
/// gone, removing `7` from `{0}` would give `{0}` back — a perfect no-op fix,
/// and the engine's panic rather than a silent repair. Note also that the arm tests
/// **presence, not the bound** — deliberately, per its own comment
/// (`resolution.rs:623-626`): after a group-list shrink has itself been
/// repaired, the group can read as in-bounds again while still having to go.
/// So this fixture pins the comparison the arm really makes.
#[test]
fn rejection_3_an_out_of_bounds_group_convicts_the_interrogation() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let period: PeriodId = apply_new!(
        app,
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "adding the period"
    );
    let week: WeekId = apply_new!(
        app,
        Op::Week(WeekOp::AddFront(period, WeekDesc::default())),
        NewId::WeekId,
        "adding the week that carries the cell"
    );
    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the subject"
    );
    let teacher: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "adding the teacher"
    );
    let slot: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(None, make_slot(subject, teacher, None, 8))),
        NewId::SlotId,
        "adding the slot"
    );
    let group_list: GroupListId = apply_new!(
        app,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Liste",
            3,
            BTreeSet::new()
        ))),
        NewId::GroupListId,
        "adding the group list of three groups"
    );
    apply_ok(
        &mut app,
        Op::GroupList(GroupListOp::AssignToSubject(
            period,
            subject,
            Some(group_list),
        )),
        "associating the group list, which is what supplies the bound",
    );
    apply_ok(
        &mut app,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot,
            week,
            BTreeSet::from([0]),
        )),
        "filling the cell with a legitimate group",
    );

    let mut data = app.get_data().clone();
    let before = data.get_inner_data().clone();

    let (target, _new_info) = data.annotate(Op::Colloscope(ColloscopeOp::SetInterrogation(
        slot,
        week,
        BTreeSet::from([0, 7]),
    )));

    let err = apply_cascade(&mut data, target)
        .expect_err("writing an out-of-bounds group must be refused, not repaired");

    assert_convicted_of(
        err,
        BTreeSet::from([FixableInvariant::Convergence(
            Convergence::InterrogationGroupOutOfBounds(slot, week, 7),
        )]),
        "the target is convicted of exactly the group its own payload adds",
    );

    assert_eq!(
        data.get_inner_data(),
        &before,
        "a convicted target leaves the document bit-identical"
    );
    assert_eq!(
        data.get_inner_data().colloscope.interrogation(slot, week),
        Some(&BTreeSet::from([0])),
        "and the innocent cell is untouched — group 0 is still interrogated"
    );
}

/// Rejection `4` — `DanglingFk @ AssignmentsStudent`. The op assigns a student
/// who does not exist.
///
/// A period `P`, a subject `S` that runs on it, an assignments row at `(P, S)`
/// already holding the live student `A`, and a dead student id `D`
/// ([dead_student_id]). Target: `AssignmentOp::SetRow(P, S, {A, D})`. The op
/// applies — `force_apply_assignment` prechecks the row's *address*, the
/// `(period, subject)` key, and nothing about the payload — the checker reports
/// `DanglingFk(Student { D, AssignmentsStudent { P, S } })`, and the gate rolls
/// it back. The `AssignmentsStudent` arm is then asked on the restored row,
/// which holds `A` alone: `row.contains(D)` is false, the arm answers `None`,
/// and the target is convicted.
///
/// **Why this fixture exists.** Until the pre-step-7 review the state layer
/// swept `SetRow`'s payload students in its precheck, so this op was refused
/// one tier up, as `AssignmentPrecheckError::InvalidStudentId`, and never
/// reached the cascade at all. The sweep was removed on the address/content
/// split (design doc D.3): the op's *address* is prechecked, the ids the op
/// writes *into* the document are content and belong to the dangling-FK net —
/// the same tier `ColloscopeOp::SetGroupList`'s placements have always been
/// reported at. The two ops are now symmetric.
///
/// What that split needs proving is that it cannot produce a *spurious fix*:
/// a dead payload student must not make the cascade quietly repair something
/// and report success. It cannot, and the reason is structural rather than
/// lucky. The gate rolls the failing target back *before* the resolution map is
/// ever consulted, so the map only ever sees the pre-state — which is valid,
/// and therefore does not contain `D` anywhere. The arm's presence test
/// (`resolution.rs:376-383`) then answers `None`, and the engine convicts,
/// handing the user back the break the target itself introduced.
///
/// **§9ter.2 — fail on the last conjunct.** As in rejection `2`, the row is made
/// to pre-exist with a legitimate member in it. The arm is a two-step chain (the
/// row exists, *and* it holds the named student); with no row at all the first
/// step would fail and the fixture would stay green even for a map that had
/// dropped the membership test.
///
/// The expected set is a single element. `S` genuinely runs on `P`, so
/// `AssignmentForSubjectNotRunningOnPeriod` cannot fire; and
/// `AssignedStudentNotPresentForPeriod` sits behind `let Some(student) = …`
/// (`invariants.rs:495-498`), so a *dead* student makes it skip rather than
/// fire. The dangle arrives alone.
#[test]
fn rejection_4_an_unknown_student_convicts_the_assignment() {
    let mut app = AppState::<Data, String>::new(Data::new());

    let period: PeriodId = apply_new!(
        app,
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "adding the period"
    );
    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            plain_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the subject, which runs on the period"
    );
    let live_student: StudentId = apply_new!(
        app,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::new()))),
        NewId::StudentId,
        "adding the student who is legitimately assigned"
    );
    let dead_student = dead_student_id(&mut app);
    apply_ok(
        &mut app,
        Op::Assignment(AssignmentOp::SetRow(
            period,
            subject,
            BTreeSet::from([live_student]),
        )),
        "filling the assignments row with the legitimate student",
    );

    let mut data = app.get_data().clone();
    let before = data.get_inner_data().clone();

    let (target, _new_info) = data.annotate(Op::Assignment(AssignmentOp::SetRow(
        period,
        subject,
        BTreeSet::from([live_student, dead_student]),
    )));

    let err = apply_cascade(&mut data, target)
        .expect_err("assigning a student who does not exist must be refused, not repaired");

    assert_convicted_of(
        err,
        BTreeSet::from([FixableInvariant::DanglingFk(Reference::Student {
            target: dead_student,
            site: StudentRefSite::AssignmentsStudent { period, subject },
        })]),
        "the target is convicted of exactly the dangle its own payload introduces",
    );

    assert_eq!(
        data.get_inner_data(),
        &before,
        "a convicted target leaves the document bit-identical"
    );
    assert_eq!(
        data.get_inner_data()
            .params
            .assignments
            .students(period, subject),
        Some(&BTreeSet::from([live_student])),
        "and the innocent row is untouched — the live student is still assigned"
    );
}

/// The incompatibility the identity-pin document carries, as a value, so that
/// the fixture and its assertion cannot drift apart.
fn identity_pin_incompat(subject_id: SubjectId) -> Incompatibility {
    Incompatibility {
        subject_id,
        name: "Sport".into(),
        slots: vec![],
        minimum_free_slots: NonZeroU32::new(2).unwrap(),
        week_pattern_id: None,
    }
}

/// A `TeacherId` that is not live.
///
/// An integration test cannot fabricate one: the id types are opaque and carry
/// no public constructor. The route is **create-then-remove** — add a teacher
/// nothing references, remove it, keep the id. The removal cascades to nothing
/// and lands alone. Three lines, but it is the step someone reading the plan
/// would otherwise stall on.
fn dead_teacher_id(app: &mut AppState<Data, String>) -> TeacherId {
    let id: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::new(),
        })),
        NewId::TeacherId,
        "adding the teacher that is about to die"
    );
    apply_ok(app, Op::Teacher(TeacherOp::Remove(id)), "killing it again");
    id
}

/// A `StudentId` that is not live. [dead_teacher_id]'s recipe, other type.
fn dead_student_id(app: &mut AppState<Data, String>) -> StudentId {
    let id: StudentId = apply_new!(
        app,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::new()))),
        NewId::StudentId,
        "adding the student that is about to die"
    );
    apply_ok(app, Op::Student(StudentOp::Remove(id)), "killing it again");
    id
}

/// A `SubjectId` that is not live. [dead_teacher_id]'s recipe, other type.
fn dead_subject_id(app: &mut AppState<Data, String>) -> SubjectId {
    let id: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            plain_subject("Éphémère", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the subject that is about to die"
    );
    apply_ok(app, Op::Subject(SubjectOp::Remove(id)), "killing it again");
    id
}

/// Everything the four collateral-damage identity pins need, in one document.
///
/// Each pin points one row at a dead id and asserts the row survives. Sharing
/// the document costs nothing — no pin's edit is visible to another's row — and
/// buys a little: the "document unchanged" assertion then covers the rows the
/// other three pins care about as well.
struct IdentityPinDocument {
    subject: SubjectId,
    other_subject: SubjectId,
    teacher: TeacherId,
    slot: SlotId,
    incompat: IncompatId,
    rule: PairingRuleId,
    dead_teacher: TeacherId,
    dead_subject: SubjectId,
}

fn build_identity_pin_document(app: &mut AppState<Data, String>) -> IdentityPinDocument {
    let subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Math", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the subject that hosts the slot"
    );
    let other_subject: SubjectId = apply_new!(
        app,
        Op::Subject(SubjectOp::AddAfter(
            Some(subject),
            plain_subject("Sport", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the pairing rule's second subject"
    );
    let teacher: TeacherId = apply_new!(
        app,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "adding the slot's real teacher"
    );
    let slot: SlotId = apply_new!(
        app,
        Op::Slot(SlotOp::AddAfter(None, make_slot(subject, teacher, None, 8))),
        NewId::SlotId,
        "adding the slot"
    );
    let incompat: IncompatId = apply_new!(
        app,
        Op::Incompat(IncompatOp::Add(identity_pin_incompat(subject))),
        NewId::IncompatId,
        "adding the incompatibility"
    );
    let rule: PairingRuleId = apply_new!(
        app,
        Op::Pairing(PairingOp::Add(pairing_rule(
            subject,
            other_subject,
            BTreeSet::new()
        ))),
        NewId::PairingRuleId,
        "adding the pairing rule"
    );

    let dead_teacher = dead_teacher_id(app);
    let dead_subject = dead_subject_id(app);

    IdentityPinDocument {
        subject,
        other_subject,
        teacher,
        slot,
        incompat,
        rule,
        dead_teacher,
        dead_subject,
    }
}

/// Identity pin `1` — a slot pointed at a **dead teacher**.
///
/// Target: `SlotOp::Update(slot, the same slot with a dead `teacher_id`)`. The
/// op really lands — `force_apply_slot`'s `Update` has no teacher-existence
/// guard (`slots.rs:455-483`) — the checker reports the dangle, and the gate
/// rolls it back. The `SlotTeacher` arm is then asked on the restored document,
/// where the slot's teacher is the live one. `slot.teacher_id != dead`, so it
/// answers `None` and the target is convicted.
///
/// **The assertion that carries the weight is not the `Err`; it is that the
/// slot is still there.** Suppose the arm skipped its identity test and
/// answered `Some(Slot::Remove(slot))` merely because the slot exists. A user
/// pointing a slot at a teacher who no longer exists — a stale UI view, or a
/// script racing another edit — would have the slot deleted, its own live
/// teacher being perfectly fine, and the target would land afterwards, so the
/// operation would report success. That is the damage this pin exists for.
///
/// The expected set is a single element, and the reason is in the checker: the
/// teacher-teaches predicate sits behind `if let Some(teacher) = …`
/// (`invariants.rs:433`), so a *dead* teacher makes it skip rather than fire.
/// `SlotTeacherDoesNotTeachSubject` does not accompany the dangle.
#[test]
fn identity_pin_1_a_slot_pointed_at_a_dead_teacher_keeps_its_live_one() {
    let mut app = AppState::<Data, String>::new(Data::new());
    let doc = build_identity_pin_document(&mut app);

    let mut data = app.get_data().clone();
    let before = data.get_inner_data().clone();

    let mut broken_slot = make_slot(doc.subject, doc.teacher, None, 8);
    broken_slot.teacher_id = doc.dead_teacher;
    let (target, _new_info) = data.annotate(Op::Slot(SlotOp::Update(doc.slot, broken_slot)));

    let err = apply_cascade(&mut data, target)
        .expect_err("a slot cannot be pointed at a teacher who does not exist");

    assert_convicted_of(
        err,
        BTreeSet::from([FixableInvariant::DanglingFk(Reference::Teacher {
            target: doc.dead_teacher,
            site: TeacherRefSite::SlotTeacher(doc.slot),
        })]),
        "exactly the dangle the target introduces, and nothing about the live teacher",
    );

    assert_eq!(
        data.get_inner_data(),
        &before,
        "a convicted target leaves the document bit-identical"
    );
    assert_eq!(
        data.get_inner_data().params.slots.find_slot(doc.slot),
        Some(&make_slot(doc.subject, doc.teacher, None, 8)),
        "and the innocent slot is still there, still on its live teacher — not deleted"
    );
}

/// Identity pin `2` — an incompatibility pointed at a **dead subject**.
///
/// Target: `IncompatOp::Update(incompat, the same row with a dead
/// `subject_id`)`. `force_apply_incompat`'s `Update` replaces the row with no
/// field guards (`incompats.rs:108-124`), so the bad op lands, the checker
/// reports the dangle, and the gate rolls it back. On the restored document the
/// row's subject is the live one, so the `IncompatSubject` arm answers `None`
/// rather than removing an innocent incompatibility.
///
/// One break, and here the reason is simpler than pin `1`'s: **no `Convergence`
/// variant mentions an incompatibility at all**, so layer C has nothing to add.
#[test]
fn identity_pin_2_an_incompat_pointed_at_a_dead_subject_survives() {
    let mut app = AppState::<Data, String>::new(Data::new());
    let doc = build_identity_pin_document(&mut app);

    let mut data = app.get_data().clone();
    let before = data.get_inner_data().clone();

    let (target, _new_info) = data.annotate(Op::Incompat(IncompatOp::Update(
        doc.incompat,
        identity_pin_incompat(doc.dead_subject),
    )));

    let err = apply_cascade(&mut data, target)
        .expect_err("an incompatibility cannot be pointed at a subject that does not exist");

    assert_convicted_of(
        err,
        BTreeSet::from([FixableInvariant::DanglingFk(Reference::Subject {
            target: doc.dead_subject,
            site: SubjectRefSite::IncompatSubject(doc.incompat),
        })]),
        "exactly the dangle the target introduces",
    );

    assert_eq!(
        data.get_inner_data(),
        &before,
        "a convicted target leaves the document bit-identical"
    );
    assert_eq!(
        data.get_inner_data()
            .params
            .incompats
            .incompat_map
            .get(&doc.incompat),
        Some(&identity_pin_incompat(doc.subject)),
        "and the innocent incompatibility is still there, still on its live subject"
    );
}

/// Identity pin `3` — a pairing rule whose **antecedent** points at a dead
/// subject.
///
/// Target: `PairingOp::Update(rule, the same rule with a dead antecedent
/// subject)`. `force_apply_pairing`'s `Update` has no field guards either
/// (`pairings.rs:237-247`). The rule is built through
/// `PairingRule::new(...).expect(..)` — the sealed constructor is the only
/// door, and it accepts this payload because its single failure is the two
/// parts *sharing* a subject, which a dead id on one side cannot cause.
///
/// One break: no `Convergence` variant mentions a `PairingRule`. (The one that
/// sounds close, `PairedSlotsNotInSameSubject`, is about *slot* pairings, a
/// different table.)
#[test]
fn identity_pin_3_a_pairing_antecedent_pointed_at_a_dead_subject_survives() {
    let mut app = AppState::<Data, String>::new(Data::new());
    let doc = build_identity_pin_document(&mut app);

    let mut data = app.get_data().clone();
    let before = data.get_inner_data().clone();

    let (target, _new_info) = data.annotate(Op::Pairing(PairingOp::Update(
        doc.rule,
        pairing_rule(doc.dead_subject, doc.other_subject, BTreeSet::new()),
    )));

    let err = apply_cascade(&mut data, target)
        .expect_err("a pairing antecedent cannot name a subject that does not exist");

    assert_convicted_of(
        err,
        BTreeSet::from([FixableInvariant::DanglingFk(Reference::Subject {
            target: doc.dead_subject,
            site: SubjectRefSite::PairingRuleAntecedent(doc.rule),
        })]),
        "exactly the antecedent dangle, and nothing about the untouched consequent",
    );

    assert_eq!(
        data.get_inner_data(),
        &before,
        "a convicted target leaves the document bit-identical"
    );
    assert_eq!(
        data.get_inner_data()
            .params
            .pairings
            .pairing_rule_map
            .get(&doc.rule),
        Some(&pairing_rule(
            doc.subject,
            doc.other_subject,
            BTreeSet::new()
        )),
        "and the innocent rule is still there, both parts on their live subjects"
    );
}

/// Identity pin `4` — a pairing rule whose **consequent** points at a dead
/// subject. The mirror of pin `3`, and the pair is the point.
///
/// §8.1 insists the antecedent and the consequent are **two arms, not one**.
/// The first draft of this plan had only pin `3`, described as "the antecedent
/// arm returns `None`, and the consequent arm must not fire at all". The second
/// half of that sentence is not an assertion a test can make: with only the
/// antecedent's subject dead the checker reports only the antecedent site, so
/// the consequent arm is never called and pin `3` tests it in no way
/// whatsoever. This mirror is what makes "two arms" true of the test suite as
/// well, and it costs three lines.
#[test]
fn identity_pin_4_a_pairing_consequent_pointed_at_a_dead_subject_survives() {
    let mut app = AppState::<Data, String>::new(Data::new());
    let doc = build_identity_pin_document(&mut app);

    let mut data = app.get_data().clone();
    let before = data.get_inner_data().clone();

    let (target, _new_info) = data.annotate(Op::Pairing(PairingOp::Update(
        doc.rule,
        pairing_rule(doc.subject, doc.dead_subject, BTreeSet::new()),
    )));

    let err = apply_cascade(&mut data, target)
        .expect_err("a pairing consequent cannot name a subject that does not exist");

    assert_convicted_of(
        err,
        BTreeSet::from([FixableInvariant::DanglingFk(Reference::Subject {
            target: doc.dead_subject,
            site: SubjectRefSite::PairingRuleConsequent(doc.rule),
        })]),
        "exactly the consequent dangle — the site the mirror exists to reach",
    );

    assert_eq!(
        data.get_inner_data(),
        &before,
        "a convicted target leaves the document bit-identical"
    );
    assert_eq!(
        data.get_inner_data()
            .params
            .pairings
            .pairing_rule_map
            .get(&doc.rule),
        Some(&pairing_rule(
            doc.subject,
            doc.other_subject,
            BTreeSet::new()
        )),
        "and the innocent rule is still there, both parts on their live subjects"
    );
}
