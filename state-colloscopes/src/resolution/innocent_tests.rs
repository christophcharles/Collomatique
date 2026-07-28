//! Innocent-state `None` tests (step-6 commit 7.5, plan §9bis).
//!
//! One test per **arm** of the resolution map, all of the same shape. Each asks
//! the single question frame point 5 asks: *given an invariant that names a
//! shape, does the arm keep its hands off a live document that does not carry
//! that shape?*
//!
//! The four steps, in order:
//!
//! 1. A **valid** document, built through the op surface — every op goes through
//!    the apply/check/rollback gate, so validity is by construction.
//! 2. Its **corrupted twin**, carrying exactly the offending shape. The twin is
//!    an [InnerData], never a [Data]: [Data::from_inner_data] validates and
//!    would reject it.
//! 3. The invariant, **derived from the twin** rather than hand-written, so the
//!    test cannot drift away from the checker. The expected literal is still
//!    hand-derived and `assert_eq!`d on the *whole* set: that pins the
//!    corruption as surgical — one edit, one broken shape.
//! 4. The point: the *valid* document holds nothing that causes it, so the arm
//!    answers `None`.
//!
//! Nothing here runs the engine. There is no `apply_cascade`, no rejection
//! semantics and no rollback reasoning — the end-to-end counterparts live in
//! `tests/cascade.rs`. And nothing here tests the *positive* half either (the
//! arm firing when the shape really is live); that belongs to the commit-7
//! scenarios, which reach it by the legitimate route.
//!
//! **Why the twin is built by field surgery and not by an op.** `force_apply`
//! keeps the coordinate carve-out prechecks, so several of these shapes are
//! simply not reachable through an op at all (`CannotChangeSubject` blocks a
//! slot's subject, the forced week ops block a dead period, `AssignToSubject`
//! checks the group list whenever the payload is `Some`). A recipe that works
//! for some arms and not others would drag a per-arm argument into every test;
//! surgery works for all of them uniformly. Nothing is *applied* here either,
//! so no gate and no id issuer are involved.
//!
//! **Two containers must be surgered through their `pub(crate)` mutators rather
//! than their raw fields.** [crate::slots::Slots] and [crate::weeks::Weeks] both
//! carry a type-level ordering mirror, and a desynced mirror is a `LogicError`
//! that short-circuits `broken_invariants()` — a twin built by hand would die at
//! step 3 with `SlotOrderingWrongSubject` instead of yielding the invariant
//! under test. The compound mutators keep the mirror consistent, and the
//! ordering sidecar's row *keys* are deliberately not liveness-checked, which is
//! exactly the hole these twins need.

use std::collections::{BTreeMap, BTreeSet};

use collomatique_state::{Fixable, InMemoryData};

use crate::balancing::BalancingOptions;
use crate::group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup};
use crate::ids::{
    GroupListId, IncompatId, NewId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekId, WeekPatternId,
};
use crate::incompats::Incompatibility;
use crate::invariants::{Convergence, FixableInvariant};
use crate::ops::{
    AssignmentOp, BalancingOp, ColloscopeOp, GroupListOp, IncompatOp, Op, PairingOp, PeriodOp,
    SettingsOp, SlotOp, SlotPairingOp, StudentOp, SubjectOp, TeacherOp, WeekOp, WeekPatternOp,
};
use crate::pairings::{PairingRule, RulePart};
use crate::refs::{
    GroupListRefSite, PeriodRefSite, Reference, SlotRefSite, StudentRefSite, SubjectRefSite,
    TeacherRefSite, WeekPatternRefSite, WeekRefSite,
};
use crate::settings::Limits;
use crate::slot_pairings::{SlotPairingRule, SlotRulePart};
use crate::slots::Slot;
use crate::soft_param::SoftParam;
use crate::students::Student;
use crate::subjects::{
    Subject, SubjectInterrogationParameters, SubjectParameters, SubjectPeriodicity,
};
use crate::teachers::Teacher;
use crate::week_patterns::WeekPattern;
use crate::weeks::WeekDesc;
use crate::{Data, InnerData, NonEmptyRangeInclusive};

// ---- Building the valid document ----

/// Annotates an op and pushes it through the gate, returning the id it issued
/// (if any). A rejection is a bug in the fixture, never something a test
/// tolerates: it panics with the op that failed.
fn apply(data: &mut Data, op: Op, what: &str) -> Option<NewId> {
    let (annotated, new_id) = data.annotate(op);
    if let Err(e) = data.apply(&annotated) {
        panic!("{what} should apply, got {e:?}");
    }
    new_id
}

/// [apply] for an op that must create a fresh entity, unwrapping its new id.
macro_rules! apply_new {
    ($data:expr, $op:expr, $variant:path, $what:expr) => {
        match apply(&mut $data, $op, $what) {
            Some($variant(id)) => id,
            other => panic!("{} should return a fresh id, got {:?}", $what, other),
        }
    };
}

/// A subject that runs interrogations, so it can host slots, an association, a
/// balancing entry and colloscope cells.
fn interrogation_subject(name: &str, excluded: BTreeSet<PeriodId>) -> Subject {
    Subject {
        parameters: SubjectParameters {
            name: name.into(),
            interrogation_parameters: Some(SubjectInterrogationParameters {
                students_per_group: NonEmptyRangeInclusive::new(
                    std::num::NonZeroU32::new(2).unwrap()..=std::num::NonZeroU32::new(3).unwrap(),
                )
                .expect("statically non-empty"),
                groups_per_interrogation: NonEmptyRangeInclusive::new(
                    std::num::NonZeroU32::new(1).unwrap()..=std::num::NonZeroU32::new(1).unwrap(),
                )
                .expect("statically non-empty"),
                duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
                take_duration_into_account: true,
                periodicity: SubjectPeriodicity::ExactlyPeriodic {
                    periodicity_in_weeks: std::num::NonZeroU32::new(2).unwrap(),
                },
            }),
        },
        excluded_periods: excluded,
    }
}

