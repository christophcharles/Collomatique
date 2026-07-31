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
//! Nothing in the four-step tests runs the engine: no `apply_cascade`, no
//! rejection semantics and no rollback reasoning — the end-to-end counterparts
//! live in `tests/cascade.rs`. The two `GlobalUpdate` policy pins at the very
//! end are the one exception, and they say so. Nothing here tests the
//! *positive* half either (the arm firing when the shape really is live); that
//! belongs to the commit-7 scenarios, which reach it by the legitimate route.
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

use collomatique_state::{ApplyError, Fixable, InMemoryData, apply_cascade};

use crate::balancing::BalancingOptions;
use crate::group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup};
use crate::ids::{
    GroupListId, IncompatId, NewId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekId, WeekPatternId,
};
use crate::incompats::Incompatibility;
use crate::invariants::{Convergence, FixableInvariant};
use crate::ops::{
    AnnotatedOp, AssignmentOp, BalancingOp, ColloscopeOp, GroupListOp, IncompatOp, Op, PairingOp,
    PeriodOp, SettingsOp, SlotOp, SlotPairingOp, StudentOp, SubjectOp, TeacherOp, WeekOp,
    WeekPatternOp,
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
// The struct carried `allow(dead_code)` for the duration of the 7.5 series,
// because each commit read only the handful of fields its arms needed. The
// attribute came off with the last commit: every field is now read by some
// test, which is the cheap check that nothing here was built for nobody.
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
    /// The one week holding no colloscope cell, and so the only one a twin can
    /// move to another period without dragging the group bound in.
    bare_week: WeekId,
    /// The one week of `other_period`. It exists so that §8.2 row 11 has a
    /// coordinate to aim at: that row needs a week whose period the cell's slot
    /// excludes, and `other_period` is the only excluded period there is.
    excluded_period_week: WeekId,
    /// Runs interrogations; hosts both slots, the association, the assignments
    /// row, the incompatibility and the balancing override.
    subject: SubjectId,
    /// Runs interrogations too, and is the pairing rule's consequent. It
    /// excludes `other_period`, and is associated to a group list on `period`.
    other_subject: SubjectId,
    /// Excludes `other_period`, and runs no interrogations.
    excluded_subject: SubjectId,
    /// Teaches `subject`, and is the teacher of all three slots.
    teacher: TeacherId,
    /// Teaches `other_subject` only, and is the teacher of `other_subject_slot`.
    /// The live teacher a twin points a slot at to break the teacher-teaches
    /// check honestly.
    other_teacher: TeacherId,
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
    /// The only slot not on `subject`, and so the only one whose subject
    /// excludes a period. §8.2 row 11's twin puts a cell on it; §8.2 row 10's
    /// two twins point a rule part at it.
    other_subject_slot: SlotId,
    incompat: IncompatId,
    pairing: PairingRuleId,
    slot_pairing: SlotPairingRuleId,
    /// In the prefilled list, the assignments row and the colloscope placements,
    /// and the holder of the per-student settings override.
    student: StudentId,
    /// The bystander: everywhere `student` is, so a rebuild has something to
    /// keep.
    other_student: StudentId,
    /// Excludes `other_period`, and is excluded by `excluding_group_list`. She
    /// is assigned on `period`, where she is present — the innocent witness
    /// §8.2 row 6 needs.
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
    // A week carrying nothing at all. Once §8.2 rows 11 and 12 have their
    // coverage witnesses, every other week in the fixture holds a colloscope
    // cell — and a week that holds one cannot be moved into a dead period
    // surgically, because the cell's group bound is read from an association at
    // *that* period and a dead period has none, so the bound falls to 0 and
    // `InterrogationGroupOutOfBounds` joins in. `PeriodRefSite::WeekPeriodFk`'s
    // twin moves this one.
    let bare_week = apply_new!(
        data,
        Op::Week(WeekOp::AddAfter(other_week, WeekDesc::default())),
        NewId::WeekId,
        "adding the week that carries nothing"
    );
    // `other_period` held no week at all, and §8.2 row 11 needs one: that row
    // fires on a colloscope cell whose slot sits on a subject excluding the
    // *week's* period, so the excluded period has to have a week to put a cell
    // on.
    let excluded_period_week = apply_new!(
        data,
        Op::Week(WeekOp::AddFront(other_period, WeekDesc::default())),
        NewId::WeekId,
        "adding the week of the excluded period"
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
    // It excludes `other_period` — the one subject in the fixture that runs
    // interrogations *and* does not run everywhere. §8.2 row 8 needs exactly
    // that combination: a subject excluded on a period fires row 8 alone, where
    // a subject without interrogations would fire row 7 as well.
    let other_subject = apply_new!(
        data,
        Op::Subject(SubjectOp::AddAfter(
            Some(subject),
            interrogation_subject("Physique", BTreeSet::from([other_period]))
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
    // A second teacher, on the *other* subject. It exists so that a twin can
    // point a slot at a **live** teacher who does not teach that slot's subject
    // — §8.2 row 1's reachable route, and the only way to pin that comparison
    // without reaching for a dead id. It also teaches `other_subject_slot`
    // below, which is the only slot of that subject.
    let other_teacher = apply_new!(
        data,
        Op::Teacher(TeacherOp::Add(Teacher {
            desc: Default::default(),
            subjects: BTreeSet::from([other_subject]),
        })),
        NewId::TeacherId,
        "adding the second teacher"
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
    // The fixture's only slot that is not on `subject`, and so the only one
    // whose subject excludes a period. Two rows of §8.2 need it: row 11, whose
    // twin puts a colloscope cell where the slot's subject excludes the week's
    // period, and row 10, whose twins point one part of the slot pairing rule at
    // a live slot on a *different* subject from the other part.
    let other_subject_slot = apply_new!(
        data,
        Op::Slot(SlotOp::AddAfter(
            None,
            make_slot(other_subject, other_teacher, None, 9)
        )),
        NewId::SlotId,
        "adding the slot of the second subject"
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
    // Two more associations, placed so that §8.2 row 8's twin — which puts an
    // entry at `(other_period, other_subject)` — has a live neighbour sharing
    // *each* half of that coordinate. The first shares the subject, and is what
    // an arm that ignored the period would wrongly find; the second shares the
    // period, and is what an arm that ignored the subject would wrongly find.
    // The second is also ordinary data: the running subject uses the same group
    // list in both periods.
    //
    // Row 7's twin has no such pair, and provably cannot: its subject is the
    // one with interrogations disabled, and *any* entry naming that subject is
    // row 7 itself. Only the period half is coverable there, and the entry on
    // `period` above covers it.
    apply(
        &mut data,
        Op::GroupList(GroupListOp::AssignToSubject(
            period,
            other_subject,
            Some(excluding_group_list),
        )),
        "associating the second group list",
    );
    apply(
        &mut data,
        Op::GroupList(GroupListOp::AssignToSubject(
            other_period,
            subject,
            Some(group_list),
        )),
        "associating the group list on the second period",
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
    // A second placements row, on the list that excludes `excluded_student`.
    // §8.2 row 15's twin adds her to *this* row, so the row has to exist first:
    // the arm looks the list up and then tests membership, and only a successful
    // lookup leaves the membership test as the thing under test. The two
    // students placed here are the ones the list does not exclude, so the row is
    // ordinary data.
    apply(
        &mut data,
        Op::Colloscope(ColloscopeOp::SetGroupList(
            excluding_group_list,
            BTreeMap::from([(student, 0), (other_student, 1)]),
        )),
        "placing the students in the excluding group list",
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
    // Three more cells, each of them a coverage witness for §8.2 rows 11 and 12.
    // Both rows are relational — they read the slot and the week *together* — so
    // neither half of the `(slot, week)` key condemns a cell on its own, and a
    // live neighbour sharing each half is therefore buildable, and required.
    //
    // Row 11's twin sits at `(other_subject_slot, excluded_period_week)`. The
    // first cell below shares its slot; the second shares its week.
    // Row 12's twin sits at `(slot, other_week)`. The fixture's cell above
    // shares its slot; the third cell below shares its week — `other_slot` wears
    // no week pattern, so no week is ever inactive for it, which is what makes
    // an innocent cell on `other_week` possible at all.
    apply(
        &mut data,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            other_subject_slot,
            week,
            BTreeSet::from([0]),
        )),
        "filling the second subject's colloscope cell",
    );
    apply(
        &mut data,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            other_slot,
            excluded_period_week,
            BTreeSet::from([0]),
        )),
        "filling the colloscope cell of the excluded period's week",
    );
    apply(
        &mut data,
        Op::Colloscope(ColloscopeOp::SetInterrogation(
            other_slot,
            other_week,
            BTreeSet::from([0]),
        )),
        "filling the colloscope cell of the second week",
    );
    // `excluded_student` is in this row on purpose. She is absent for the
    // *second* period only, so taking this subject in the first one is ordinary
    // data — and it is what §8.2 row 6's twin needs: that twin puts her in the
    // row on `other_period`, so the valid document must hold her somewhere
    // else, or an arm that searched every row for the named student instead of
    // reading the named coordinate would find nothing and pass for the wrong
    // reason.
    apply(
        &mut data,
        Op::Assignment(AssignmentOp::SetRow(
            period,
            subject,
            BTreeSet::from([student, other_student, excluded_student]),
        )),
        "filling the assignments row",
    );
    // Two more rows, placed so that §8.2 row 5's twin — which puts a row at
    // `(other_period, excluded_subject)` — has a live neighbour sharing *each*
    // half of that coordinate: same period different subject, then same subject
    // different period. An arm that dropped either half of the key would find
    // one of them and clear a row nobody complained about. `Sport` runs on the
    // first period (it excludes only the second), so the second row is ordinary
    // data, not a contrivance.
    //
    // The first of the two does double duty: it is the row §8.2 row 6's twin
    // adds an excluded student to, so that arm's lookup succeeds and its
    // membership test is what the test is really about.
    apply(
        &mut data,
        Op::Assignment(AssignmentOp::SetRow(
            other_period,
            subject,
            BTreeSet::from([student, other_student]),
        )),
        "filling the second assignments row",
    );
    apply(
        &mut data,
        Op::Assignment(AssignmentOp::SetRow(
            period,
            excluded_subject,
            BTreeSet::from([student, other_student]),
        )),
        "filling the third assignments row",
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
        Op::Period(PeriodOp::RemoveWithWeeks(dead_period)),
        "removing the dead period",
    );

    let doc = ValidDocument {
        period,
        other_period,
        week,
        other_week,
        bare_week,
        excluded_period_week,
        subject,
        other_subject,
        excluded_subject,
        teacher,
        other_teacher,
        week_pattern,
        slot,
        other_slot,
        lone_slot,
        other_subject_slot,
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
/// **Exactly one break**: the moved week is `bare_week`, which carries no
/// colloscope cell, so none of the interrogation-versus-period variants have
/// anything to say about it. That is why the fixture holds a week for this test
/// alone. Every other week carries a cell, and moving one of *those* is not
/// surgical: the cell's group bound is read from the association at the week's
/// period, a dead period has none, so the bound falls to 0 and
/// `InterrogationGroupOutOfBounds` fires alongside the dangle.
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
        .move_week_entry(doc.bare_week, doc.dead_period, 0);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::DanglingFk(Reference::Period {
            target: doc.dead_period,
            site: PeriodRefSite::WeekPeriodFk(doc.bare_week),
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
    // Every live member is carried across, so the edit really is "one member
    // added" rather than a rewritten row.
    corrupt.params.assignments.map.insert(
        (doc.period, doc.subject),
        BTreeSet::from([
            doc.student,
            doc.other_student,
            doc.excluded_student,
            doc.dead_student,
        ]),
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

// ---- `Convergence`, rows 1-4: the slot and teacher block ----
//
// The dangling-FK half is behind us. From here the invariant is a *predicate*
// over live rows rather than a reference, and that changes what an innocent
// twin has to look like. The change is the same for all sixteen `Convergence`
// rows, so it is worth stating once, here.
//
// A `Convergence` variant names an offending *configuration*: a row together
// with the field values that make it offending. The arm never re-checks the
// predicate (rule 1) and pins only what its fix is about to destroy (rule 5) —
// for these four rows, a slot's `teacher_id`, its `subject_id`, its
// `start_time`, or a subject's membership in a teacher's `subjects` set. So an
// innocent twin has to differ from the valid document in **exactly the field
// the arm compares**, and nowhere else.
//
// That rules out the corruption that first comes to mind in three of the four
// rows. Turning a subject's interrogations off does make rows 2 and 3 fire —
// but it leaves the teacher still teaching that subject and the slot still
// sitting on it, so the *valid* document carries the very shape the arm tests
// for, and `Some` is the right answer there. Said the short way: disabling a
// subject's interrogations is the cascade route this whole step exists for, so
// a document on it is by definition one the map is *supposed* to repair — it
// can never be the innocent half of anything. The twin has to move the **row**
// instead: give the slot another teacher, put it on another subject, move its
// start time, add a subject to the teacher's set.
//
// Read the other way round, this is the whole point of the series restated for
// layer C: the two sides of a `Convergence` predicate are not
// interchangeable. One of them is the row the fix destroys, and that is the
// side an innocent-state test must vary.
//
// **The unit is one test per comparison, not one per arm.** A twin varies one
// field, and the arm then answers `None` because of that one field — so any
// second comparison the arm makes is left untouched, and a version of the arm
// that dropped it would pass. Row 1 compares two fields and therefore gets two
// tests. Rows 2, 3 and 4 compare one each. (Row 10 in the next block and row 16
// in the last one compare two as well; §8.1 had the same situation with pairing
// rules and solved it by making the two parts two separate *sites*, which is
// why they already have a test each.)

/// `Convergence::SlotTeacherDoesNotTeachSubject` — §8.2 row 1, **teacher half**.
///
/// The fix is `Slot::Remove(slot)`, which names only the slot, so both of the
/// arm's comparisons are pure identity tests and each needs its own twin. This
/// one varies the teacher, which §8.2 calls the load-bearing comparison; the
/// next test varies the subject.
///
/// The corruption is row 1's reachable route, reached here by surgery: the slot
/// is pointed at `other_teacher`, who is alive and teaches `other_subject`
/// only. A user gets to the same state with `SlotOp::Update` rewriting a slot's
/// teacher over a document that was fine; the gate rolls that update back, and
/// the map is then asked about the slot as it stood before.
///
/// **Exactly one break**: the teacher resolves, so the check runs rather than
/// skipping, and fires. The slot's subject is untouched and still runs
/// interrogations, so rows 3 and 4 have nothing to say about it, and the second
/// teacher's own subject runs interrogations too, so row 2 stays quiet.
///
/// Without the identity test the answer here would be
/// `Some(Slot::Remove(lone_slot))` merely because the slot exists — deleting a
/// slot whose teacher is precisely the one who should be teaching it.
#[test]
fn slot_teacher_does_not_teach_subject_arm_spares_a_slot_whose_teacher_teaches_it() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let live_slot = corrupt
        .params
        .slots
        .find_slot(doc.lone_slot)
        .expect("the fixture's lone slot is there")
        .clone();
    // `replace_slot`: the subject is unchanged, so the slot ordering — which is
    // keyed by subject — stays consistent.
    corrupt.params.slots.replace_slot(
        doc.lone_slot,
        Slot {
            teacher_id: doc.other_teacher,
            ..live_slot
        },
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::SlotTeacherDoesNotTeachSubject(
            doc.lone_slot,
            doc.other_teacher,
            doc.subject,
        )),
        "the live slot's teacher is not the named one, so the arm has no slot to remove",
    );
}

/// `Convergence::SlotTeacherDoesNotTeachSubject` — §8.2 row 1, **subject half**.
///
/// The twin above leaves the arm's second comparison untested: it varies the
/// teacher, so an arm that had dropped the `subject_id` comparison would still
/// answer `None` and pass. This twin closes that. It moves the slot onto
/// `other_subject`, which `teacher` does not teach, and leaves the teacher
/// alone — so in the valid document the teacher matches the invariant exactly,
/// and `None` can only come from the subject comparison.
///
/// §8.2 calls that comparison defensive, and on today's code it is: a slot's
/// subject cannot change through an op, so the invariant can only carry a
/// mismatched subject via `Op::GlobalUpdate`. That is a statement about how
/// reachable the route is, not a reason to leave the comparison unpinned — the
/// module docs already settle the same argument for arms whose `Some` branch
/// cannot fire, and testing follows the arm.
///
/// The move changes what subject the slot belongs to, so it goes through
/// `remove_slot` + `insert_slot_at` like row 3's twin: the slot ordering is
/// keyed by subject, and a raw write would desync the mirror.
///
/// **Exactly one break**: `other_subject` runs interrogations, so row 3 stays
/// quiet; an hour from 16:00 does not overflow the day, so row 4 does; and
/// `lone_slot` sits in no pairing rule and no colloscope cell.
#[test]
fn slot_teacher_does_not_teach_subject_arm_spares_a_slot_on_another_subject() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (_position, live_slot) = corrupt.params.slots.remove_slot(doc.lone_slot);
    corrupt.params.slots.insert_slot_at(
        doc.lone_slot,
        Slot {
            subject_id: doc.other_subject,
            ..live_slot
        },
        0,
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::SlotTeacherDoesNotTeachSubject(
            doc.lone_slot,
            doc.teacher,
            doc.other_subject,
        )),
        "the live slot's teacher matches, but its subject is not the named one, \
         so the arm has no slot to remove",
    );
}

/// `Convergence::TeacherSubjectWithoutInterrogations` — §8.2 row 2, and the one
/// row of this block whose fix keeps the row alive: `Teacher::subjects` is a
/// set, so the element leaves and the teacher stays.
///
/// The fix is `Teacher::Update(teacher, rebuilt)`, which names the teacher but
/// not the subject being dropped, so rule 4 asks for an identity test — and
/// frame point 4's corollary supplies it for free, since the membership test an
/// element-removal rebuild needs *is* the identity test. This is the test that
/// the arm really makes it.
///
/// The corruption adds `excluded_subject`, which runs no interrogations, to
/// `other_teacher`'s set. It cannot be done the other way round: a subject
/// whose interrogations are off is one no valid document lets any teacher
/// teach, so switching a subject off would leave the *valid* document carrying
/// the offending membership.
///
/// The teacher chosen already teaches something, so the valid document is not
/// innocent by holding an empty set: an arm that asked "does this teacher teach
/// anything at all" would find `other_subject` and strip it.
///
/// **Exactly one break**: `other_teacher` teaches one slot,
/// `other_subject_slot`, and the corruption only *adds* to their set, so that
/// slot's subject is still in it and the slots loop finds nothing. Nothing else
/// in `convergence_breaks` reads a teacher's set.
#[test]
fn teacher_subject_without_interrogations_arm_spares_a_teacher_of_live_subjects() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .teachers
        .teacher_map
        .get_mut(&doc.other_teacher)
        .expect("the fixture's second teacher is there")
        .subjects
        .insert(doc.excluded_subject);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::TeacherSubjectWithoutInterrogations(
            doc.other_teacher,
            doc.excluded_subject,
        )),
        "the live teacher does not teach the named subject, so the arm has nothing to drop",
    );
}

/// `Convergence::SlotForSubjectWithoutInterrogations` — §8.2 row 3, and **the
/// second two-element exception** (§9bis).
///
/// No state where this fires breaks only one invariant: the slot's teacher
/// either teaches the offending subject — row 2 — or does not — row 1 — or
/// dangles, and then the `SlotTeacher` dangle fires. That is §8.2 row 3's
/// shadowing argument, and it holds on a corrupted twin exactly as it does on a
/// live state. So the expected literal is a **two-element set**, still
/// hand-derived and still `assert_eq!`d whole, and step 4 runs on the element
/// the test is about, picked out by shape.
///
/// Deriving the companion is worth the trouble for one reason only: the set is
/// asserted *whole*, so the literal has to name it. Nothing here depends on
/// which of the two sorts first — this test calls `fix_invariant` directly on
/// the element it picked, and the canonical pick is the engine's business, which
/// no test in this module touches.
///
/// **The companion is row 1, and it cannot be anything else.** §9bis predicted
/// row 2 and prescribed "turn the subject's `interrogation_parameters` to
/// `None`". That recipe does not produce an innocent state at all: disabling a
/// subject's interrogations is the legitimate cascade route itself, so the
/// document it makes is one the map is *supposed* to repair — the slot still
/// sits on the very subject the invariant names, and the arm rightly answers
/// `Some`. An innocent twin has to move the slot onto a
/// *different* subject that runs no interrogations — and no valid document ever
/// lets a teacher teach such a subject, so the teacher-teaches check is
/// guaranteed to fire beside row 3. The slot's teacher is kept live, which
/// makes that companion the deterministic one.
///
/// The slot moved is the one referenced by nothing else, and the surgery goes
/// through `remove_slot` + `insert_slot_at`: the slot ordering is keyed by
/// subject, so a raw field write would leave the slot filed under its old
/// subject and the twin would die at step 3 with a `LogicError`.
#[test]
fn slot_for_subject_without_interrogations_arm_spares_a_slot_on_a_live_subject() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    let (_position, live_slot) = corrupt.params.slots.remove_slot(doc.lone_slot);
    corrupt.params.slots.insert_slot_at(
        doc.lone_slot,
        Slot {
            subject_id: doc.excluded_subject,
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
            FixableInvariant::Convergence(Convergence::SlotTeacherDoesNotTeachSubject(
                doc.lone_slot,
                doc.teacher,
                doc.excluded_subject,
            )),
            FixableInvariant::Convergence(Convergence::SlotForSubjectWithoutInterrogations(
                doc.lone_slot,
                doc.excluded_subject,
            )),
        ])
    );
    let invariant = set
        .into_iter()
        .find(|invariant| {
            matches!(
                invariant,
                FixableInvariant::Convergence(Convergence::SlotForSubjectWithoutInterrogations(..))
            )
        })
        .expect("the set was just asserted to hold row 3");

    assert_eq!(
        valid.fix_invariant(&invariant),
        None,
        "the live slot sits on a subject that does run interrogations, so the arm has no slot to remove"
    );
}

