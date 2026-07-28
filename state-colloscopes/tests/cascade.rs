//! Colloscope cascade fixtures (step-6 commit 7, plan §9).
//!
//! Each fixture builds a document through the public op surface
//! (`AppState` + `Manager::apply`), then takes `app.get_data().clone()`,
//! annotates the target op itself through `Data::annotate`, and drives
//! [collomatique_state::apply_cascade] on that [Data] directly. What is under
//! test is the resolution map (`src/resolution.rs`) driven by the real engine,
//! not the `AppState` surface.
//!
//! Two rules govern the assertions in this file.
//!
//! **The expected op list is derived on paper first.** Every fixture asserts
//! something about the ops that landed, and that list comes from the plan's
//! §8.1 / §8.2 arm tables *before* the test is ever run. A difference between
//! the hand-derived list and what the engine produced is a finding to explain —
//! possibly a map bug — never a value to paste back in.
//!
//! **Sequence versus content.** Asserting the *order* of the landed ops is only
//! meaningful where the engine actually made a choice, i.e. where one failing
//! apply reported more than one broken invariant and the engine picked
//! `set.first()` out of the `BTreeSet`. Fixture `1a` is the one that pins that
//! canonical pick order; every later fixture asserts content (length plus
//! `contains`) and is deliberately blind to order.