/// A subject with no interrogations. It cannot host a slot, an association or a
/// balancing entry, so it stays inert: the cheapest way to own a
/// `SubjectExcludedPeriods` reference and nothing else.
fn plain_subject(name: &str, excluded: BTreeSet<PeriodId>) -> Subject {
    Subject {
        parameters: SubjectParameters {
            name: name.into(),
            interrogation_parameters: None,
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

/// A slot starting at `hour:00`, well clear of the end of the day (the subjects
/// above last an hour, so `SlotOverflowsDay` never fires).
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

/// The ids of [build_valid_document]'s entities.
///
/// The `dead_*` fields are ids that were issued and then removed again, so they
/// are of the right type and provably not live. That is the recipe every test
/// here needs: the id types are opaque, and forging one risks colliding with a
/// live entity.
//
// The whole struct is `allow(dead_code)` for the duration of the 7.5 series:
// each commit reads the handful of fields its arms need, and the document is
// deliberately built whole rather than grown per commit. The attribute comes
// off once the series has landed.
#[allow(dead_code)]
pub(super) struct ValidDocument {
    /// The active period: it holds the weeks, the assignments row, the
    /// association and the colloscope cell.
    period: PeriodId,
    /// The period the exclusion sets name. Keeping the exclusions off `period`
    /// is what lets those rows be innocent without making the rest inert.
    other_period: PeriodId,
    /// Active for `slot`, and carrying the colloscope cell.
    week: WeekId,
    /// Excluded by `week_pattern`.
    other_week: WeekId,
    /// Runs interrogations; hosts both slots, the association, the assignments
    /// row, the incompatibility and the balancing override.
    subject: SubjectId,
    /// Runs interrogations too, and is the pairing rule's consequent.
    other_subject: SubjectId,
    /// Excludes `other_period`, and runs no interrogations.
    excluded_subject: SubjectId,
    /// Teaches `subject`, and is the teacher of both slots.
    teacher: TeacherId,
    /// Excludes `other_week`; worn by `slot` and by `incompat`.
    week_pattern: WeekPatternId,
    /// Wears `week_pattern`, and carries the colloscope cell on `week`.
    slot: SlotId,
    /// Same subject as `slot` (the slot pairing rule demands it), no pattern.
    other_slot: SlotId,
    /// Same subject again, but referenced by nothing else at all: no pattern, no
    /// colloscope cell, no pairing rule. The slot to corrupt when the corruption
    /// changes what a slot *is*.
    lone_slot: SlotId,
    incompat: IncompatId,
    pairing: PairingRuleId,
    slot_pairing: SlotPairingRuleId,
    /// In the prefilled list, the assignments row and the colloscope placements,
    /// and the holder of the per-student settings override.
    student: StudentId,
    /// The bystander: everywhere `student` is, so a rebuild has something to
    /// keep.
    other_student: StudentId,
    /// Excludes `other_period`, and is excluded by `excluding_group_list`.
    excluded_student: StudentId,
    /// Automatic, associated to `(period, subject)`, and carrying the colloscope
    /// placements row.
    group_list: GroupListId,
    /// Prefilled, holding both students.
    prefilled_group_list: GroupListId,
    /// Automatic, excluding `excluded_student`.
    excluding_group_list: GroupListId,

    dead_period: PeriodId,
    dead_week: WeekId,
    dead_subject: SubjectId,
    dead_teacher: TeacherId,
    dead_student: StudentId,
    dead_week_pattern: WeekPatternId,
    dead_slot: SlotId,
    dead_group_list: GroupListId,
}

/// The document shared by every test in this module.
///
/// It is a single valid state holding, at least once, the innocent counterpart
/// of every shape §8.1 and §8.2 talk about: a week in a live period, exclusion
/// sets naming a live period, a slot on a live subject with a live teacher, a
/// colloscope cell on a live `(slot, week)`, and so on. A test then corrupts
/// **one** of those rows in a throwaway clone and asks the untouched document
/// about the resulting invariant.
///
/// It is valid by construction: every op below goes through the gate, which
/// rejects anything that would break an invariant.
pub(super) fn build_valid_document() -> (Data, ValidDocument) {
    let mut data = Data::default();

    let period = apply_new!(
        data,
        Op::Period(PeriodOp::AddFront),
        NewId::PeriodId,
        "adding the active period"
    );
    let other_period = apply_new!(
        data,
        Op::Period(PeriodOp::AddAfter(period)),
        NewId::PeriodId,
        "adding the excluded period"
    );
    let week = apply_new!(
        data,
        Op::Week(WeekOp::AddFront(period, WeekDesc::default())),
        NewId::WeekId,
        "adding the first week"
    );
    let other_week = apply_new!(
        data,
        Op::Week(WeekOp::AddAfter(week, WeekDesc::default())),
        NewId::WeekId,
        "adding the second week"
    );

    let subject = apply_new!(
        data,
        Op::Subject(SubjectOp::AddAfter(
            None,
            interrogation_subject("Maths", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the running subject"
    );
    let other_subject = apply_new!(
        data,
        Op::Subject(SubjectOp::AddAfter(
            Some(subject),
            interrogation_subject("Physique", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the second running subject"
    );
    let excluded_subject = apply_new!(
        data,
        Op::Subject(SubjectOp::AddAfter(
            Some(other_subject),
            plain_subject("Sport", BTreeSet::from([other_period]))
        )),
        NewId::SubjectId,
        "adding the excluding subject"
    );
    let teacher = apply_new!(
        data,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([subject]),
        })),
        NewId::TeacherId,
        "adding the teacher"
    );

    // The pattern excludes the *second* week, so the colloscope cell below —
    // which sits on the first — stays on an active week.
    let week_pattern = apply_new!(
        data,
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "sauf la deuxième semaine".into(),
            excluded_weeks: BTreeSet::from([other_week]),
        })),
        NewId::WeekPatternId,
        "adding the week pattern"
    );
    // Two slots on the same subject: the slot pairing rule needs that
    // (`PairedSlotsNotInSameSubject`).
    let slot = apply_new!(
        data,
        Op::Slot(SlotOp::AddAfter(
            None,
            make_slot(subject, teacher, Some(week_pattern), 8)
        )),
        NewId::SlotId,
        "adding the patterned slot"
    );
    let other_slot = apply_new!(
        data,
        Op::Slot(SlotOp::AddAfter(
            Some(slot),
            make_slot(subject, teacher, None, 10)
        )),
        NewId::SlotId,
        "adding the second slot"
    );
    // A third slot, deliberately in nothing: no week pattern, no colloscope
    // cell, no pairing rule. Moving *this* slot to another subject breaks only
    // what the move is about; moving either of the two above would drag in
    // `PairedSlotsNotInSameSubject` and the cell's group bound as well.
    let lone_slot = apply_new!(
        data,
        Op::Slot(SlotOp::AddAfter(
            Some(other_slot),
            make_slot(subject, teacher, None, 16)
        )),
        NewId::SlotId,
        "adding the lone slot"
    );
    let incompat = apply_new!(
        data,
        Op::Incompat(IncompatOp::Add(Incompatibility {
            subject_id: subject,
            name: "Sport".into(),
            slots: vec![],
            minimum_free_slots: std::num::NonZeroU32::new(2).unwrap(),
            week_pattern_id: Some(week_pattern),
        })),
        NewId::IncompatId,
        "adding the incompatibility"
    );

    let pairing = apply_new!(
        data,
        Op::Pairing(PairingOp::Add(pairing_rule(
            subject,
            other_subject,
            BTreeSet::from([other_period])
        ))),
        NewId::PairingRuleId,
        "adding the pairing rule"
    );
    let slot_pairing = apply_new!(
        data,
        Op::SlotPairing(SlotPairingOp::Add(slot_pairing_rule(
            slot,
            other_slot,
            BTreeSet::from([other_period])
        ))),
        NewId::SlotPairingRuleId,
        "adding the slot pairing rule"
    );

    let student = apply_new!(
        data,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::new()))),
        NewId::StudentId,
        "adding the student"
    );
    let other_student = apply_new!(
        data,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::new()))),
        NewId::StudentId,
        "adding the bystander student"
    );
    let excluded_student = apply_new!(
        data,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::from([
            other_period
        ])))),
        NewId::StudentId,
        "adding the excluding student"
    );

    let group_list = apply_new!(
        data,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Liste",
            2,
            BTreeSet::new()
        ))),
        NewId::GroupListId,
        "adding the associated group list"
    );
    let prefilled_group_list_id = apply_new!(
        data,
        Op::GroupList(GroupListOp::Add(prefilled_group_list(
            "Prérempli",
            vec![BTreeSet::from([student, other_student]), BTreeSet::new()]
        ))),
        NewId::GroupListId,
        "adding the prefilled group list"
    );
    let excluding_group_list = apply_new!(
        data,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Exclusions",
            2,
            BTreeSet::from([excluded_student])
        ))),
        NewId::GroupListId,
        "adding the excluding group list"
    );
    apply(
        &mut data,
        Op::GroupList(GroupListOp::AssignToSubject(
            period,
            subject,
            Some(group_list),
        )),
        "associating the group list",
    );
    // Only an *automatic* list may carry a colloscope row
    // (`ColloscopeGroupListPrefilled`), so the placements go on `group_list`.
    apply(
        &mut data,
        Op::Colloscope(ColloscopeOp::SetGroupList(
            group_list,
            BTreeMap::from([(student, 0), (other_student, 1)]),
        )),
        "placing the students in the colloscope group list",
    );
    // The association above is what bounds the group numbers: without it the
    // bound is 0 and no cell can be filled at all.
    apply(
        &mut data,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            slot,
            week,
            BTreeSet::from([0]),
        )),
        "filling the colloscope cell",
    );
    apply(
        &mut data,
        Op::Assignment(AssignmentOp::SetRow(
            period,
            subject,
            BTreeSet::from([student, other_student]),
        )),
        "filling the assignments row",
    );

    // Both overrides are deliberately *different* from the global values, so
    // that a test which drops one is looking at a real change.
    apply(
        &mut data,
        Op::Settings(SettingsOp::SetStudent(
            student,
            Some(Limits {
                interrogations_per_week_max: Some(SoftParam {
                    soft: false,
                    value: 3,
                }),
                ..Default::default()
            }),
        )),
        "setting the per-student limits",
    );
    apply(
        &mut data,
        Op::Balancing(BalancingOp::SetSubject(
            subject,
            Some(BalancingOptions {
                avoid_twice_in_a_row: false,
                year_teacher_rotation: true,
                ..Default::default()
            }),
        )),
        "setting the per-subject balancing override",
    );

    // The dead ids, created last so that nothing above can reference them, and
    // removed straight away. Each removal cascades to nothing and lands alone.
    let dead_period = apply_new!(
        data,
        Op::Period(PeriodOp::AddAfter(other_period)),
        NewId::PeriodId,
        "adding the soon-dead period"
    );
    let dead_week = apply_new!(
        data,
        Op::Week(WeekOp::AddFront(dead_period, WeekDesc::default())),
        NewId::WeekId,
        "adding the soon-dead week"
    );
    let dead_subject = apply_new!(
        data,
        Op::Subject(SubjectOp::AddAfter(
            Some(excluded_subject),
            plain_subject("Éphémère", BTreeSet::new())
        )),
        NewId::SubjectId,
        "adding the soon-dead subject"
    );
    let dead_teacher = apply_new!(
        data,
        Op::Teacher(TeacherOp::Add(Teacher::default())),
        NewId::TeacherId,
        "adding the soon-dead teacher"
    );
    let dead_student = apply_new!(
        data,
        Op::Student(StudentOp::Add(plain_student(BTreeSet::new()))),
        NewId::StudentId,
        "adding the soon-dead student"
    );
    let dead_week_pattern = apply_new!(
        data,
        Op::WeekPattern(WeekPatternOp::Add(WeekPattern {
            name: "éphémère".into(),
            excluded_weeks: BTreeSet::new(),
        })),
        NewId::WeekPatternId,
        "adding the soon-dead week pattern"
    );
    let dead_slot = apply_new!(
        data,
        Op::Slot(SlotOp::AddAfter(
            Some(other_slot),
            make_slot(subject, teacher, None, 14)
        )),
        NewId::SlotId,
        "adding the soon-dead slot"
    );
    let dead_group_list = apply_new!(
        data,
        Op::GroupList(GroupListOp::Add(automatic_group_list(
            "Éphémère",
            2,
            BTreeSet::new()
        ))),
        NewId::GroupListId,
        "adding the soon-dead group list"
    );

    apply(
        &mut data,
        Op::GroupList(GroupListOp::Remove(dead_group_list)),
        "removing the dead group list",
    );
    apply(
        &mut data,
        Op::Slot(SlotOp::Remove(dead_slot)),
        "removing the dead slot",
    );
    apply(
        &mut data,
        Op::WeekPattern(WeekPatternOp::Remove(dead_week_pattern)),
        "removing the dead week pattern",
    );
    apply(
        &mut data,
        Op::Student(StudentOp::Remove(dead_student)),
        "removing the dead student",
    );
    apply(
        &mut data,
        Op::Teacher(TeacherOp::Remove(dead_teacher)),
        "removing the dead teacher",
    );
    apply(
        &mut data,
        Op::Subject(SubjectOp::Remove(dead_subject)),
        "removing the dead subject",
    );
    apply(
        &mut data,
        Op::Week(WeekOp::Remove(dead_week)),
        "removing the dead week",
    );
    apply(
        &mut data,
        Op::Period(PeriodOp::Remove(dead_period)),
        "removing the dead period",
    );

    let doc = ValidDocument {
        period,
        other_period,
        week,
        other_week,
        subject,
        other_subject,
        excluded_subject,
        teacher,
        week_pattern,
        slot,
        other_slot,
        lone_slot,
        incompat,
        pairing,
        slot_pairing,
        student,
        other_student,
        excluded_student,
        group_list,
        prefilled_group_list: prefilled_group_list_id,
        excluding_group_list,
        dead_period,
        dead_week,
        dead_subject,
        dead_teacher,
        dead_student,
        dead_week_pattern,
        dead_slot,
        dead_group_list,
    };
    (data, doc)
}