/// `Convergence::SlotOverflowsDay` — §8.2 row 4, and the arm frame point 5's
/// corollary was written for: it tests `start` and deliberately does **not**
/// test `duration`.
///
/// So `start` is the only field this test can vary, and it is the one it
/// varies: the twin moves the slot to 23:30, where the subject's one-hour
/// interrogation runs past midnight. The invariant's `duration` is the live
/// one, unchanged — which is what makes the answer rest entirely on the start
/// comparison.
///
/// The case the arm must *not* reject is the mirror image: the subject's
/// interrogation is lengthened, so the live subject still holds the old
/// duration while the live slot still holds the offending start, and the fix
/// has to land. That is the legitimate cascade route, it belongs to the
/// commit-7 scenarios, and pinning `duration` here would break it.
///
/// **Exactly one break**: the slot keeps its teacher and its subject, and it is
/// referenced by nothing — no week pattern, no colloscope cell, no pairing
/// rule — so a start time is all that changes.
#[test]
fn slot_overflows_day_arm_spares_a_slot_that_starts_elsewhere() {
    let (valid, doc) = build_valid_document();

    // 23:30 plus the fixture's one-hour interrogation crosses midnight. (23:00
    // would not: a slot ending exactly at 00:00 is valid.)
    let late = collomatique_time::SlotStart {
        weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
        start_time: collomatique_time::WholeMinuteTime::new(
            chrono::NaiveTime::from_hms_opt(23, 30, 0).unwrap(),
        )
        .unwrap(),
    };

    let mut corrupt = valid.get_inner_data().clone();
    let live_slot = corrupt
        .params
        .slots
        .find_slot(doc.lone_slot)
        .expect("the fixture's lone slot is there")
        .clone();
    corrupt.params.slots.replace_slot(
        doc.lone_slot,
        Slot {
            start_time: late.clone(),
            ..live_slot
        },
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::SlotOverflowsDay {
            slot: doc.lone_slot,
            start: late,
            duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
        }),
        "the live slot starts hours earlier, so the arm has no overflowing slot to remove",
    );
}