use collomatique_state::{AppState, InMemoryData, apply_cascade, traits::Manager};
use collomatique_state_colloscopes::{
    ColloscopeOp, Data, GroupListOp, NewId, NonEmptyRangeInclusive, Op, PairingOp, PeriodOp,
    SlotOp, SlotPairingOp, StudentOp, Subject, SubjectInterrogationParameters, SubjectOp,
    SubjectParameters, SubjectPeriodicity, TeacherOp, WeekOp, WeekPatternOp,
    group_lists::{GroupList, GroupListFilling, GroupListParameters},
    ids::{
        GroupListId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId, SubjectId,
        TeacherId, WeekId, WeekPatternId,
    },
    ops::{
        AnnotatedAssignmentOp, AnnotatedColloscopeOp, AnnotatedGroupListOp, AnnotatedOp,
        AnnotatedPairingOp, AnnotatedPeriodOp, AnnotatedSlotOp, AnnotatedSlotPairingOp,
        AnnotatedStudentOp, AnnotatedSubjectOp, AnnotatedTeacherOp, AnnotatedWeekOp,
        AnnotatedWeekPatternOp,
    },
    pairings::{PairingRule, RulePart},
    slot_pairings::{SlotPairingRule, SlotRulePart},
    slots::Slot,
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
fn interrogation_subject(name: &str, excluded: BTreeSet<PeriodId>) -> Subject {
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
    Slot {
        subject_id,
        teacher_id,
        start_time: collomatique_time::SlotStart {
            weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
            start_time: collomatique_time::WholeMinuteTime::new(
                chrono::NaiveTime::from_hms_opt(hour, 0, 0).unwrap(),
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

/// The forward op of every landed step, in order.
fn forward_ops(
    applied: &collomatique_state::history::AggregatedOp<AnnotatedOp>,
) -> Vec<AnnotatedOp> {
    applied.inner().iter().map(|r| r.inner().clone()).collect()
}

/// Content, not sequence: the same ops landed, in any order.
///
/// Length plus `contains` catches an extra, a missing and a wrong op. The one
/// case it misses — a duplicate paired with an omission — cannot occur, since a
/// fix landing twice would be a perfect no-op and the engine panics on that.
fn assert_same_ops(actual: &[AnnotatedOp], expected: &[AnnotatedOp]) {
    for op in expected {
        assert!(
            actual.contains(op),
            "expected op never landed: {op:#?}\nlanded: {actual:#?}"
        );
    }
    assert_eq!(
        actual.len(),
        expected.len(),
        "landed op count\nlanded: {actual:#?}"
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

    let applied = apply_cascade(&mut data, target).expect("the cascade resolves both breaks");

    assert_eq!(
        forward_ops(&applied),
        vec![
            AnnotatedOp::from(AnnotatedSubjectOp::Update(
                subject,
                plain_subject("Math", BTreeSet::new()),
            )),
            AnnotatedOp::from(AnnotatedStudentOp::Update(
                student,
                plain_student(BTreeSet::new()),
            )),
            AnnotatedOp::from(AnnotatedPeriodOp::Remove(period)),
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

    let applied = apply_cascade(&mut data, target).expect("the cascade resolves the chain");

    assert_eq!(
        forward_ops(&applied),
        vec![
            AnnotatedOp::from(AnnotatedColloscopeOp::SetInterrogation(
                slot,
                week,
                BTreeSet::new(),
            )),
            AnnotatedOp::from(AnnotatedWeekOp::Remove(week)),
            AnnotatedOp::from(AnnotatedGroupListOp::AssignToSubject(period, subject, None,)),
            AnnotatedOp::from(AnnotatedPeriodOp::Remove(period)),
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
/// declaration order. `1c` lands exactly these; `1e` lands these plus the ops
/// of the two sub-cascades.
fn seven_flat_period_fixes(doc: &PeriodDocument, week: WeekId) -> Vec<AnnotatedOp> {
    vec![
        AnnotatedOp::from(AnnotatedWeekOp::Remove(week)),
        AnnotatedOp::from(AnnotatedSubjectOp::Update(
            doc.excluded_subject,
            plain_subject("Sport", BTreeSet::new()),
        )),
        AnnotatedOp::from(AnnotatedStudentOp::Update(
            doc.excluded_student,
            plain_student(BTreeSet::new()),
        )),
        AnnotatedOp::from(AnnotatedPairingOp::Update(
            doc.pairing,
            pairing_rule(doc.subject, doc.excluded_subject, BTreeSet::new()),
        )),
        AnnotatedOp::from(AnnotatedSlotPairingOp::Update(
            doc.slot_pairing,
            slot_pairing_rule(doc.slots[0], doc.slots[1], BTreeSet::new()),
        )),
        AnnotatedOp::from(AnnotatedAssignmentOp::SetRow(
            doc.period,
            doc.subject,
            BTreeSet::new(),
        )),
        AnnotatedOp::from(AnnotatedGroupListOp::AssignToSubject(
            doc.period,
            doc.subject,
            None,
        )),
    ]
}

/// Fixture `1c` — **breadth**. A period referenced from all seven of its sites
/// at once, every fix hanging flat off the target.
///
/// Note what this does *not* catch: an arm missing from the map is a compile
/// error already, since `fix_period_ref` matches `PeriodRefSite` totally with
/// no wildcard. What it catches is an arm that wrongly answers `None`, or emits
/// the wrong op — including the two sealed rebuilds (`PairingRule` and
/// `SlotPairingRule`), whose `.expect` is exercised here for the first time.
#[test]
fn fixture_1c_all_seven_period_sites_are_repaired() {
    let mut app = AppState::<Data, String>::new(Data::new());
    let doc = build_period_document(&mut app, false);

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::Period(PeriodOp::Remove(doc.period)));

    let applied = apply_cascade(&mut data, target).expect("the cascade resolves all seven sites");

    let mut expected = seven_flat_period_fixes(&doc, doc.weeks[0]);
    expected.push(AnnotatedOp::from(AnnotatedPeriodOp::Remove(doc.period)));
    assert_same_ops(&forward_ops(&applied), &expected);

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
/// colloscope row placing `st` at group 1. Per §8.2 both arms emit
/// `SetGroupList(gl, placements minus st)`.
///
/// The point of the fixture is the *second* half: whichever break is picked,
/// the one fix kills both, the retry succeeds, and no second fix is ever
/// requested. A redundant second fix would apply as a perfect no-op and the
/// engine would panic — so "exactly two ops landed" is the real assertion here.
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

    let applied = apply_cascade(&mut data, target).expect("the cascade resolves both breaks");

    assert_same_ops(
        &forward_ops(&applied),
        &[
            AnnotatedOp::from(AnnotatedColloscopeOp::SetGroupList(
                group_list,
                BTreeMap::new(),
            )),
            AnnotatedOp::from(AnnotatedGroupListOp::Update(group_list, shrunk.clone())),
        ],
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
/// Eleven ops. Content, not sequence: round 1 makes a genuine pick and pinning
/// that pick is `1a`'s job.
#[test]
fn fixture_1e_the_flagship_period_removal() {
    let mut app = AppState::<Data, String>::new(Data::new());
    let doc = build_period_document(&mut app, true);
    let pattern = doc.week_pattern.expect("built with depth");

    let mut data = app.get_data().clone();
    let (target, _new_info) = data.annotate(Op::Period(PeriodOp::Remove(doc.period)));

    let applied =
        apply_cascade(&mut data, target).expect("the cascade resolves the whole document");

    let mut expected = seven_flat_period_fixes(&doc, doc.weeks[0]);
    expected.extend([
        // The first week's sub-cascade.
        AnnotatedOp::from(AnnotatedColloscopeOp::SetInterrogation(
            doc.slots[0],
            doc.weeks[0],
            BTreeSet::new(),
        )),
        // The second week, and its own sub-cascade.
        AnnotatedOp::from(AnnotatedWeekPatternOp::Update(
            pattern,
            WeekPattern {
                name: "skip the second week".into(),
                excluded_weeks: BTreeSet::new(),
            },
        )),
        AnnotatedOp::from(AnnotatedWeekOp::Remove(doc.weeks[1])),
        AnnotatedOp::from(AnnotatedPeriodOp::Remove(doc.period)),
    ]);
    assert_same_ops(&forward_ops(&applied), &expected);

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
/// goes, the slot goes, and the teacher lands. Six ops. Which of the two slots
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

    let applied = apply_cascade(&mut data, target).expect("the cascade resolves both fan-outs");

    let expected = vec![
        AnnotatedOp::from(AnnotatedSlotPairingOp::Remove(rule_ac)),
        AnnotatedOp::from(AnnotatedColloscopeOp::SetInterrogation(
            slot_a,
            week,
            BTreeSet::new(),
        )),
        AnnotatedOp::from(AnnotatedSlotOp::Remove(slot_a)),
        AnnotatedOp::from(AnnotatedSlotPairingOp::Remove(rule_cb)),
        AnnotatedOp::from(AnnotatedSlotOp::Remove(slot_b)),
        AnnotatedOp::from(AnnotatedTeacherOp::Remove(teacher)),
    ];
    assert_same_ops(&forward_ops(&applied), &expected);

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