/// Steps 3 and 4, shared by every one-break test here.
///
/// Step 3 derives the invariant from the twin instead of hand-writing it, so
/// the test cannot drift away from the checker, and pins the *whole* set: that
/// is what makes the corruption surgical — one edit, one broken shape. Step 4
/// then asks the untouched document, and `why` says in one sentence what makes
/// it innocent.
///
/// Arms whose corruption provably co-breaks a second invariant (§9bis names
/// two) cannot use this helper: they pin a two-element set and then select the
/// element under test, which is not always `set.first()`.
fn assert_arm_finds_nothing(
    valid: &Data,
    corrupt: &InnerData,
    expected: FixableInvariant,
    why: &str,
) {
    let set = corrupt
        .broken_invariants()
        .expect("the corruption is fixable, not a logic error");
    assert_eq!(set, BTreeSet::from([expected]));
    let invariant = set
        .into_iter()
        .next()
        .expect("the set was just asserted to hold one element");

    assert_eq!(valid.fix_invariant(&invariant), None, "{why}");
}

// ---- The arms ----

/// `TeacherRefSite::SlotTeacher` — the arm that produced frame point 5.
///
/// The corruption points the slot at a dead teacher. That is a shape a user can
/// really reach: `force_apply_slot`'s `Update` keeps only `CannotChangeSubject`
/// and has no teacher-existence carve-out, so `SlotOp::Update(slot, new_slot
/// naming a dead teacher)` lands, the checker reports this dangle, and the gate
/// rolls it back — leaving the arm to be asked about a slot whose live teacher
/// is perfectly fine.
///
/// **Exactly one break**, hand-derived: the teacher-teaches predicate sits
/// behind a teacher lookup (`invariants.rs:428`), so a *dead* teacher makes it
/// skip rather than fire. `SlotTeacherDoesNotTeachSubject` does not accompany
/// the dangle.
///
/// Without the arm's identity test, the answer here would be
/// `Some(Slot::Remove(slot))` merely because the slot exists — and the user
/// would have a slot deleted, then be told the operation succeeded.
#[test]
fn slot_teacher_arm_spares_a_slot_whose_teacher_is_live() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let live_slot = corrupt
        .params
        .slots
        .find_slot(doc.slot)
        .expect("the fixture's slot is there")
        .clone();
    // `replace_slot` and not raw field access: the subject is unchanged here, so
    // the ordering mirror stays consistent and step 3 sees the dangle rather
    // than a `LogicError`.
    corrupt.params.slots.replace_slot(
        doc.slot,
        Slot {
            teacher_id: doc.dead_teacher,
            ..live_slot
        },
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Teacher {
            target: doc.dead_teacher,
            site: TeacherRefSite::SlotTeacher(doc.slot),
        }),
        "the live slot's teacher is not the dead one, so the arm has nothing to remove",
    );
}

// ---- Target: a period (`PeriodRefSite`) ----
//
// Seven arms, and they split cleanly in two. The first five hold the period
// inside a row — as a bare FK or as a member of an excluded set — so the fix
// names only the row and the arm needs an explicit identity test; the fixture's
// innocent counterpart is a row naming a *live* period, and the test is that
// the arm compares. The last two hold the period in a **row key**, so the fix
// carries it and no identity test is possible: there the whole content is the
// lookup, and the fixture is built so that a live row for the same subject
// exists on another period — an arm that keyed on the subject alone would find
// it and fire.