// ---- `Convergence`, rows 5-8: assignments rows and group-list associations ----
//
// Four rows, and §8.2 calls three of them coordinate-shaped: the invariant
// names a coordinate, the fix op carries that same coordinate, and the fix
// removes the whole thing sitting at it. So there is no field left to compare
// and no identity test to write — the arm's entire content is the lookup, and
// the offending shape simply *is* the presence of the row or the entry. Row 6
// is the one variation, and its membership test is an identity test that comes
// for free (frame point 4).
//
// Rows 7 and 8 share a single match arm, since both name the same offending
// configuration and both clear it. They still get a test each: what differs is
// which predicate the *checker* fires, and a twin that produced the wrong one
// would be asserting about an invariant nobody meant.
//
// The block-1 rule applies unchanged — vary the row, not the predicate's other
// side — and it bites hardest here, because for a coordinate-shaped arm the
// only bug a `None` test can catch is a mis-keyed or missing lookup. Without a
// live neighbour sharing one half of the corrupt coordinate, an arm that
// dropped the other half would sail through every test in this block. Five
// fixture rows exist for exactly that: three assignments rows and two extra
// associations.
//
// Which halves are coverable is decided by the predicate, and the rule is
// mechanical: **a neighbour keeping half H fixed exists only if a valid
// document may hold that H again — so whatever the predicate reads on its own
// is uncoverable.**
//
// - Rows 5 and 8 read the subject and the period *together*. Relational, so
//   neither half alone is condemned and both witnesses are buildable. Both
//   twins have both.
// - Row 7 reads the subject alone. Any association naming a subject without
//   interrogations *is* row 7, so the subject half has no witness at any
//   fixture. The period half is covered.
// - Row 6 reads the student and the period. Any assignments row on that period
//   holding that student *is* row 6, so the period half has no witness. The
//   subject half is covered.