/// `PeriodRefSite::WeekPeriodFk` — the one arm here that *removes* a row.
///
/// The corruption moves a week into the dead period. It goes through
/// `move_week_entry` rather than raw fields: the week ordering is a type-level
/// mirror of `week_map`, and a desynced mirror is a `LogicError` that
/// short-circuits the checker, so a hand-built twin would die at step 3 with
/// `WeekOrderingWrongPeriod` instead of yielding the dangle. The mutator
/// rewrites both together, and the sidecar's row keys are not liveness-checked.
///
/// **Exactly one break**: the moved week is `other_week`, which carries no
/// colloscope cell, so none of the interrogation-versus-period variants have
/// anything to say about it. (Its week pattern excludes it, but that is a
/// property of the pattern and says nothing about which period it sits in.)
///
/// The arm is asked about a document where that week sits in a live period, and
/// must not delete it.
#[test]
fn week_period_fk_arm_spares_a_week_whose_period_is_live() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .weeks
        .move_week_entry(doc.other_week, doc.dead_period, 0);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Period {
            target: doc.dead_period,
            site: PeriodRefSite::WeekPeriodFk(doc.other_week),
        }),
        "the live week belongs to a live period, so the arm has no week to remove",
    );
}

/// `PeriodRefSite::SubjectExcludedPeriods`.
///
/// The corruption swaps the excluded period for the dead one, rather than
/// adding it: keeping the set a singleton is what makes the twin surgical, and
/// it also keeps the subject inert. The subject chosen runs no interrogations
/// and is referenced by nothing else, so no `Convergence` variant can join in.
///
/// The arm is asked about a document where that subject excludes a *live*
/// period, and must not rewrite it.
#[test]
fn subject_excluded_periods_arm_spares_a_subject_excluding_a_live_period() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .subjects
        .ordered_subject_list
        .get_mut(&doc.excluded_subject)
        .expect("the fixture's excluding subject is there")
        .excluded_periods = BTreeSet::from([doc.dead_period]);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Period {
            target: doc.dead_period,
            site: PeriodRefSite::SubjectExcludedPeriods(doc.excluded_subject),
        }),
        "the live subject excludes a live period, so the arm has no element to drop",
    );
}

/// `PeriodRefSite::StudentExcludedPeriods` — the student twin of the arm above.
///
/// The student chosen sits in no assignments row, so
/// `AssignedStudentNotPresentForPeriod` has nothing to say about the swap.
#[test]
fn student_excluded_periods_arm_spares_a_student_excluding_a_live_period() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .students
        .student_map
        .get_mut(&doc.excluded_student)
        .expect("the fixture's excluding student is there")
        .excluded_periods = BTreeSet::from([doc.dead_period]);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Period {
            target: doc.dead_period,
            site: PeriodRefSite::StudentExcludedPeriods(doc.excluded_student),
        }),
        "the live student excludes a live period, so the arm has no element to drop",
    );
}

/// `PeriodRefSite::PairingRuleExcludedPeriods`.
///
/// `PairingRule`'s fields are private, so the twin is rebuilt through
/// `into_parts` and the validating constructor — the same door the arm itself
/// uses — and written back over its own id. The two parts are carried across
/// untouched, so the rebuild cannot fail.
#[test]
fn pairing_rule_excluded_periods_arm_spares_a_rule_excluding_a_live_period() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (antecedent, consequent, _excluded, soft) = corrupt
        .params
        .pairings
        .pairing_rule_map
        .get(&doc.pairing)
        .expect("the fixture's pairing rule is there")
        .clone()
        .into_parts();
    corrupt.params.pairings.pairing_rule_map.insert(
        doc.pairing,
        PairingRule::new(
            antecedent,
            consequent,
            BTreeSet::from([doc.dead_period]),
            soft,
        )
        .expect("the parts are carried across untouched, so they still name distinct subjects"),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Period {
            target: doc.dead_period,
            site: PeriodRefSite::PairingRuleExcludedPeriods(doc.pairing),
        }),
        "the live rule excludes a live period, so the arm has no element to drop",
    );
}

/// `PeriodRefSite::SlotPairingRuleExcludedPeriods` — the exact twin of the arm
/// above, on the slot-level rule.
#[test]
fn slot_pairing_rule_excluded_periods_arm_spares_a_rule_excluding_a_live_period() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (antecedent, consequent, _excluded, soft) = corrupt
        .params
        .slot_pairings
        .slot_pairing_rule_map
        .get(&doc.slot_pairing)
        .expect("the fixture's slot pairing rule is there")
        .clone()
        .into_parts();
    corrupt.params.slot_pairings.slot_pairing_rule_map.insert(
        doc.slot_pairing,
        SlotPairingRule::new(
            antecedent,
            consequent,
            BTreeSet::from([doc.dead_period]),
            soft,
        )
        .expect("the parts are carried across untouched, so they still name distinct slots"),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Period {
            target: doc.dead_period,
            site: PeriodRefSite::SlotPairingRuleExcludedPeriods(doc.slot_pairing),
        }),
        "the live rule excludes a live period, so the arm has no element to drop",
    );
}

/// `PeriodRefSite::AssignmentsKey` — the period is half the row key, so the fix
/// carries it and there is no identity test to write. The arm's whole content is
/// the lookup, and this is the test that it is keyed on the *pair*.
///
/// The corruption adds a row on the dead period **for the subject that already
/// has one on a live period**. So the valid document does hold an assignments
/// row for that subject; what it does not hold is one on the dead period. An arm
/// that looked the subject up and ignored the period would find that row, answer
/// `Some`, and clear a row nothing complained about.
///
/// **Exactly one break**: the row's subject excludes no period and its students
/// exclude none either, so neither `AssignmentForSubjectNotRunningOnPeriod` nor
/// `AssignedStudentNotPresentForPeriod` fires beside the dangle.
#[test]
fn assignments_key_arm_spares_a_row_on_another_period() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt.params.assignments.map.insert(
        (doc.dead_period, doc.subject),
        BTreeSet::from([doc.student, doc.other_student]),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Period {
            target: doc.dead_period,
            site: PeriodRefSite::AssignmentsKey {
                subject: doc.subject,
            },
        }),
        "the subject's only live row sits on another period, so the arm has no row to clear",
    );
}

/// `PeriodRefSite::AssociationEntry` — the key-half twin of the arm above, and
/// the same trap: the corruption associates a group list to the dead period for
/// the subject that is *already* associated on a live one.
///
/// The live association is left in place on purpose. It is what makes the cell
/// in the colloscope legal (an unassociated `(period, subject)` bounds every
/// group number by 0), and it is what an arm keyed on the subject alone would
/// wrongly find.
///
/// **Exactly one break**: the subject runs interrogations and excludes no
/// period, so neither `AssociationForSubjectWithoutInterrogations` nor
/// `AssociationForSubjectNotRunningOnPeriod` joins the dangle.
#[test]
fn association_entry_arm_spares_an_entry_on_another_period() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .group_lists
        .subjects_associations
        .insert((doc.dead_period, doc.subject), doc.group_list);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Period {
            target: doc.dead_period,
            site: PeriodRefSite::AssociationEntry {
                subject: doc.subject,
            },
        }),
        "the subject's only live association sits on another period, so there is nothing to unassign",
    );
}

// ---- Target: a subject (`SubjectRefSite`) ----
//
// Eight arms, and the widest spread of consequences in the map: three of them
// *delete* a row outright (a slot, an incompatibility, a pairing rule), so a
// missing identity test here does not merely widen something — it destroys
// data. Two of the three are also **reachable on today's code**: nothing in
// `force_apply_incompat`'s or `force_apply_pairing`'s `Update` guards the
// subject field, so a user really can leave a live row pointing at a dead
// subject and have the arm asked about it.
//
// The corruptions all gate the same way in the checker, which is what keeps
// seven of the eight at one break: every `Convergence` predicate that would
// have something to say about these rows reads the subject first
// (`find_subject` behind a `let … else { continue }` or an `if let`), and a
// dead subject makes it skip. `SlotSubject` is the exception, and it is the
// documented one.

/// `SubjectRefSite::TeacherSubjects`.
///
/// The corruption *adds* the dead subject to the teacher's set rather than
/// replacing what is there. Replacing would take `subject` out of the set, and
/// the teacher's three slots are all on `subject` — so
/// `SlotTeacherDoesNotTeachSubject` would fire three times over and bury the
/// dangle. Adding leaves every slot happy.
///
/// **Exactly one break**: `TeacherSubjectWithoutInterrogations` reads the
/// subject behind a `let … else { continue }` (`invariants.rs:472`), so a dead
/// one makes it skip.
#[test]
fn teacher_subjects_arm_spares_a_teacher_teaching_only_live_subjects() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .teachers
        .teacher_map
        .get_mut(&doc.teacher)
        .expect("the fixture's teacher is there")
        .subjects
        .insert(doc.dead_subject);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Subject {
            target: doc.dead_subject,
            site: SubjectRefSite::TeacherSubjects(doc.teacher),
        }),
        "the live teacher teaches only live subjects, so the arm has no element to drop",
    );
}

/// `SubjectRefSite::SlotSubject` — **the two-element exception** (§9bis).
///
/// No state where a slot's subject dangles breaks only one invariant. The
/// teacher-teaches check gates on the *teacher* lookup alone — the subject id is
/// a compared value, and a compared value deliberately does not gate
/// (`invariants.rs:433-441`) — so a live teacher never `contains` the dead
/// subject and `SlotTeacherDoesNotTeachSubject` fires beside the dangle. Killing
/// the teacher too would only swap that companion for the `SlotTeacher` dangle.
/// So the fixture keeps the teacher live and teaching, which makes the companion
/// the deterministic one, and the expected literal is a **two-element set** —
/// still hand-derived, still `assert_eq!`d whole.
///
/// Step 4 then runs on the element the test is about, picked out of the set by
/// shape. It is *not* `set.first()`: that happens to be the dangle here, but
/// relying on the derived `Ord` would make this test quietly about pick order
/// instead of about the arm.
///
/// The slot corrupted is the one referenced by nothing else, and the surgery
/// goes through `remove_slot` + `insert_slot_at` rather than a field write:
/// the slot ordering is keyed *by subject*, so a raw write would leave the slot
/// filed under its old subject and the twin would die at step 3 with a
/// `LogicError`. `insert_slot_at` creates the dead subject's ordering row on
/// demand, and those row keys are not liveness-checked.
#[test]
fn slot_subject_arm_spares_a_slot_whose_subject_is_live() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (_position, live_slot) = corrupt.params.slots.remove_slot(doc.lone_slot);
    corrupt.params.slots.insert_slot_at(
        doc.lone_slot,
        Slot {
            subject_id: doc.dead_subject,
            ..live_slot
        },
        0,
    );

    let set = corrupt
        .broken_invariants()
        .expect("the corruption is fixable, not a logic error");
    assert_eq!(
        set,
        BTreeSet::from([
            FixableInvariant::DanglingFk(Reference::Subject {
                target: doc.dead_subject,
                site: SubjectRefSite::SlotSubject(doc.lone_slot),
            }),
            FixableInvariant::Convergence(Convergence::SlotTeacherDoesNotTeachSubject(
                doc.lone_slot,
                doc.teacher,
                doc.dead_subject,
            )),
        ])
    );
    let invariant = set
        .into_iter()
        .find(|invariant| matches!(invariant, FixableInvariant::DanglingFk(_)))
        .expect("the set was just asserted to hold the dangle");

    assert_eq!(
        valid.fix_invariant(&invariant),
        None,
        "the live slot's subject is not the dead one, so the arm has no slot to remove"
    );
}

/// `SubjectRefSite::IncompatSubject` — **reachable on today's code**.
///
/// `force_apply_incompat`'s `Update` replaces the whole row with no field guards
/// (`incompats.rs:108-124`), so `IncompatOp::Update(incompat, row naming a dead
/// subject)` really lands, the checker really reports this dangle, and the gate
/// really rolls it back. Without the arm's identity test the answer would be
/// `Some(Incompat::Remove(incompat))` merely because the row exists — and the
/// user would lose an incompatibility whose live subject was fine.
///
/// **Exactly one break**: no `Convergence` variant mentions an incompatibility
/// at all.
#[test]
fn incompat_subject_arm_spares_an_incompat_whose_subject_is_live() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .incompats
        .incompat_map
        .get_mut(&doc.incompat)
        .expect("the fixture's incompatibility is there")
        .subject_id = doc.dead_subject;

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Subject {
            target: doc.dead_subject,
            site: SubjectRefSite::IncompatSubject(doc.incompat),
        }),
        "the live incompatibility names a live subject, so the arm has no row to remove",
    );
}

/// `SubjectRefSite::PairingRuleAntecedent` — **reachable**, like the arm above:
/// `force_apply_pairing`'s `Update` has no field guards
/// (`pairings.rs:237-247`).
///
/// The two parts get separate arms even though both emit the same `Remove`,
/// which is exactly what this test and the next one pin: an arm testing neither
/// part — or testing the wrong one — would delete a rule whose two parts are
/// both live. Here only the **antecedent** is corrupted, so only the antecedent
/// arm is asked; the consequent's own test is the next one.
///
/// `PairingRule`'s fields are private, so the twin goes through `into_parts` and
/// the validating constructor. The dead subject is not the consequent's, so the
/// rebuild cannot trip the one build error.
#[test]
fn pairing_rule_antecedent_arm_spares_a_rule_whose_antecedent_is_live() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (mut antecedent, consequent, excluded_periods, soft) = corrupt
        .params
        .pairings
        .pairing_rule_map
        .get(&doc.pairing)
        .expect("the fixture's pairing rule is there")
        .clone()
        .into_parts();
    antecedent.subject_id = doc.dead_subject;
    corrupt.params.pairings.pairing_rule_map.insert(
        doc.pairing,
        PairingRule::new(antecedent, consequent, excluded_periods, soft)
            .expect("the dead subject is not the consequent's, so the parts stay distinct"),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Subject {
            target: doc.dead_subject,
            site: SubjectRefSite::PairingRuleAntecedent(doc.pairing),
        }),
        "the live rule's antecedent names a live subject, so the arm has no rule to remove",
    );
}

/// `SubjectRefSite::PairingRuleConsequent` — the mirror of the arm above, and
/// the reason the two are separate arms rather than one.
#[test]
fn pairing_rule_consequent_arm_spares_a_rule_whose_consequent_is_live() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (antecedent, mut consequent, excluded_periods, soft) = corrupt
        .params
        .pairings
        .pairing_rule_map
        .get(&doc.pairing)
        .expect("the fixture's pairing rule is there")
        .clone()
        .into_parts();
    consequent.subject_id = doc.dead_subject;
    corrupt.params.pairings.pairing_rule_map.insert(
        doc.pairing,
        PairingRule::new(antecedent, consequent, excluded_periods, soft)
            .expect("the dead subject is not the antecedent's, so the parts stay distinct"),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Subject {
            target: doc.dead_subject,
            site: SubjectRefSite::PairingRuleConsequent(doc.pairing),
        }),
        "the live rule's consequent names a live subject, so the arm has no rule to remove",
    );
}