/// `Convergence::AssignmentForSubjectNotRunningOnPeriod` — §8.2 row 5.
///
/// The twin puts a row at `(other_period, excluded_subject)`, a coordinate
/// where the subject already excludes the period. That is the corruption the
/// block-1 rule asks for: the offending configuration is "a row exists **and**
/// the subject excludes the period", and adding the row varies the side the arm
/// tests. Widening the subject's exclusions instead would leave the fixture's
/// own row offending and make the valid document guilty.
///
/// The valid document holds no row at that coordinate — but it holds a
/// neighbour on each half of it: `(other_period, subject)` for the period,
/// `(period, excluded_subject)` for the subject. So the arm cannot pass by
/// having nothing to look at, and an arm that dropped either half of the key
/// would find one of those rows and clear something nobody complained about.
///
/// **Exactly one break**: the row's students exclude no period, so row 6 does
/// not join, and every id in it is live.
#[test]
fn assignment_for_subject_not_running_on_period_arm_spares_a_missing_row() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt.params.assignments.map.insert(
        (doc.other_period, doc.excluded_subject),
        BTreeSet::from([doc.student, doc.other_student]),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::AssignmentForSubjectNotRunningOnPeriod(
            doc.other_period,
            doc.excluded_subject,
        )),
        "the live document has no row at that coordinate, so the arm has nothing to clear",
    );
}