/// `SubjectRefSite::BalancingSubjectKey` — a pure key site: the subject *is* the
/// key, so there is no other half to drop and no identity test to write. The
/// arm's whole content is one `contains`.
///
/// The fixture already carries a per-subject override for a live subject, and
/// the corruption adds one for the dead subject beside it. So the valid document
/// is not innocent by being empty: an arm that asked "are there any overrides at
/// all" would find one and answer `Some` — and emit `SetSubject(dead, None)`,
/// which against a state with no such entry is a perfect no-op, which the engine
/// answers with a panic.
///
/// **Exactly one break**: `BalancingForSubjectWithoutInterrogations` reads the
/// subject behind a `let … else { continue }` (`invariants.rs:534`).
#[test]
fn balancing_subject_key_arm_spares_an_override_on_a_live_subject() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .balancing
        .subjects
        .insert(doc.dead_subject, BalancingOptions::default());

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Subject {
            target: doc.dead_subject,
            site: SubjectRefSite::BalancingSubjectKey,
        }),
        "the only live override is on a live subject, so the arm has no entry to drop",
    );
}

/// `SubjectRefSite::AssignmentsKey` — the subject half of the key whose period
/// half 7.5b tested, and corrupted the same way: the row is *added* on the dead
/// subject for the period that already carries a live row.
///
/// So the valid document really does hold an assignments row on that period, and
/// an arm that keyed on the period alone would find it and clear a row nothing
/// complained about.
///
/// **Exactly one break**: `AssignmentForSubjectNotRunningOnPeriod` reads the
/// subject behind an `if let` (`invariants.rs:488`), and the assigned student
/// excludes no period.
#[test]
fn assignments_key_arm_spares_a_row_on_another_subject() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt.params.assignments.map.insert(
        (doc.period, doc.dead_subject),
        BTreeSet::from([doc.student]),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Subject {
            target: doc.dead_subject,
            site: SubjectRefSite::AssignmentsKey { period: doc.period },
        }),
        "the period's only live row is on another subject, so the arm has no row to clear",
    );
}

/// `SubjectRefSite::AssociationEntry` — the key-half twin of the arm above.
///
/// The live association on `(period, subject)` is left in place: it is what an
/// arm keyed on the period alone would wrongly find, and it is also what keeps
/// the fixture's colloscope cell legal.
///
/// **Exactly one break**: both association predicates read the subject behind a
/// `let … else { continue }` (`invariants.rs:516`).
#[test]
fn association_entry_arm_spares_an_entry_on_another_subject() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .group_lists
        .subjects_associations
        .insert((doc.period, doc.dead_subject), doc.group_list);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Subject {
            target: doc.dead_subject,
            site: SubjectRefSite::AssociationEntry { period: doc.period },
        }),
        "the period's only live association is on another subject, so there is nothing to unassign",
    );
}

// ---- Target: a student (`StudentRefSite`) ----
//
// Five arms, and not one of them deletes a row: a student is always dropped
// *out* of something that survives — a prefilled group, an excluded set, an
// assignments row, a colloscope placements row — or an override entry keyed by
// the student is cleared. So a missing identity test here does not destroy a
// row; it silently drops an innocent student out of one, which is worse in a
// different way, because nothing about the resulting document looks wrong.
//
// Four of the five emit an op that does **not** name the student
// (`GroupListOp::Update`, `AssignmentOp::SetRow`, `ColloscopeOp::SetGroupList`
// all carry the rebuilt row, never the member being removed), so rule 4 asks
// each of them for an explicit identity test. The fifth,
// `SettingsStudentKey`, emits `SetStudent(student, None)`, which does carry the
// target — there the test under scrutiny is the *presence* one, which exists to
// keep the arm from emitting a perfect no-op.
//
// Two of the four are also the arms whose presence test doubles as a
// variant test: a group list is prefilled or automatic, never both, so
// `contains_student` is `false` on an automatic list and the excluded arm
// matches `GroupListFilling::Automatic` explicitly. The fixture holds one list
// of each kind, so an arm that forgot which it was looking at has something to
// trip over.

/// `StudentRefSite::GroupListPrefilledStudent`.
///
/// The corruption puts the dead student in the prefilled list's *second* group,
/// leaving the first exactly as built — so the valid document still holds the
/// innocent counterpart, a prefilled list whose members are all live.
///
/// **Exactly one break**: no `Convergence` predicate reads `group_list_map` at
/// all. The colloscope group-list loop is the only one that looks at group-list
/// membership (`invariants.rs:628`), and it iterates over the *colloscope*'s
/// rows, which this list has none of.
#[test]
fn group_list_prefilled_student_arm_spares_a_list_of_live_members() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (params, filling) = corrupt
        .params
        .group_lists
        .group_list_map
        .get(&doc.prefilled_group_list)
        .expect("the fixture's prefilled group list is there")
        .clone()
        .into_parts();
    let GroupListFilling::Prefilled { mut groups } = filling else {
        panic!("the fixture's prefilled group list is prefilled");
    };
    groups[1].students.insert(doc.dead_student);
    corrupt.params.group_lists.group_list_map.insert(
        doc.prefilled_group_list,
        GroupList::new(params, GroupListFilling::Prefilled { groups })
            .expect("the group count is unchanged and the dead student sits in one group only"),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Student {
            target: doc.dead_student,
            site: StudentRefSite::GroupListPrefilledStudent(doc.prefilled_group_list),
        }),
        "the live list holds only live students, so the arm has no member to drop",
    );
}

/// `StudentRefSite::GroupListExcludedStudent`.
///
/// The corruption swaps the excluded student for the dead one rather than adding
/// it, keeping the set a singleton — the same choice the two
/// `*ExcludedPeriods` arms above make, and for the same reason: a singleton is
/// the surgical edit. The valid document's counterpart is still a list that
/// excludes *somebody*, so the arm cannot pass by finding an empty set.
///
/// **Exactly one break**, for the same reason as the arm above: nothing in
/// `convergence_breaks` reads an excluded set except through a colloscope row,
/// and this list has none.
#[test]
fn group_list_excluded_student_arm_spares_a_list_excluding_a_live_student() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (params, _filling) = corrupt
        .params
        .group_lists
        .group_list_map
        .get(&doc.excluding_group_list)
        .expect("the fixture's excluding group list is there")
        .clone()
        .into_parts();
    corrupt.params.group_lists.group_list_map.insert(
        doc.excluding_group_list,
        GroupList::new(
            params,
            GroupListFilling::Automatic {
                excluded_students: BTreeSet::from([doc.dead_student]),
            },
        )
        .expect("`GroupList::new` validates the prefilled branch only"),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Student {
            target: doc.dead_student,
            site: StudentRefSite::GroupListExcludedStudent(doc.excluding_group_list),
        }),
        "the live list excludes a live student, so the arm has no element to drop",
    );
}

/// `StudentRefSite::SettingsStudentKey` — a pure key site, the student twin of
/// `BalancingSubjectKey`. The op names the student, so a wrong target is not
/// expressible; what the arm needs, and what this pins, is the *presence* test.
///
/// The corruption adds an override for the dead student beside the fixture's
/// live one. So the valid document is not innocent by being empty: an arm that
/// asked "are there any per-student overrides at all" would find one and emit
/// `SetStudent(dead_student, None)`, which against a state with no such entry is
/// a perfect no-op — and the engine answers a no-op fix with a panic.
///
/// **Exactly one break**: no `Convergence` predicate mentions
/// `settings.students`.
#[test]
fn settings_student_key_arm_spares_an_override_on_a_live_student() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .settings
        .students
        .insert(doc.dead_student, Limits::default());

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Student {
            target: doc.dead_student,
            site: StudentRefSite::SettingsStudentKey,
        }),
        "the only live override is on a live student, so the arm has no entry to drop",
    );
}

/// `StudentRefSite::AssignmentsStudent` — the site whose payload is a whole
/// *key*, `{period, subject}`, neither component of which is the target. The op
/// is `SetRow(period, subject, rebuilt)`, which names the row but not the member
/// being removed, so the identity test is the only thing standing between an
/// innocent student and being unassigned.
///
/// The corruption adds the dead student to the fixture's live row rather than
/// building a row of its own: the arm is meant to be asked about a row that
/// really exists and really has members, and to answer `None` because none of
/// them is the one named.
///
/// **Exactly one break**: `AssignedStudentNotPresentForPeriod` reads the student
/// behind a `let … else { continue }` (`invariants.rs:496`), so a dead one makes
/// it skip, and the row's subject excludes no period.
#[test]
fn assignments_student_arm_spares_a_row_of_live_students() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt.params.assignments.map.insert(
        (doc.period, doc.subject),
        BTreeSet::from([doc.student, doc.other_student, doc.dead_student]),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Student {
            target: doc.dead_student,
            site: StudentRefSite::AssignmentsStudent {
                period: doc.period,
                subject: doc.subject,
            },
        }),
        "the live row assigns only live students, so the arm has no member to drop",
    );
}

/// `StudentRefSite::ColloscopeGroupListStudent`.
///
/// The corruption places the dead student in the fixture's colloscope row, in
/// group 0. The group number matters: the row's list has two groups, so 0 is in
/// bounds and `ColloscopeStudentGroupOutOfBounds` stays quiet, and the list is
/// automatic with an empty excluded set, so `ColloscopeStudentExcluded` does
/// too. **Exactly one break.**
///
/// The arm answers `None` on three counts, and this test pins the third: a
/// missing row, a live row that does not place the student, and — the one under
/// test here — a live row that places somebody else entirely. All three would
/// otherwise produce a `SetGroupList` that either changes nothing at all (the
/// no-op panic again) or quietly unplaces an innocent student.
#[test]
fn colloscope_group_list_student_arm_spares_a_row_of_live_placements() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let mut placements = corrupt
        .colloscope
        .group_list(doc.group_list)
        .expect("the fixture's colloscope group-list row is there")
        .clone();
    placements.insert(doc.dead_student, 0);
    corrupt
        .colloscope
        .set_group_list(doc.group_list, placements);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Student {
            target: doc.dead_student,
            site: StudentRefSite::ColloscopeGroupListStudent(doc.group_list),
        }),
        "the live row places only live students, so the arm has no placement to remove",
    );
}

// ---- Targets: a week and a week pattern (`WeekRefSite`, `WeekPatternRefSite`) ----
//
// Two arms each, merged into one commit because they share a shape: all four
// clear a reference out of a row that survives, and none of them removes
// anything. The two week-pattern arms are the map's one deliberate divergence
// from the legacy cleaning, which deleted the slot and the incompatibility
// outright: here the field is an `Option` whose `None` is a legal, documented
// value meaning "every week", so the reference can leave on its own. That makes
// their identity test the only thing between a live pattern and a slot quietly
// losing it — and the loss is invisible, because a slot with no pattern is a
// perfectly ordinary slot that simply runs every week.
//
// The `ColloscopeInterrogation` arm is the third key-half site in the series,
// after 7.5b's and 7.5c's, and is corrupted the same way: the row is *added* on
// the dead week for a slot that already carries a live row.

/// `WeekRefSite::WeekPatternExcludedWeek`.
///
/// The corruption swaps the excluded week for the dead one rather than adding
/// it, keeping the set a singleton — and keeping the pattern's meaning innocent
/// in the one place it is read. `is_week_active` is consulted for the fixture's
/// colloscope cell, which sits on `week`; the pattern excluded `other_week`
/// before and excludes `dead_week` after, so `week` is active either way and
/// `InterrogationOnInactiveWeek` stays quiet. **Exactly one break.**
///
/// The arm is asked about a document where the pattern excludes a *live* week,
/// and must not rewrite it.
#[test]
fn week_pattern_excluded_week_arm_spares_a_pattern_excluding_a_live_week() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .week_patterns
        .week_pattern_map
        .get_mut(&doc.week_pattern)
        .expect("the fixture's week pattern is there")
        .excluded_weeks = BTreeSet::from([doc.dead_week]);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Week {
            target: doc.dead_week,
            site: WeekRefSite::WeekPatternExcludedWeek(doc.week_pattern),
        }),
        "the live pattern excludes a live week, so the arm has no element to drop",
    );
}

/// `WeekRefSite::ColloscopeInterrogation` — a key-half site: the week is half of
/// the `(slot, week)` key and the slot is the payload, so
/// `SetInterrogation(slot, week, {})` carries both and no identity test is
/// expressible. The arm's whole content is the lookup, and this is the test that
/// it is keyed on the *pair*.
///
/// The corruption adds a row on the dead week **for the slot that already
/// carries one on a live week**. So the valid document does hold a colloscope
/// row for that slot; what it does not hold is one on the dead week. An arm that
/// looked the slot up and ignored the week would find that row, answer `Some`,
/// and clear a cell nothing complained about — which for a colloscope means
/// losing a placement the user made by hand.
///
/// **Exactly one break**: all three predicates of the interrogation loop are
/// gated on the week resolving to a period (`invariants.rs:572`, then `if let
/// (Some(period_id), …)` twice and `period.is_some()` once), and a dead week has
/// no position, so none of them runs. The row's slot is live, so the `Slot`
/// reference the same row emits does not dangle either.
#[test]
fn colloscope_interrogation_week_arm_spares_a_cell_on_another_week() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    // Non-empty: rows are canonical-absent, so an empty one is a `LogicError`
    // that would short-circuit the checker before it reaches the dangle.
    corrupt
        .colloscope
        .set_interrogation(doc.slot, doc.dead_week, BTreeSet::from([0]));

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Week {
            target: doc.dead_week,
            site: WeekRefSite::ColloscopeInterrogation { slot: doc.slot },
        }),
        "the slot's only live cell sits on another week, so the arm has no row to clear",
    );
}

/// `WeekPatternRefSite::SlotWeekPattern`.
///
/// The corruption is done with `replace_slot` rather than raw field access: the
/// subject is unchanged, so the ordering mirror stays consistent and step 3 sees
/// the dangle instead of a `LogicError`. It targets `slot`, the one slot that
/// *wears* a pattern — so in the valid document the arm finds a live pattern
/// there, not an empty field, and the comparison it makes is the real one.
///
/// **Exactly one break**: the checker treats a dangling pattern as excluding
/// nothing (`is_week_active`, `invariants.rs:587-594`), matching the old
/// checker, so the fixture's cell stays on an active week and
/// `InterrogationOnInactiveWeek` does not fire. The slot's teacher and subject
/// are untouched, so the slots loop stays quiet too.
#[test]
fn slot_week_pattern_arm_spares_a_slot_wearing_a_live_pattern() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let live_slot = corrupt
        .params
        .slots
        .find_slot(doc.slot)
        .expect("the fixture's patterned slot is there")
        .clone();
    corrupt.params.slots.replace_slot(
        doc.slot,
        Slot {
            week_pattern: Some(doc.dead_week_pattern),
            ..live_slot
        },
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::WeekPattern {
            target: doc.dead_week_pattern,
            site: WeekPatternRefSite::SlotWeekPattern(doc.slot),
        }),
        "the live slot wears a live pattern, so the arm has no field to clear",
    );
}

/// `WeekPatternRefSite::IncompatWeekPattern` — the incompatibility twin of the
/// arm above, and the one whose divergence from the legacy cleaning is the
/// starkest: the old code deleted the whole incompatibility, this one clears a
/// field and keeps the row.
///
/// **Exactly one break**: no `Convergence` variant mentions an incompatibility
/// at all, so nothing in layer C can react to the swap.
#[test]
fn incompat_week_pattern_arm_spares_an_incompat_wearing_a_live_pattern() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .incompats
        .incompat_map
        .get_mut(&doc.incompat)
        .expect("the fixture's incompatibility is there")
        .week_pattern_id = Some(doc.dead_week_pattern);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::WeekPattern {
            target: doc.dead_week_pattern,
            site: WeekPatternRefSite::IncompatWeekPattern(doc.incompat),
        }),
        "the live incompatibility wears a live pattern, so the arm has no field to clear",
    );
}