/// `Convergence::AssignedStudentNotPresentForPeriod` — §8.2 row 6, and the one
/// row of this block whose fix keeps the row alive.
///
/// The op is `SetRow(period, subject, rebuilt)`: it names the row but not the
/// member being dropped, so rule 4 asks for an identity test, and the
/// membership test the element-removal rebuild needs already is one. This test
/// is that the arm makes it.
///
/// The twin adds `excluded_student`, who excludes `other_period`, to the
/// fixture's row on `other_period`. So the arm's lookup **succeeds** on the
/// valid document and finds a real row with real members — only the membership
/// test stands between an innocent student and being silently unassigned.
///
/// She is also a member of the fixture's row on `period`, where she is present.
/// That is the innocent witness for the *other* bug shape: an arm that searched
/// every row for the named student, rather than reading the named coordinate,
/// would find that row and unassign her from a term nobody complained about.
///
/// **The subject half of the coordinate has no innocent witness**, and the
/// proof is the same shape as row 7's: a witness would be a row on
/// `other_period` holding her, and that row *is* this invariant. The predicate
/// depends on the pair `(student, period)` alone — the subject plays no part —
/// so the period is the only half a valid document can vary.
///
/// **Exactly one break**: the row's subject excludes no period, so row 5 stays
/// quiet, and the two students already in the row exclude nothing.
#[test]
fn assigned_student_not_present_for_period_arm_spares_a_row_of_present_students() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt.params.assignments.map.insert(
        (doc.other_period, doc.subject),
        BTreeSet::from([doc.student, doc.other_student, doc.excluded_student]),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::AssignedStudentNotPresentForPeriod {
            period: doc.other_period,
            subject: doc.subject,
            student: doc.excluded_student,
        }),
        "the live row does not hold the named student, so the arm has nobody to unassign",
    );
}

/// `Convergence::AssociationForSubjectWithoutInterrogations` — §8.2 row 7.
///
/// The twin associates a group list to `(period, excluded_subject)`: the entry
/// is what is added, and the subject's interrogations — the other side of the
/// predicate — are left exactly as the fixture built them.
///
/// The valid document holds no entry at that coordinate, and this is the arm's
/// whole content: `AssignToSubject(period, subject, None)` carries the
/// coordinate, so no identity test is expressible and the lookup is the only
/// thing keeping the arm from unassigning a live association or emitting a
/// perfect no-op.
///
/// **The subject half of the coordinate has no innocent witness**, and cannot
/// have one: a live association naming a subject without interrogations *is*
/// this invariant. The predicate reads the subject alone, so that half is
/// uncoverable at any fixture — a proof, not a gap left open. The period half
/// is covered: the fixture holds two entries on `period`, so an arm that
/// ignored the subject would find one and detach it.
///
/// **Exactly one break**: `excluded_subject` excludes `other_period`, not
/// `period`, so row 8 does not join; the list associated has no colloscope row,
/// so the colloscope block stays out of it.
#[test]
fn association_for_subject_without_interrogations_arm_spares_a_missing_entry() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .group_lists
        .subjects_associations
        .insert((doc.period, doc.excluded_subject), doc.excluding_group_list);

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::AssociationForSubjectWithoutInterrogations(
            doc.period,
            doc.excluded_subject,
        )),
        "the live document has no entry at that coordinate, so the arm has nothing to unassign",
    );
}

/// `Convergence::AssociationForSubjectNotRunningOnPeriod` — §8.2 row 8, which
/// shares its match arm with row 7 and is tested separately because the
/// *checker* is what tells the two apart.
///
/// The twin associates a group list to `(other_period, other_subject)`, the one
/// coordinate in the fixture where a subject that runs interrogations is
/// excluded on the period. That is what keeps row 7 out of the set: a subject
/// with interrogations disabled would fire both.
///
/// Unlike row 7, this coordinate has a live neighbour on **both** halves, and
/// that is not luck — row 8's predicate is *relational*, reading the subject
/// and the period together, so neither half alone determines it and both
/// witnesses are buildable. `(period, other_subject)` shares the subject and
/// catches an arm that ignored the period; `(other_period, subject)` shares the
/// period and catches an arm that ignored the subject. Either mistake finds a
/// live entry and clears an association nothing complained about.
///
/// **Exactly one break**: `other_subject` runs interrogations, so row 7 stays
/// quiet, and it hosts no slot, so nothing in the colloscope block is
/// downstream of the entry.
#[test]
fn association_for_subject_not_running_on_period_arm_spares_a_missing_entry() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt.params.group_lists.subjects_associations.insert(
        (doc.other_period, doc.other_subject),
        doc.excluding_group_list,
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::AssociationForSubjectNotRunningOnPeriod(
            doc.other_period,
            doc.other_subject,
        )),
        "the live document has no entry at that coordinate, so the arm has nothing to unassign",
    );
}

// ---- `Convergence`, rows 9-12: balancing, pairing rules and colloscope cells ----
//
// Four rows, five tests: row 10 compares two fields, so it gets one test per
// comparison, exactly as row 1 did.
//
// All three site shapes of the series show up here, one per group:
//
// - **Row 9 is a pure key site.** The subject *is* the whole coordinate, and
//   `SetSubject(subject, None)` carries it, so no identity test is expressible
//   and the arm's entire content is one lookup. It joins the dangling-FK half's
//   three pure key sites — `BalancingSubjectKey`, `SettingsStudentKey` and
//   `ColloscopeGroupListKey`.
// - **Row 10 is a row site with two fields.** The fix removes the rule outright
//   and names only the rule, so both comparisons are pure identity tests and
//   each needs its own twin.
// - **Rows 11 and 12 are a two-half coordinate**, sharing one match arm the way
//   rows 7 and 8 do, and tested separately because the *checker* is what tells
//   them apart.
//
// Both colloscope rows are relational — each reads the slot and the week
// together — so by the block-2 rule both halves of `(slot, week)` are coverable,
// and the fixture now covers all four. That is what the three extra colloscope
// cells are for, along with the week in `other_period` and `other_subject_slot`,
// the fixture's only slot outside `subject`.
//
// **A finding §8.2 does not record: row 11 can never break alone.** Its
// predicate says the cell's slot sits on a subject that excludes the week's
// period. The group-number bound for that same cell is read from the association
// at `(the week's period, the slot's subject)` — and an association *there* is
// exactly row 8, so no valid document holds one. The bound therefore falls to
// its missing-association default of 0, every group number in the cell is out of
// bounds, and `InterrogationGroupOutOfBounds` fires beside row 11 every time.
// Row 11's test is a two-element-set test for that reason, like row 3's. Unlike
// row 3 this is not a shadowing: row 11 is declared *before* row 13, so it
// remains the canonical pick and its `Some` branch stays reachable.