// ---- Targets: a slot and a group list (`SlotRefSite`, `GroupListRefSite`) ----
//
// Five arms, and the commit that closes the dangling-FK half. Three of them
// clear a colloscope row or an association; two remove a slot pairing rule
// outright.
//
// They also complete the series' catalogue of key shapes, which is worth
// reading as a whole, because the corruption recipe follows from it:
//
// - The target is **in the row key** (`SlotRefSite::ColloscopeInterrogation`,
//   `GroupListRefSite::ColloscopeGroupListKey`, and 7.5b/7.5c's `AssignmentsKey`
//   and `AssociationEntry`). The emitted op carries the key, so a wrong target
//   is not expressible and no identity test exists to write. The corruption must
//   *add* a row on the dead coordinate beside a live one, or the valid document
//   would be innocent for the trivial reason that it holds no such row at all.
// - The target is the entry's **value** (`GroupListRefSite::AssociationEntry`).
//   The op carries the key but not the value, so the identity test is the only
//   thing tying the fix to the target — and the corruption *replaces* the live
//   value, so the valid document answers `None` by comparing two live ids.
// - The target is a **field inside the row** (the two pairing parts, and most of
//   §8.1). Same as above: replace, and the comparison is real.

/// `SlotRefSite::SlotPairingRuleAntecedent` — the slot twin of 7.5c's two
/// pairing-rule arms, and separate from its consequent for the same reason: an
/// arm testing neither part, or the wrong one, would delete a rule whose two
/// slots are both live.
///
/// **Exactly one break**: `PairedSlotsNotInSameSubject` is gated on *both* slots
/// resolving (`invariants.rs:550-557`), so a dead antecedent makes it skip
/// rather than fire about mismatched subjects.
#[test]
fn slot_pairing_rule_antecedent_arm_spares_a_rule_whose_antecedent_is_live() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (mut antecedent, consequent, excluded_periods, soft) = corrupt
        .params
        .slot_pairings
        .slot_pairing_rule_map
        .get(&doc.slot_pairing)
        .expect("the fixture's slot pairing rule is there")
        .clone()
        .into_parts();
    antecedent.slot_id = doc.dead_slot;
    corrupt.params.slot_pairings.slot_pairing_rule_map.insert(
        doc.slot_pairing,
        SlotPairingRule::new(antecedent, consequent, excluded_periods, soft)
            .expect("the dead slot is not the consequent's, so the parts stay distinct"),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Slot {
            target: doc.dead_slot,
            site: SlotRefSite::SlotPairingRuleAntecedent(doc.slot_pairing),
        }),
        "the live rule's antecedent names a live slot, so the arm has no rule to remove",
    );
}

/// `SlotRefSite::SlotPairingRuleConsequent` — the mirror of the arm above.
#[test]
fn slot_pairing_rule_consequent_arm_spares_a_rule_whose_consequent_is_live() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (antecedent, mut consequent, excluded_periods, soft) = corrupt
        .params
        .slot_pairings
        .slot_pairing_rule_map
        .get(&doc.slot_pairing)
        .expect("the fixture's slot pairing rule is there")
        .clone()
        .into_parts();
    consequent.slot_id = doc.dead_slot;
    corrupt.params.slot_pairings.slot_pairing_rule_map.insert(
        doc.slot_pairing,
        SlotPairingRule::new(antecedent, consequent, excluded_periods, soft)
            .expect("the dead slot is not the antecedent's, so the parts stay distinct"),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Slot {
            target: doc.dead_slot,
            site: SlotRefSite::SlotPairingRuleConsequent(doc.slot_pairing),
        }),
        "the live rule's consequent names a live slot, so the arm has no rule to remove",
    );
}

/// `SlotRefSite::ColloscopeInterrogation` — the slot half of the `(slot, week)`
/// key whose week half 7.5e tested, and corrupted the same way: the row is
/// *added* on the dead slot for the week that already carries a live one.
///
/// So the valid document really does hold a colloscope row on that week; what it
/// does not hold is one for the dead slot. An arm that looked the week up and
/// ignored the slot would find that row and clear a cell nothing complained
/// about.
///
/// **Exactly one break**: the interrogation loop resolves the slot once
/// (`invariants.rs:573`) and all three of its predicates are gated on that
/// resolving, so a dead slot switches the whole loop body off for its row. The
/// row's week is live, so the `Week` reference the same row emits does not
/// dangle either.
#[test]
fn colloscope_interrogation_slot_arm_spares_a_cell_for_another_slot() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .colloscope
        .set_interrogation(doc.dead_slot, doc.week, BTreeSet::from([0]));

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Slot {
            target: doc.dead_slot,
            site: SlotRefSite::ColloscopeInterrogation { week: doc.week },
        }),
        "the week's only live cell belongs to another slot, so the arm has no row to clear",
    );
}

/// `GroupListRefSite::AssociationEntry` — the only site in the map where the
/// target is an entry's **value** rather than part of its key. The op is
/// `AssignToSubject(period, subject, None)`, which names the entry but not the
/// group list it held, so the identity test is the only thing tying the fix to
/// the target.
///
/// The corruption therefore *replaces* the live association rather than adding
/// one elsewhere: in the valid document the same `(period, subject)` entry
/// exists and holds a *live* group list, so the arm answers `None` by comparing
/// two live ids — which is the comparison under test.
///
/// **Exactly one break**: both association predicates read the subject, which is
/// live and runs interrogations on a period it does not exclude; and the
/// fixture's colloscope cell survives because the group-number bound treats an
/// association to a *dangling* list as unknown and skips the check entirely
/// (`invariants.rs:596-611`) rather than falling back to a bound of 0.
#[test]
fn association_entry_group_list_arm_spares_an_entry_holding_a_live_list() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .group_lists
        .subjects_associations
        .insert((doc.period, doc.subject), doc.dead_group_list);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::GroupList {
            target: doc.dead_group_list,
            site: GroupListRefSite::AssociationEntry {
                period: doc.period,
                subject: doc.subject,
            },
        }),
        "the live entry holds a live group list, so the arm has nothing to unassign",
    );
}

/// `GroupListRefSite::ColloscopeGroupListKey` — a pure key site, like
/// `BalancingSubjectKey` and `SettingsStudentKey`: the group list *is* the key,
/// so there is no complement to carry and no identity test to write. The arm's
/// whole content is one lookup.
///
/// The fixture already carries a colloscope group-list row for a live list, and
/// the corruption adds one for the dead list beside it. So the valid document is
/// not innocent by being empty: an arm that asked "are there any colloscope
/// group-list rows at all" would find one and emit
/// `SetGroupList(dead_group_list, {})`, which against a state with no such row
/// is a perfect no-op — and the engine answers a no-op fix with a panic.
///
/// **Exactly one break**: the colloscope group-list loop reads the list behind a
/// `let … else { continue }` (`invariants.rs:629`), so a dead one makes it skip
/// before it can judge the placement. The placed student is live, so the
/// `Student` reference the same row emits does not dangle.
#[test]
fn colloscope_group_list_key_arm_spares_a_row_of_a_live_list() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    // Non-empty: rows are canonical-absent, so an empty one is a `LogicError`
    // that would short-circuit the checker before it reaches the dangle.
    corrupt
        .colloscope
        .set_group_list(doc.dead_group_list, BTreeMap::from([(doc.student, 0)]));

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::GroupList {
            target: doc.dead_group_list,
            site: GroupListRefSite::ColloscopeGroupListKey,
        }),
        "the only live colloscope row belongs to a live list, so the arm has no row to clear",
    );
}