/// `Convergence::BalancingForSubjectWithoutInterrogations` — §8.2 row 9.
///
/// The twin gives `excluded_subject`, which runs no interrogations, a
/// per-subject balancing override. The block-1 rule fixes the direction:
/// switching a subject's interrogations off instead would leave the fixture's
/// *own* override — the one on `subject` — offending, and the valid document
/// would be guilty.
///
/// The coordinate has a single half, so there is no neighbour to build here and
/// nothing to prove uncoverable. What the fixture does supply is the other bug
/// shape a pure key site is exposed to: the valid document's override table is
/// **not empty**, so an arm that asked "does this document have any per-subject
/// override at all" would find `subject`'s, answer `Some`, and emit
/// `SetSubject(excluded_subject, None)` against a state holding no such entry —
/// a perfect no-op, which the engine answers with a panic.
///
/// **Exactly one break**: the balancing loop is the only place a per-subject
/// override key is read, and `excluded_subject` is live, so nothing dangles.
#[test]
fn balancing_for_subject_without_interrogations_arm_spares_a_missing_override() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .params
        .balancing
        .subjects
        .insert(doc.excluded_subject, BalancingOptions::default());

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::BalancingForSubjectWithoutInterrogations(
            doc.excluded_subject,
        )),
        "the live document holds no override for that subject, so the arm has nothing to drop",
    );
}

/// `Convergence::PairedSlotsNotInSameSubject` — §8.2 row 10, **antecedent half**.
///
/// The fix is `SlotPairing::Remove(rule)`, which names only the rule, so both of
/// the arm's comparisons are pure identity tests and each needs its own twin.
/// This one varies the antecedent; the next varies the consequent.
///
/// The corruption points the antecedent at `other_subject_slot`, the fixture's
/// only slot outside `subject`. It has to be a **live** slot: the checker's
/// predicate is gated on both slots resolving (`invariants.rs:550-557`), so a
/// dead one would make it skip and report a dangle instead. And it has to sit on
/// another subject, because that mismatch *is* the predicate — the corruption
/// cannot come from the subject side, since moving one of the fixture's own
/// slots would leave the live rule offending and make the valid document guilty.
///
/// The consequent is left alone, so in the valid document it matches the
/// invariant exactly and `None` can only come from the antecedent comparison.
///
/// **Exactly one break**: nothing outside the slot-pairing loop reads a pairing
/// rule, both slots are live, and the rule's excluded period is untouched.
#[test]
fn paired_slots_not_in_same_subject_arm_spares_a_rule_with_another_antecedent() {
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
    antecedent.slot_id = doc.other_subject_slot;
    corrupt.params.slot_pairings.slot_pairing_rule_map.insert(
        doc.slot_pairing,
        SlotPairingRule::new(antecedent, consequent, excluded_periods, soft).expect(
            "the second subject's slot is not the consequent's, so the parts stay distinct",
        ),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::PairedSlotsNotInSameSubject(
            doc.slot_pairing,
            doc.other_subject_slot,
            doc.other_slot,
        )),
        "the live rule's antecedent is not the named slot, so the arm has no rule to remove",
    );
}

/// `Convergence::PairedSlotsNotInSameSubject` — §8.2 row 10, **consequent
/// half**, and the mirror of the test above: the antecedent is left alone and
/// matches the invariant, so `None` can only come from the consequent
/// comparison.
///
/// Deleting a rule is the most destructive fix in this block, and each half of
/// the rule can go wrong on its own — which is exactly the argument §8.1 made
/// when it split the two pairing parts into two separate *sites*. Row 10 keeps
/// both parts inside one variant, so the split happens in the tests instead.
#[test]
fn paired_slots_not_in_same_subject_arm_spares_a_rule_with_another_consequent() {
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
    consequent.slot_id = doc.other_subject_slot;
    corrupt.params.slot_pairings.slot_pairing_rule_map.insert(
        doc.slot_pairing,
        SlotPairingRule::new(antecedent, consequent, excluded_periods, soft).expect(
            "the second subject's slot is not the antecedent's, so the parts stay distinct",
        ),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::PairedSlotsNotInSameSubject(
            doc.slot_pairing,
            doc.slot,
            doc.other_subject_slot,
        )),
        "the live rule's consequent is not the named slot, so the arm has no rule to remove",
    );
}

/// `Convergence::InterrogationSlotNotRunningOnPeriod` — §8.2 row 11, and **the
/// second two-element-set test of the series**, for the reason the block comment
/// proves: the association that would bound this cell's group numbers is exactly
/// row 8, so no valid document holds one, the bound is 0, and
/// `InterrogationGroupOutOfBounds` always fires beside row 11.
///
/// The set is asserted whole, so the literal has to name both; the element under
/// test is then selected explicitly. Row 11 happens to sort first, but nothing
/// here rests on that — the test calls `fix_invariant` on the element it picked.
///
/// The twin puts a cell at `(other_subject_slot, excluded_period_week)`:
/// `other_subject` excludes `other_period`, which is that week's period. The
/// corruption is the cell, and the exclusion — the predicate's other side — is
/// left exactly as the fixture built it.
///
/// Both halves of the coordinate have a live neighbour, and that is not luck:
/// the predicate is relational, reading the slot's subject and the week's period
/// together, so neither half alone condemns a cell.
/// `(other_subject_slot, week)` shares the slot and catches an arm that ignored
/// the week; `(other_slot, excluded_period_week)` shares the week and catches an
/// arm that ignored the slot. Either mistake clears a colloscope cell nobody
/// complained about, which for a colloscope means losing placements made by
/// hand.
///
/// **Row 12 stays quiet**: `other_subject_slot` wears no week pattern, so every
/// week is active for it.
#[test]
fn interrogation_slot_not_running_on_period_arm_spares_a_missing_cell() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt.colloscope.set_interrogation(
        doc.other_subject_slot,
        doc.excluded_period_week,
        BTreeSet::from([0]),
    );

    let set = corrupt
        .broken_invariants()
        .expect("the corruption is fixable, not a logic error");
    assert_eq!(
        set,
        BTreeSet::from([
            FixableInvariant::Convergence(Convergence::InterrogationSlotNotRunningOnPeriod(
                doc.other_subject_slot,
                doc.excluded_period_week,
            )),
            FixableInvariant::Convergence(Convergence::InterrogationGroupOutOfBounds(
                doc.other_subject_slot,
                doc.excluded_period_week,
                0,
            )),
        ])
    );
    let invariant = set
        .into_iter()
        .find(|invariant| {
            matches!(
                invariant,
                FixableInvariant::Convergence(Convergence::InterrogationSlotNotRunningOnPeriod(..))
            )
        })
        .expect("the set was just asserted to hold row 11");

    assert_eq!(
        valid.fix_invariant(&invariant),
        None,
        "the live document has no cell at that coordinate, so the arm has nothing to clear",
    );
}

/// `Convergence::InterrogationOnInactiveWeek` — §8.2 row 12, which shares its
/// match arm with row 11 and is tested separately because the *checker* is what
/// tells the two apart.
///
/// The twin puts a cell at `(slot, other_week)`. `slot` is the fixture's only
/// patterned slot and its pattern excludes exactly `other_week`, so the cell
/// sits on a week that is inactive for it. Varying the pattern instead would
/// leave the fixture's own cell on `week` offending, and the valid document
/// would be guilty.
///
/// Unlike row 11 this row breaks alone: the bound comes from the association at
/// `(period, subject)`, which the fixture holds, so group 0 is in bounds. Week
/// activity has nothing to do with the association, which is what lets row 12 be
/// surgical where row 11 cannot.
///
/// Both halves have a live neighbour. The fixture's first cell, `(slot, week)`,
/// shares the slot; `(other_slot, other_week)` shares the week, and is innocent
/// because `other_slot` wears no pattern at all — a week no pattern excludes is
/// active for it. That neighbour exists only because the predicate is
/// relational: "inactive" is a fact about a week *and* a slot's pattern
/// together, never about the week alone.
///
/// **Row 11 stays quiet**: `subject` excludes no period.
#[test]
fn interrogation_on_inactive_week_arm_spares_a_missing_cell() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .colloscope
        .set_interrogation(doc.slot, doc.other_week, BTreeSet::from([0]));

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::InterrogationOnInactiveWeek(
            doc.slot,
            doc.other_week,
        )),
        "the live document has no cell at that coordinate, so the arm has nothing to clear",
    );
}

// ---- `Convergence`, rows 13-16: the colloscope block ----
//
// Four rows, five tests: row 16 compares two things, so it gets one test per
// comparison, like rows 1 and 10.
//
// This block is where the block-1 rule — vary the row the fix destroys, never
// the predicate's other side — stops being merely the way to build an innocent
// twin and becomes the arms' actual specification. §8.2 proves it for row 14 and
// the proof carries to 15 and 16. The offending configuration has **two routes**:
// the op writes a row onto a list that is already prefilled, or the op flips an
// existing list to prefilled while a row sits on it. On the first route the
// pre-op state has no row, the presence test fails, and the engine convicts the
// op — right. On the second the pre-op row is a real, innocent row that the arm
// must clear, which is what legacy does too. So an arm that read prefilled-ness
// from `self` would reject an edit legacy accepts. Reading the predicate is not
// merely unnecessary here; it would be **wrong**.
//
// The tests inherit that directly: every twin below adds or edits the *row*, and
// leaves the list's prefilled-ness, its excluded set and its group count exactly
// as the fixture built them.
//
// The two element-removal rows, 15 and 16, emit `SetGroupList(gl, rebuilt)`,
// which names the list but not the student leaving it. Rule 4 therefore asks for
// an identity test, and the membership test the rebuild needs already is one —
// so both twins add their student to a row that **already exists**, leaving the
// membership test as the only thing that can answer `None`. A missing identity
// test here does not destroy a row; it silently unplaces an innocent student,
// which is worse in a different way, because the resulting colloscope looks
// perfectly ordinary.

/// `Convergence::InterrogationGroupOutOfBounds` — §8.2 row 13.
///
/// The op is `SetInterrogation(slot, week, cell minus group)`: it carries the
/// coordinate but not the group number being dropped, so the membership test is
/// the identity test (frame point 4).
///
/// The twin adds group 2 to the fixture's own cell, whose list has two groups
/// and so bounds them at 0 and 1. The cell is edited, never the bound: shrinking
/// the group list instead is the legitimate cascade route this row exists for,
/// and on that route the live document really does hold the offending cell.
///
/// The arm must **not** re-check the bound, and that is not an oversight — after
/// a repaired shrink the group reads as in-bounds again while the trim is still
/// needed. So the only thing standing between an innocent cell and a silent trim
/// is the membership test, and the twin makes the lookup succeed so that the
/// membership test is what answers.
///
/// **Exactly one break**: the cell's week is active for the slot's pattern and
/// its subject excludes no period, so rows 11 and 12 stay quiet.
#[test]
fn interrogation_group_out_of_bounds_arm_spares_a_cell_of_in_bounds_groups() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .colloscope
        .set_interrogation(doc.slot, doc.week, BTreeSet::from([0, 2]));

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::InterrogationGroupOutOfBounds(
            doc.slot, doc.week, 2,
        )),
        "the live cell does not hold the named group, so the arm has nothing to trim",
    );
}

/// `Convergence::ColloscopeGroupListPrefilled` — §8.2 row 14, a pure key site:
/// the group list *is* the coordinate, `SetGroupList(gl, ∅)` carries it, and the
/// arm's whole content is one lookup. For a prefilled list there is no single
/// element to blame — the offending thing is the whole row.
///
/// The twin puts a placements row on `prefilled_group_list`. The direction is
/// forced twice over: by the block-1 rule, and by the argument above that
/// reading prefilled-ness from `self` would be wrong.
///
/// The valid document is not innocent by being empty: it carries a colloscope
/// row for `group_list`, so an arm asking "are there any colloscope group-list
/// rows at all" would find one, answer `Some`, and emit
/// `SetGroupList(prefilled_group_list, ∅)` against a state with no such row — a
/// perfect no-op, which the engine answers with a panic.
///
/// **Exactly one break**: a prefilled filling reports an empty excluded set
/// (`group_lists.rs:152-162`), so row 15 stays quiet, and the list has two
/// groups, so group 0 is in bounds and row 16 does too.
#[test]
fn colloscope_group_list_prefilled_arm_spares_a_missing_row() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt
        .colloscope
        .set_group_list(doc.prefilled_group_list, BTreeMap::from([(doc.student, 0)]));

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::ColloscopeGroupListPrefilled(
            doc.prefilled_group_list,
        )),
        "the live document has no colloscope row for that list, so the arm has nothing to clear",
    );
}

/// `Convergence::ColloscopeStudentExcluded` — §8.2 row 15.
///
/// The twin adds `excluded_student` to the placements of
/// `excluding_group_list`, which is the list that excludes her. The row already
/// exists in the fixture — that is what it was added for — so the arm's lookup
/// **succeeds** on the valid document and finds real placements. Only the
/// membership test stands between an innocent student and being silently
/// unplaced.
///
/// The excluded set is left alone, and must be: adding a student to it is the
/// legitimate cascade route, and on that route the live document really does
/// hold the offending placement. That is also why the arm never reads the
/// excluded set — the presence test has to serve both routes, cleaning the
/// placement on one and rejecting the op on the other.
///
/// **Exactly one break**: the list is automatic, so row 14 stays quiet, and it
/// has two groups, so group 0 is in bounds and row 16 does too.
#[test]
fn colloscope_student_excluded_arm_spares_a_row_of_included_students() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    // Every live placement is carried across, so the edit really is "one student
    // added" rather than a rewritten row.
    corrupt.colloscope.set_group_list(
        doc.excluding_group_list,
        BTreeMap::from([
            (doc.student, 0),
            (doc.other_student, 1),
            (doc.excluded_student, 0),
        ]),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::ColloscopeStudentExcluded(
            doc.excluding_group_list,
            doc.excluded_student,
        )),
        "the live row does not place the named student, so the arm has nobody to unplace",
    );
}

/// `Convergence::ColloscopeStudentGroupOutOfBounds` — §8.2 row 16, **group
/// half**.
///
/// The arm's test is one expression, `placements.get(student) != Some(group)`,
/// asserting two things: the student is placed at all, and placed in *that*
/// group. Each needs its own twin. This one varies the group; the next varies
/// who is placed.
///
/// The twin moves `student` from group 0 to group 2, which is out of bounds for
/// a two-group list. So in the valid document she **is** placed — the presence
/// half of the test is satisfied — and `None` can come from nowhere but the
/// group comparison.
///
/// The bound is left alone, as everywhere in this block: shrinking the group
/// list is the legitimate route, and on it the live document really does hold
/// the offending placement.
///
/// **Exactly one break**: `group_list` excludes nobody, so row 15 stays quiet,
/// and it is automatic, so row 14 does. The colloscope cells are unaffected —
/// their bound is the list's group count, which the twin does not touch.
#[test]
fn colloscope_student_group_out_of_bounds_arm_spares_a_student_placed_elsewhere() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt.colloscope.set_group_list(
        doc.group_list,
        BTreeMap::from([(doc.student, 2), (doc.other_student, 1)]),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::ColloscopeStudentGroupOutOfBounds(
            doc.group_list,
            doc.student,
            2,
        )),
        "the live row places the named student in another group, so the arm has nobody to unplace",
    );
}

/// `Convergence::ColloscopeStudentGroupOutOfBounds` — §8.2 row 16, **presence
/// half**, and the mirror of the test above.
///
/// The twin places `excluded_student` — who is in no colloscope row at all — in
/// `group_list`, out of bounds at group 2. So the valid document's row exists
/// and holds real placements, but not hers, and `None` can come only from the
/// presence half of the same expression.
///
/// Without it the arm would rebuild the row minus a student who was never in it,
/// which is a perfect no-op, and the engine answers a no-op fix with a panic in
/// front of the user.
///
/// **Exactly one break**: `group_list` excludes nobody, so row 15 stays quiet
/// even though the student added is the one another list excludes.
#[test]
fn colloscope_student_group_out_of_bounds_arm_spares_a_row_that_places_somebody_else() {
    let (valid, doc) = build_valid_document();

    let mut corrupt = valid.get_inner_data().clone();
    corrupt.colloscope.set_group_list(
        doc.group_list,
        BTreeMap::from([
            (doc.student, 0),
            (doc.other_student, 1),
            (doc.excluded_student, 2),
        ]),
    );

    assert_arm_finds_nothing(
        &valid,
        &corrupt,
        FixableInvariant::Convergence(Convergence::ColloscopeStudentGroupOutOfBounds(
            doc.group_list,
            doc.excluded_student,
            2,
        )),
        "the live row does not place the named student at all, so the arm has nobody to unplace",
    );
}

// ---- The two `GlobalUpdate` policy pins ----
//
// These two are not four-step tests and do run the engine. They close §9bis,
// and they pin a policy rather than an arm: **a `GlobalUpdate` carrying a
// corrupt document is rejected whole, never cleaned.** That is D4 applied to a
// whole-document target, and it is user-visible, because an import takes exactly
// that shape.
//
// One test per *half of the map* is enough, and deliberately so. Such a test
// cannot see a missing shape test: a `GlobalUpdate` payload that is corrupt is
// corrupt whatever the live document looks like, so no repair to the live
// document can ever make the retry succeed. A sloppy arm merely churns — repair
// the innocent row, retry, same break, material now gone, `None`, convict — and
// since the engine restores its entry snapshot on failure, the caller sees the
// same `Err` and the same untouched document either way. What these pin is the
// outcome, not the route.
//
// Each reuses a twin from above verbatim, sent as a payload instead of being
// asked about. That is the point of the pairing: the tests above prove the arm
// answers `None` for these very documents, and these two show what the engine
// then does with that `None` when the target is the whole document.

/// A `GlobalUpdate` whose payload holds a dangling reference is rejected, and
/// the document is left bit-identical.
///
/// The payload is the twin of
/// `assignments_student_arm_spares_a_row_of_live_students`: a valid document
/// plus one dead student in an assignments row. The engine applies it, the gate
/// rolls it back, the arm is asked about the dangle on the (now restored) live
/// document and answers `None` — which is exactly what that test proves — so
/// the target is convicted and the entry snapshot is restored.
#[test]
fn global_update_with_a_dangling_reference_is_rejected_whole() {
    let (mut data, doc) = build_valid_document();
    let before = data.clone();

    let mut corrupt = data.get_inner_data().clone();
    corrupt.params.assignments.map.insert(
        (doc.period, doc.subject),
        BTreeSet::from([
            doc.student,
            doc.other_student,
            doc.excluded_student,
            doc.dead_student,
        ]),
    );

    let err = apply_cascade(&mut data, AnnotatedOp::GlobalUpdate(corrupt))
        .expect_err("a corrupt GlobalUpdate payload must be rejected");

    match err {
        ApplyError::BrokenInvariants(set) => assert_eq!(
            set,
            BTreeSet::from([FixableInvariant::DanglingFk(Reference::Student {
                target: doc.dead_student,
                site: StudentRefSite::AssignmentsStudent {
                    period: doc.period,
                    subject: doc.subject,
                },
            })]),
            "the error carries the target's own break, not a fix's"
        ),
        other => panic!("expected BrokenInvariants, got {other:?}"),
    }
    assert!(
        data == before,
        "a rejected GlobalUpdate must leave the document bit-identical: \
         the cascade cleans the live document to make a *target* land, never to \
         make a corrupt payload acceptable"
    );
}

/// The `Convergence` half of the same policy: a `GlobalUpdate` whose payload
/// breaks a convergence invariant is rejected whole too.
///
/// The payload is the twin of
/// `interrogation_on_inactive_week_arm_spares_a_missing_cell`: a valid document
/// plus one colloscope cell on a week the slot's pattern excludes. Both halves
/// of the map are covered by these two, and neither can see more than the
/// outcome — see the block comment.
#[test]
fn global_update_breaking_a_convergence_invariant_is_rejected_whole() {
    let (mut data, doc) = build_valid_document();
    let before = data.clone();

    let mut corrupt = data.get_inner_data().clone();
    corrupt
        .colloscope
        .set_interrogation(doc.slot, doc.other_week, BTreeSet::from([0]));

    let err = apply_cascade(&mut data, AnnotatedOp::GlobalUpdate(corrupt))
        .expect_err("a corrupt GlobalUpdate payload must be rejected");

    match err {
        ApplyError::BrokenInvariants(set) => assert_eq!(
            set,
            BTreeSet::from([FixableInvariant::Convergence(
                Convergence::InterrogationOnInactiveWeek(doc.slot, doc.other_week)
            )]),
            "the error carries the target's own break, not a fix's"
        ),
        other => panic!("expected BrokenInvariants, got {other:?}"),
    }
    assert!(
        data == before,
        "a rejected GlobalUpdate must leave the document bit-identical"
    );
}
