//! Invariant vocabulary for the precise whole-model checker.
//!
//! This module defines *what can be broken*, in three kinds, classified
//! mechanically (plan §3, pinned at `git show 49b4f77d:docs/plans/plan_step_2.md`):
//!
//! - [FixableInvariant::DanglingFk] — the edge is in the refs registry
//!   ([crate::InnerData::for_each_reference]) and its target id does not
//!   resolve. Fixed by removing or clearing the referencing data.
//! - [LogicError] — truth decidable from a row's *own value* (or, for
//!   [LogicError::DuplicatedId], a whole-document id-uniqueness property; or,
//!   for the ordering variants, whether an ordering sidecar agrees with its
//!   entity table): no *other* entity's state can flip it, so no legitimate
//!   elementary op can produce it by side effect. Only buggy code (or a
//!   hand-forged file) can.
//!   Not fixable: consumers panic (cascade) or hard-error (decode).
//! - [Convergence] — a predicate over *existing* edges that legitimate ops can
//!   break indirectly (e.g. `UpdateSubject` turning interrogations off breaks
//!   every "subject has interrogations" referrer). The cascade resolves these
//!   lossily (clear the now-invalid data).
//!
//! `FixableInvariant = DanglingFk | Convergence` is the `Ok` payload of the
//! checker; `LogicError` is the `Err` payload and short-circuits: dangling and
//! convergence sweeps cannot be trusted over a state whose own rows are
//! malformed, so rather than return a fixable set it cannot vouch for, the
//! checker reports only the logic errors. Consequence (intended): `Err` says
//! nothing about dangles or convergence breaks that may co-occur — the API
//! refuses to guess rather than lie.
//!
//! ## Canonical order
//!
//! The checker returns `BTreeSet`s; `Ord` is derived on every type here, so
//! **declaration order is the canonical order**. [FixableInvariant::DanglingFk]
//! is declared before [FixableInvariant::Convergence] so that when a row is
//! both dangling and convergence-broken, `min()` picks the precise row-removal
//! fix over the lossy one.
//!
//! Inside [Convergence] one placement has been decided on purpose, and is
//! written down here because nothing in the type says it: **the
//! interrogation-row predicates are declared before the association ones**. An
//! interrogation row's group numbers are bounded by the group list associated
//! at its `(period, subject)` coordinate. So a repair that clears the
//! association takes that bound to zero, and every group of every cell at that
//! coordinate becomes its own [Convergence::InterrogationGroupOutOfBounds]
//! break — the cells then die one group at a time, described to the user as
//! « le groupe N sera retiré » rather than as the loss of the colle. Repairing
//! the rows first spares them that, and costs nothing here: an interrogation
//! row is downstream data, so clearing one cannot invalidate an association.
//!
//! That is a judgement about this pair, and about the sentences it produces —
//! not a principle the rest of the order was derived from. Every other
//! placement is as it was declared at step 6.
//!
//! The checker ([crate::InnerData]`::broken_invariants`) lives here too, in
//! three layers: the logic-error sweep (layer A, the `Err` path), the
//! dangling-reference sweep (layer B) and the convergence sweep (layer C), the
//! last two together forming the `Ok` payload. Layer C skips a predicate
//! whenever an FK *lookup it needs to read data* fails (the matching layer-B
//! [FixableInvariant::DanglingFk] already reports that dangle); an id used only
//! as a compared value does not gate. Where the old first-error checker
//! fail-fasts, layer C accumulates, so every broken row surfaces.
//!
//! ## Deliberate non-checks (confirmed in the step-3 completeness audit)
//!
//! - An [crate::incompats::Incompatibility]'s subject is *not* required to run
//!   interrogations (see the `subject_id` field docs for why) — the one
//!   subject reference without a "has interrogations" convergence predicate.
//! - The slots and weeks ordering↔table mirrors *are* validated here, by the
//!   layer-A sweeps, as [LogicError]s (a desync is unreachable through any op —
//!   the compound mutators keep both containers in lockstep, force ops included;
//!   only the test-only `forge_ordering_row` hatch can split them — so it is
//!   code-at-fault, exactly the [LogicError] contract). The one mirror fact left
//!   *out* is row-key liveness: an ordering row keyed by a removed period (or
//!   subject) is the op-reachable dangle that layer B owns as
//!   [FixableInvariant::DanglingFk] and the cascade repairs, so promoting it to
//!   a short-circuiting [LogicError] would block that repair.
//! - The id-issuer high-water check lives in `Data::assert_id_issuer_high_water`:
//!   the issuer is `Data`-level state outside [crate::InnerData], so it stays a
//!   separate companion to `broken_invariants`.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::ids::{
    GroupListId, PeriodId, SlotId, SlotPairingRuleId, StudentId, SubjectId, TeacherId, WeekId,
    WeekPatternId,
};
use crate::refs::Reference;

/// A state no legitimate elementary op can reach: the *code* (or a hand-forged
/// file) is at fault, not the data. Truth is decidable from the row's own value
/// (or, for [LogicError::DuplicatedId], whole-document id uniqueness; or, for
/// the ordering variants, consistency of an ordering sidecar with its entity
/// table) — see the module docs for the classification rule.
///
/// Declaration order is the canonical order (derived `Ord`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Error)]
pub enum LogicError {
    /// A raw id used by two different entities across the shared `u64`
    /// namespace
    #[error("raw id {0} is used by two different entities")]
    DuplicatedId(u64),
    /// A stored assignments row with an empty student set (rows are
    /// canonical-absent: a row exists iff it holds an assigned student)
    #[error("assignments row ({0:?}, {1:?}) is stored with an empty student set")]
    EmptyAssignmentsRow(PeriodId, SubjectId),
    /// A stored slots-ordering row with an empty slot list (a row exists iff
    /// the subject has ≥ 1 slot)
    #[error("slots-ordering row for subject {0:?} is stored with an empty slot list")]
    EmptySlotsRow(SubjectId),
    /// A slots-ordering row lists a slot id that is absent from the slot table
    #[error("slots-ordering row for subject {0:?} lists unknown slot {1:?}")]
    SlotOrderingUnknownId(SubjectId, SlotId),
    /// A slot sits in the ordering row of a subject other than its own
    /// `slot.subject_id` (the row-key/entity mismatch)
    #[error("slot {1:?} is filed under subject {0:?} but names another subject")]
    SlotOrderingWrongSubject(SubjectId, SlotId),
    /// A slot id appears more than once across the ordering rows
    #[error("slot {0:?} appears more than once in the slots ordering")]
    SlotOrderingDuplicate(SlotId),
    /// A slot table entry is covered by no ordering row
    #[error("slot {0:?} exists in the slot table but is absent from the ordering")]
    OrphanSlot(SlotId),
    /// A stored weeks-ordering row with an empty week list (a row exists iff
    /// the period has ≥ 1 week)
    #[error("weeks-ordering row for period {0:?} is stored with an empty week list")]
    EmptyWeeksRow(PeriodId),
    /// A weeks-ordering row lists a week id that is absent from the week table
    #[error("weeks-ordering row for period {0:?} lists unknown week {1:?}")]
    WeekOrderingUnknownId(PeriodId, WeekId),
    /// A week sits in the ordering row of a period other than its own
    /// `week.period_id` (the row-key/entity mismatch)
    #[error("week {1:?} is filed under period {0:?} but names another period")]
    WeekOrderingWrongPeriod(PeriodId, WeekId),
    /// A week id appears more than once across the ordering rows
    #[error("week {0:?} appears more than once in the weeks ordering")]
    WeekOrderingDuplicate(WeekId),
    /// A week table entry is covered by no ordering row
    #[error("week {0:?} exists in the week table but is absent from the ordering")]
    OrphanWeek(WeekId),
    /// A stored colloscope interrogation row with an empty group set (rows are
    /// canonical-absent: a row exists iff it holds an assigned group)
    #[error("colloscope interrogation row ({0:?}, {1:?}) is stored with an empty group set")]
    EmptyInterrogationRow(SlotId, WeekId),
    /// A stored colloscope group-list row with an empty placement map (a row
    /// exists iff it holds a placement)
    #[error("colloscope group-list row {0:?} is stored with an empty placement map")]
    EmptyColloscopeGroupListRow(GroupListId),
}

/// A predicate over *existing* edges that legitimate ops can break indirectly —
/// see the module docs for the classification rule. The step-6 cascade resolves
/// these lossily (clear the now-invalid data). Every predicate skips when a
/// prerequisite reference dangles: the [FixableInvariant::DanglingFk] entry
/// already reports that.
///
/// Declaration order is the canonical order (derived `Ord`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Error)]
pub enum Convergence {
    /// The slot's teacher's `subjects` set lacks the slot's subject
    #[error("teacher {1:?} of slot {0:?} does not teach the slot's subject {2:?}")]
    SlotTeacherDoesNotTeachSubject(SlotId, TeacherId, SubjectId),
    /// A teacher references a subject whose interrogations are disabled
    #[error("teacher {0:?} references subject {1:?} which has interrogations disabled")]
    TeacherSubjectWithoutInterrogations(TeacherId, SubjectId),
    /// A slot on a subject whose interrogations are disabled
    #[error("slot {0:?} is on subject {1:?} which has interrogations disabled")]
    SlotForSubjectWithoutInterrogations(SlotId, SubjectId),
    /// The slot's start time plus its subject's interrogation duration
    /// overflows the day
    #[error("slot {slot:?} starts at {start} and lasts {duration}, overflowing its day")]
    SlotOverflowsDay {
        slot: SlotId,
        start: collomatique_time::SlotStart,
        duration: collomatique_time::NonZeroMinutes,
    },
    /// An assignments row whose subject excludes the row's period
    #[error("assignments row ({0:?}, {1:?}): the subject does not run on the period")]
    AssignmentForSubjectNotRunningOnPeriod(PeriodId, SubjectId),
    /// An assigned student who excludes the row's period
    #[error(
        "student {student:?} assigned in ({period:?}, {subject:?}) is not present for the period"
    )]
    AssignedStudentNotPresentForPeriod {
        period: PeriodId,
        subject: SubjectId,
        student: StudentId,
    },
    // The three interrogation-row predicates are declared *before* the two
    // association ones, and that is load-bearing rather than tidy — see the
    // module docs' "Canonical order" section. An interrogation row's group
    // numbers are bounded by the group list associated at its `(period,
    // subject)` coordinate, so clearing that association takes the bound to
    // zero and turns every group in every cell there into a separate
    // `InterrogationGroupOutOfBounds` break. Repairing the rows first spares
    // the user that: the cells go whole, each with the sentence it deserves.
    /// An interrogation whose slot's subject excludes the week's period
    #[error("interrogation ({0:?}, {1:?}): the slot's subject does not run on the week's period")]
    InterrogationSlotNotRunningOnPeriod(SlotId, WeekId),
    /// An interrogation on a week the slot's week pattern deactivates
    #[error("interrogation ({0:?}, {1:?}) is on an inactive week")]
    InterrogationOnInactiveWeek(SlotId, WeekId),
    /// An interrogation assigning a group number ≥ the associated group list's
    /// group count — one entry per offending group number
    #[error("interrogation ({0:?}, {1:?}) assigns out-of-bounds group number {2}")]
    InterrogationGroupOutOfBounds(SlotId, WeekId, u32),
    /// A group-list association whose subject has interrogations disabled
    #[error("association ({0:?}, {1:?}): the subject has interrogations disabled")]
    AssociationForSubjectWithoutInterrogations(PeriodId, SubjectId),
    /// A group-list association whose subject excludes the period
    #[error("association ({0:?}, {1:?}): the subject does not run on the period")]
    AssociationForSubjectNotRunningOnPeriod(PeriodId, SubjectId),
    /// A balancing entry for a subject whose interrogations are disabled
    #[error("balancing entry for subject {0:?} which has interrogations disabled")]
    BalancingForSubjectWithoutInterrogations(SubjectId),
    /// A slot pairing rule whose two slots are on different subjects
    #[error("slot pairing rule {0:?} pairs slots {1:?} and {2:?} of different subjects")]
    PairedSlotsNotInSameSubject(SlotPairingRuleId, SlotId, SlotId),
    /// A colloscope row for a prefilled group list
    #[error("colloscope holds a row for prefilled group list {0:?}")]
    ColloscopeGroupListPrefilled(GroupListId),
    /// A placed student who is in the automatic filling's excluded set
    #[error("colloscope group list {0:?} places excluded student {1:?}")]
    ColloscopeStudentExcluded(GroupListId, StudentId),
    /// A placed student with a group number ≥ the list's group count —
    /// the third field is that offending group number
    #[error("colloscope group list {0:?} places student {1:?} in out-of-bounds group {2}")]
    ColloscopeStudentGroupOutOfBounds(GroupListId, StudentId, u32),
}

/// A broken invariant the *data* is responsible for — the `Ok` payload of the
/// checker. Fixed by removing or clearing the referencing data; the step-6
/// cascade's resolution map is total over this type, so every consumer matches
/// both variants exhaustively (no variant is "the panicking one").
///
/// [FixableInvariant::DanglingFk] is declared first so that when a row is both
/// dangling and convergence-broken, `BTreeSet::first()` picks the precise
/// row-removal fix over the lossy one (derived `Ord`, declaration order).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Error)]
pub enum FixableInvariant {
    /// A reference whose target id does not resolve
    #[error("dangling reference: {0:?}")]
    DanglingFk(Reference),
    /// A broken convergence predicate
    #[error(transparent)]
    Convergence(Convergence),
}

impl crate::InnerData {
    /// Returns every broken invariant of the document, deduplicated, in
    /// canonical order (the derived `Ord`s — see the module docs).
    ///
    /// `Ok` means the *code* is sound: the payload is what the *data* needs
    /// fixed (empty = fully valid). `Err` means a logic error — a state no
    /// legitimate elementary op can reach — and short-circuits: a logic error
    /// undermines the meaningfulness of the fixable sweep.
    ///
    /// The three layers: the logic-error sweep (layer A, the `Err` path), the
    /// dangling-reference sweep (layer B) and the convergence sweep (layer C).
    /// Layers B and C together form the `Ok` payload; every layer-C predicate
    /// skips a row whose data-reading lookup dangles (layer B already reports
    /// that edge), so both layers coexist on the same state.
    pub fn broken_invariants(&self) -> Result<BTreeSet<FixableInvariant>, BTreeSet<LogicError>> {
        let logic_errors = self.logic_errors();
        if !logic_errors.is_empty() {
            return Err(logic_errors);
        }
        let mut fixable = self.dangling_refs();
        fixable.extend(
            self.convergence_breaks()
                .into_iter()
                .map(FixableInvariant::Convergence),
        );
        Ok(fixable)
    }

    /// Layer A: every [LogicError] in the document — a state no legitimate
    /// elementary op can reach (see [LogicError] for the classification rule).
    /// Each check is decidable from a row's own value (or, for the duplicate-id
    /// sweep, whole-document id uniqueness), so no reference-resolution guard is
    /// needed and the sweep is exhaustive: unlike the old first-error checker,
    /// every broken row is reported, and both prefill predicates can fire on the
    /// same group list. A non-empty result short-circuits [Self::broken_invariants]
    /// as `Err` — a logic error undermines the meaningfulness of the fixable sweep.
    fn logic_errors(&self) -> BTreeSet<LogicError> {
        let mut errors = BTreeSet::new();

        // Duplicate raw ids across the shared `u64` namespace. The retired old
        // check (`InnerData::check_no_duplicate_ids`) was a bool; here each
        // colliding raw id is reported (an id reused three times still yields
        // one entry — the set dedups).
        let mut seen = BTreeSet::new();
        for id in self.ids() {
            if !seen.insert(id) {
                errors.insert(LogicError::DuplicatedId(id));
            }
        }

        // Canonical-absent rows: a stored row exists iff it is non-empty.
        for (period, subject, students) in self.params.assignments.iter() {
            if students.is_empty() {
                errors.insert(LogicError::EmptyAssignmentsRow(period, subject));
            }
        }
        // Ordering↔table mirror (slots). Row-key liveness is deliberately NOT
        // checked here: a row keyed by a removed subject is the op-reachable
        // dangling state, reported per slot as DanglingFk(SlotSubjectFk) in
        // layer B and repaired by the cascade. Everything else about the mirror
        // is a code bug — every listed id must exist in the slot table, name the
        // subject that keys its row, appear exactly once, and cover every slot.
        let mut seen_slots = BTreeSet::new();
        for (subject, order) in self.params.slots.ordering_entries() {
            if order.is_empty() {
                errors.insert(LogicError::EmptySlotsRow(subject));
            }
            for slot_id in order {
                match self.params.slots.find_slot(*slot_id) {
                    None => {
                        errors.insert(LogicError::SlotOrderingUnknownId(subject, *slot_id));
                    }
                    Some(slot) => {
                        if slot.subject_id != subject {
                            errors.insert(LogicError::SlotOrderingWrongSubject(subject, *slot_id));
                        }
                    }
                }
                if !seen_slots.insert(*slot_id) {
                    errors.insert(LogicError::SlotOrderingDuplicate(*slot_id));
                }
            }
        }
        for (slot_id, _slot) in self.params.slots.slot_entries() {
            if !seen_slots.contains(&slot_id) {
                errors.insert(LogicError::OrphanSlot(slot_id));
            }
        }

        // Ordering↔table mirror (weeks), the exact twin of the slots sweep
        // above. Row-key liveness (a row keyed by a removed period) stays out
        // for the same reason: it is the op-reachable DanglingFk(WeekPeriodFk)
        // the cascade repairs.
        let mut seen_weeks = BTreeSet::new();
        for (period, order) in self.params.weeks.ordering_entries() {
            if order.is_empty() {
                errors.insert(LogicError::EmptyWeeksRow(period));
            }
            for week_id in order {
                match self.params.weeks.find_week(*week_id) {
                    None => {
                        errors.insert(LogicError::WeekOrderingUnknownId(period, *week_id));
                    }
                    Some(week) => {
                        if week.period_id != period {
                            errors.insert(LogicError::WeekOrderingWrongPeriod(period, *week_id));
                        }
                    }
                }
                if !seen_weeks.insert(*week_id) {
                    errors.insert(LogicError::WeekOrderingDuplicate(*week_id));
                }
            }
        }
        for (week_id, _week) in self.params.weeks.week_entries() {
            if !seen_weeks.contains(&week_id) {
                errors.insert(LogicError::OrphanWeek(week_id));
            }
        }
        for ((slot, week), groups) in self.colloscope.iter() {
            if groups.is_empty() {
                errors.insert(LogicError::EmptyInterrogationRow(slot, week));
            }
        }
        for (group_list, placements) in self.colloscope.group_lists_iter() {
            if placements.is_empty() {
                errors.insert(LogicError::EmptyColloscopeGroupListRow(group_list));
            }
        }

        // (Prefilled group lists cannot violate the count/duplicate invariants:
        // `GroupList::new` enforces them by construction, so no state can hold a
        // mismatched or duplicate-student filling. Likewise a pairing rule cannot
        // have both parts on one subject, and a slot pairing rule cannot have
        // both parts on one slot: `PairingRule::new` and `SlotPairingRule::new`
        // enforce it by construction — there is nothing to sweep for any of
        // these.)

        errors
    }

    /// Layer B: every registry edge ([Self::for_each_reference]) whose target
    /// id does not resolve, as [FixableInvariant::DanglingFk] entries.
    ///
    /// The eight existence sets are read from the entities' own tables (not the
    /// ordering sidecars), so the sweep stays sound on potentially inconsistent
    /// data. `Week@WeekPeriodFk` — a week whose `period_id` points at an absent
    /// period — fires when `force_apply` removes a period that still has weeks
    /// (the guard was dropped from the force path; the week rows and ordering
    /// sidecar are left dangling for the cascade).
    fn dangling_refs(&self) -> BTreeSet<FixableInvariant> {
        let periods: BTreeSet<PeriodId> = self.params.periods.period_ids().collect();
        let weeks: BTreeSet<WeekId> = self
            .params
            .weeks
            .week_entries()
            .map(|(id, _week)| id)
            .collect();
        let subjects: BTreeSet<SubjectId> =
            self.params.subjects.ordered_subject_list.keys().collect();
        let teachers: BTreeSet<TeacherId> = self.params.teachers.teacher_map.keys().collect();
        let students: BTreeSet<StudentId> = self.params.students.student_map.keys().collect();
        let week_patterns: BTreeSet<WeekPatternId> =
            self.params.week_patterns.week_pattern_map.keys().collect();
        let slots: BTreeSet<SlotId> = self.params.slots.slot_ids().collect();
        let group_lists: BTreeSet<GroupListId> =
            self.params.group_lists.group_list_map.keys().collect();

        let mut dangling = BTreeSet::new();
        self.for_each_reference(&mut |reference| {
            let resolves = match reference {
                Reference::Period { target, .. } => periods.contains(&target),
                Reference::Week { target, .. } => weeks.contains(&target),
                Reference::Subject { target, .. } => subjects.contains(&target),
                Reference::Teacher { target, .. } => teachers.contains(&target),
                Reference::Student { target, .. } => students.contains(&target),
                Reference::WeekPattern { target, .. } => week_patterns.contains(&target),
                Reference::Slot { target, .. } => slots.contains(&target),
                Reference::GroupList { target, .. } => group_lists.contains(&target),
            };
            if !resolves {
                dangling.insert(FixableInvariant::DanglingFk(reference));
            }
        });
        dangling
    }

    /// Layer C: every broken [Convergence] predicate — a check over *existing*
    /// edges that legitimate ops can break indirectly (see [Convergence]).
    ///
    /// Every predicate skips (`continue`, or a guarded `if let`) when a lookup
    /// it needs to *read data* fails: the matching [FixableInvariant::DanglingFk]
    /// entry already reports that dangle, so layers B and C coexist. An id used
    /// only as a *compared value* does not gate — e.g. the teacher-teaches check
    /// runs even when the slot's subject id dangles. Unlike the old first-error
    /// checker this sweep accumulates: every broken row (and every true
    /// predicate on a row) is reported.
    fn convergence_breaks(&self) -> BTreeSet<Convergence> {
        let mut out = BTreeSet::new();
        let params = &self.params;

        // ---- Slots: teacher-teaches, subject-has-interrogations, day overflow.
        // Mirrors `validate_slot_internal`. The teacher-teaches check gates only
        // on the teacher resolving (the subject id is compared, not read); the
        // overflow check gates on the subject *and* its interrogation parameters
        // being present (the duration lives there — a `None` fires
        // `SlotForSubjectWithoutInterrogations` instead).
        for (slot_id, slot) in params.slots.slot_entries() {
            if let Some(teacher) = params.teachers.teacher_map.get(&slot.teacher_id)
                && !teacher.subjects.contains(&slot.subject_id)
            {
                out.insert(Convergence::SlotTeacherDoesNotTeachSubject(
                    slot_id,
                    slot.teacher_id,
                    slot.subject_id,
                ));
            }
            if let Some(subject) = params.subjects.find_subject(slot.subject_id) {
                match &subject.parameters.interrogation_parameters {
                    None => {
                        out.insert(Convergence::SlotForSubjectWithoutInterrogations(
                            slot_id,
                            slot.subject_id,
                        ));
                    }
                    Some(interrogation_params) => {
                        if collomatique_time::SlotWithDuration::new(
                            slot.start_time.clone(),
                            interrogation_params.duration,
                        )
                        .is_none()
                        {
                            out.insert(Convergence::SlotOverflowsDay {
                                slot: slot_id,
                                start: slot.start_time.clone(),
                                duration: interrogation_params.duration,
                            });
                        }
                    }
                }
            }
        }

        // ---- Teachers: every subject a teacher teaches must run interrogations.
        // Mirrors `validate_teacher_internal`.
        for (teacher_id, teacher) in params.teachers.teacher_map.iter() {
            for &subject_id in &teacher.subjects {
                let Some(subject) = params.subjects.find_subject(subject_id) else {
                    continue;
                };
                if subject.parameters.interrogation_parameters.is_none() {
                    out.insert(Convergence::TeacherSubjectWithoutInterrogations(
                        teacher_id, subject_id,
                    ));
                }
            }
        }

        // ---- Assignments rows: the subject runs on the period, and every
        // assigned student is present for it. Mirrors what the retired
        // `check_assignments_data_consistency` checked (the empty-row case is
        // layer A).
        for (period_id, subject_id, students) in params.assignments.iter() {
            if let Some(subject) = params.subjects.find_subject(subject_id)
                && subject.excluded_periods.contains(&period_id)
            {
                out.insert(Convergence::AssignmentForSubjectNotRunningOnPeriod(
                    period_id, subject_id,
                ));
            }
            for student_id in students {
                let Some(student) = params.students.student_map.get(student_id) else {
                    continue;
                };
                if student.excluded_periods.contains(&period_id) {
                    out.insert(Convergence::AssignedStudentNotPresentForPeriod {
                        period: period_id,
                        subject: subject_id,
                        student: *student_id,
                    });
                }
            }
        }

        // ---- Association rows: the subject runs interrogations and is not
        // excluded on the period. Mirrors what the retired
        // `check_group_lists_data_consistency` checked; both predicates
        // accumulate (the old checker stopped at the first).
        for ((period_id, subject_id), _group_list_id) in
            params.group_lists.subjects_associations.iter()
        {
            let Some(subject) = params.subjects.find_subject(subject_id) else {
                continue;
            };
            if subject.parameters.interrogation_parameters.is_none() {
                out.insert(Convergence::AssociationForSubjectWithoutInterrogations(
                    period_id, subject_id,
                ));
            }
            if subject.excluded_periods.contains(&period_id) {
                out.insert(Convergence::AssociationForSubjectNotRunningOnPeriod(
                    period_id, subject_id,
                ));
            }
        }

        // ---- Balancing keys: every override subject must run interrogations.
        // Mirrors `validate_balancing`.
        for subject_id in params.balancing.subjects.keys() {
            let Some(subject) = params.subjects.find_subject(subject_id) else {
                continue;
            };
            if subject.parameters.interrogation_parameters.is_none() {
                out.insert(Convergence::BalancingForSubjectWithoutInterrogations(
                    subject_id,
                ));
            }
        }

        // ---- Slot pairings: the two paired slots must be on the same subject
        // (the same-slot degeneracy is unrepresentable — `SlotPairingRule::new`
        // enforces it by construction). Mirrors
        // `validate_slot_pairing_rule_internal`. Gated on both slots resolving;
        // the subject ids are only compared, so they do not gate.
        for (rule_id, rule) in params.slot_pairings.slot_pairing_rule_map.iter() {
            if let (Some((ant_subject, _)), Some((con_subject, _))) = (
                params
                    .slots
                    .find_slot_with_subject(rule.antecedent().slot_id),
                params
                    .slots
                    .find_slot_with_subject(rule.consequent().slot_id),
            ) && ant_subject != con_subject
            {
                out.insert(Convergence::PairedSlotsNotInSameSubject(
                    rule_id,
                    rule.antecedent().slot_id,
                    rule.consequent().slot_id,
                ));
            }
        }

        // ---- Colloscope interrogation rows. Mirrors what the retired
        // `validate_against_params` checked: the slot's subject runs on the
        // week's period, the week is active for the slot's pattern, and every
        // group number fits the association bound.
        for ((slot_id, week_id), groups) in self.colloscope.iter() {
            let period = params.weeks.week_position(week_id).map(|(p, _pos)| p);
            let slot = params.slots.find_slot_with_subject(slot_id);

            // Subject-excludes-period half of the old `SlotNotRunningOnPeriod`
            // (the interrogations-off half is covered per-slot by
            // `SlotForSubjectWithoutInterrogations`).
            if let (Some(period_id), Some((subject_id, _))) = (period, slot)
                && let Some(subject) = params.subjects.find_subject(subject_id)
                && subject.excluded_periods.contains(&period_id)
            {
                out.insert(Convergence::InterrogationSlotNotRunningOnPeriod(
                    slot_id, week_id,
                ));
            }

            // A dangling week pattern counts as "no exclusion" (`is_week_active`),
            // matching the old checker; layer B reports the dangle itself.
            if period.is_some()
                && let Some((_, slot_desc)) = slot
                && !params.is_week_active(week_id, slot_desc.week_pattern)
            {
                out.insert(Convergence::InterrogationOnInactiveWeek(slot_id, week_id));
            }

            // Group-number bound. A missing association means bound 0 (the old
            // code's `.unwrap_or(0)`); an association to a *dangling* group
            // list is skipped — the old code `.expect`ed it live, we cannot.
            if let (Some(period_id), Some((subject_id, _))) = (period, slot) {
                let bound = match params
                    .group_lists
                    .subjects_associations
                    .get(&(period_id, subject_id))
                {
                    None => Some(0u32),
                    Some(group_list_id) => params
                        .group_lists
                        .group_list_map
                        .get(group_list_id)
                        .map(|gl| gl.params().group_names.len() as u32),
                };
                if let Some(bound) = bound {
                    for &group_num in groups {
                        if group_num >= bound {
                            out.insert(Convergence::InterrogationGroupOutOfBounds(
                                slot_id, week_id, group_num,
                            ));
                        }
                    }
                }
            }
        }

        // ---- Colloscope group-list rows. Mirrors what the retired
        // `validate_against_params` + `validate_group_list_placements` checked:
        // the list must not be prefilled, and every placement must name a
        // non-excluded student in an in-bounds group.
        for (group_list_id, placements) in self.colloscope.group_lists_iter() {
            let Some(group_list) = params.group_lists.group_list_map.get(&group_list_id) else {
                continue;
            };
            if group_list.is_prefilled() {
                out.insert(Convergence::ColloscopeGroupListPrefilled(group_list_id));
            }
            let excluded = group_list.filling().excluded_students();
            let bound = group_list.params().group_names.len() as u32;
            for (&student_id, &group_num) in placements {
                if excluded.contains(&student_id) {
                    out.insert(Convergence::ColloscopeStudentExcluded(
                        group_list_id,
                        student_id,
                    ));
                }
                if group_num >= bound {
                    out.insert(Convergence::ColloscopeStudentGroupOutOfBounds(
                        group_list_id,
                        student_id,
                        group_num,
                    ));
                }
            }
        }

        out
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::InnerData;
    use crate::balancing::BalancingOptions;
    use crate::group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup};
    use crate::ids::{Id, IncompatId, PairingRuleId};
    use crate::incompats::Incompatibility;
    use crate::pairings::{PairingRule, RulePart};
    use crate::periods::Periods;
    use crate::refs::{
        GroupListRefSite, PeriodRefSite, Reference, SlotRefSite, StudentRefSite, SubjectRefSite,
        TeacherRefSite, WeekPatternRefSite, WeekRefSite,
    };
    use crate::settings::Limits;
    use crate::slot_pairings::{SlotPairingRule, SlotRulePart};
    use crate::slots::{Slot, Slots};
    use crate::students::Student;
    use crate::subjects::{Subject, SubjectParameters};
    use crate::teachers::Teacher;
    use crate::week_patterns::WeekPattern;
    use crate::weeks::{WeekDesc, Weeks};
    use collomatique_time::{SlotStart, WholeMinuteTime};
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;

    /// A minimal well-formed slot on the given subject/teacher. Its `week_pattern`
    /// is `None`; callers override the fields they want to make dangle.
    fn test_slot(subject_id: SubjectId, teacher_id: TeacherId) -> Slot {
        Slot {
            subject_id,
            teacher_id,
            start_time: SlotStart {
                weekday: chrono::Weekday::Mon.into(),
                start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap())
                    .unwrap(),
            },
            extra_info: String::new(),
            week_pattern: None,
            cost: 0,
        }
    }

    /// Shorthand for [crate::InnerData::broken_invariants]: every fixture below
    /// asserts on the checker through this wrapper. (Until step-5 R1.5 it also
    /// ran the old-vs-new differential on each fixture's state; the old checker
    /// retired with step 5.)
    fn broken_invariants(
        data: &InnerData,
    ) -> Result<BTreeSet<FixableInvariant>, BTreeSet<LogicError>> {
        data.broken_invariants()
    }

    // ---- Layer B: the dangling-reference sweep ----
    //
    // Each per-kind test registers just enough host entities that *exactly* the
    // intended reference dangles, then asserts exact set equality on the whole
    // `Ok(...)` — not mere membership. Ids are forged via `unsafe { Id::new(n) }`
    // (test-only corruption); the fixtures reach the pub map fields / pub
    // constructors directly, bypassing the ops that would reject a dangling id.

    #[test]
    fn dangling_period_in_student_exclusions() {
        let mut data = InnerData::default();
        let student = unsafe { StudentId::new(1) };
        let period = unsafe { PeriodId::new(2) };
        data.params.students.student_map.insert(
            student,
            Student {
                excluded_periods: BTreeSet::from([period]),
                ..Default::default()
            },
        );
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::Period {
                    target: period,
                    site: PeriodRefSite::StudentExcludedPeriods(student),
                }
            )]))
        );
    }

    #[test]
    fn dangling_week_in_week_pattern() {
        let mut data = InnerData::default();
        let pattern = unsafe { WeekPatternId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.params.week_patterns.week_pattern_map.insert(
            pattern,
            WeekPattern {
                name: "P".into(),
                excluded_weeks: BTreeSet::from([week]),
            },
        );
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::Week {
                    target: week,
                    site: WeekRefSite::WeekPatternExcludedWeek(pattern),
                }
            )]))
        );
    }

    #[test]
    fn dangling_subject_in_teacher() {
        let mut data = InnerData::default();
        let teacher = unsafe { TeacherId::new(1) };
        let subject = unsafe { SubjectId::new(2) };
        data.params.teachers.teacher_map.insert(
            teacher,
            Teacher {
                subjects: BTreeSet::from([subject]),
                ..Default::default()
            },
        );
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::Subject {
                    target: subject,
                    site: SubjectRefSite::TeacherSubjects(teacher),
                }
            )]))
        );
    }

    #[test]
    fn dangling_teacher_in_slot() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let slot = unsafe { SlotId::new(2) };
        let teacher = unsafe { TeacherId::new(3) };
        // Register the subject so only the teacher dangles.
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
        data.params.slots =
            Slots::from_subject_rows([(subject, vec![(slot, test_slot(subject, teacher))])])
                .unwrap();
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::Teacher {
                    target: teacher,
                    site: TeacherRefSite::SlotTeacher(slot),
                }
            )]))
        );
    }

    #[test]
    fn dangling_student_in_settings_key() {
        let mut data = InnerData::default();
        let student = unsafe { StudentId::new(1) };
        data.params
            .settings
            .students
            .insert(student, Limits::default());
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::Student {
                    target: student,
                    site: StudentRefSite::SettingsStudentKey,
                }
            )]))
        );
    }

    #[test]
    fn dangling_week_pattern_in_slot() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let slot = unsafe { SlotId::new(2) };
        let teacher = unsafe { TeacherId::new(3) };
        let pattern = unsafe { WeekPatternId::new(4) };
        // Register subject and teacher so only the week pattern dangles. The
        // teacher must teach the slot's subject, else layer C would also fire
        // `SlotTeacherDoesNotTeachSubject`.
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
        data.params.teachers.teacher_map.insert(
            teacher,
            Teacher {
                subjects: BTreeSet::from([subject]),
                ..Default::default()
            },
        );
        let mut slot_desc = test_slot(subject, teacher);
        slot_desc.week_pattern = Some(pattern);
        data.params.slots = Slots::from_subject_rows([(subject, vec![(slot, slot_desc)])]).unwrap();
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::WeekPattern {
                    target: pattern,
                    site: WeekPatternRefSite::SlotWeekPattern(slot),
                }
            )]))
        );
    }

    #[test]
    fn dangling_slots_in_slot_pairing_yield_distinct_sites() {
        // Both parts forged: the antecedent and consequent slots dangle at
        // *distinct* sites (D6 — the two-sided row doubles as a site-split pin).
        let mut data = InnerData::default();
        let rule = unsafe { SlotPairingRuleId::new(1) };
        let slot_a = unsafe { SlotId::new(2) };
        let slot_b = unsafe { SlotId::new(3) };
        data.params.slot_pairings.slot_pairing_rule_map.insert(
            rule,
            SlotPairingRule::new(
                SlotRulePart {
                    slot_id: slot_a,
                    should_have: true,
                },
                SlotRulePart {
                    slot_id: slot_b,
                    should_have: false,
                },
                BTreeSet::new(),
                false,
            )
            .expect("distinct slots"),
        );
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([
                FixableInvariant::DanglingFk(Reference::Slot {
                    target: slot_a,
                    site: SlotRefSite::SlotPairingRuleAntecedent(rule),
                }),
                FixableInvariant::DanglingFk(Reference::Slot {
                    target: slot_b,
                    site: SlotRefSite::SlotPairingRuleConsequent(rule),
                }),
            ]))
        );
    }

    #[test]
    fn dangling_group_list_in_colloscope() {
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        let student = unsafe { StudentId::new(2) };
        // Place a *registered* student so only the group list dangles.
        data.params
            .students
            .student_map
            .insert(student, Student::default());
        data.colloscope
            .set_group_list(group_list, BTreeMap::from([(student, 0)]));
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::GroupList {
                    target: group_list,
                    site: GroupListRefSite::ColloscopeGroupListKey,
                }
            )]))
        );
    }

    #[test]
    fn assignments_row_with_both_key_components_dangling() {
        // The `(period, subject)` key contributes two references (a Period edge
        // and a Subject edge); both dangle ⇒ two entries. The placed student is
        // registered, so the row stays canonical (non-empty) and its own
        // reference resolves.
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let subject = unsafe { SubjectId::new(2) };
        let student = unsafe { StudentId::new(3) };
        data.params
            .students
            .student_map
            .insert(student, Student::default());
        data.params
            .assignments
            .map
            .insert((period, subject), BTreeSet::from([student]));
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([
                FixableInvariant::DanglingFk(Reference::Period {
                    target: period,
                    site: PeriodRefSite::AssignmentsKey { subject },
                }),
                FixableInvariant::DanglingFk(Reference::Subject {
                    target: subject,
                    site: SubjectRefSite::AssignmentsKey { period },
                }),
            ]))
        );
    }

    #[test]
    fn interrogation_row_with_both_key_components_dangling() {
        // The `(slot, week)` colloscope key contributes a Slot edge and a Week
        // edge; both dangle ⇒ two entries.
        let mut data = InnerData::default();
        let slot = unsafe { SlotId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.colloscope
            .set_interrogation(slot, week, BTreeSet::from([0]));
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([
                FixableInvariant::DanglingFk(Reference::Week {
                    target: week,
                    site: WeekRefSite::ColloscopeInterrogation { slot },
                }),
                FixableInvariant::DanglingFk(Reference::Slot {
                    target: slot,
                    site: SlotRefSite::ColloscopeInterrogation { week },
                }),
            ]))
        );
    }

    #[test]
    fn one_entry_per_id_occurrence() {
        // Two teachers reference the *same* forged subject: the registry's unit
        // of account is the occurrence, so the two distinct-site references both
        // survive dedup ⇒ two entries with the same target.
        let mut data = InnerData::default();
        let teacher_a = unsafe { TeacherId::new(1) };
        let teacher_b = unsafe { TeacherId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        data.params.teachers.teacher_map.insert(
            teacher_a,
            Teacher {
                subjects: BTreeSet::from([subject]),
                ..Default::default()
            },
        );
        data.params.teachers.teacher_map.insert(
            teacher_b,
            Teacher {
                subjects: BTreeSet::from([subject]),
                ..Default::default()
            },
        );
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([
                FixableInvariant::DanglingFk(Reference::Subject {
                    target: subject,
                    site: SubjectRefSite::TeacherSubjects(teacher_a),
                }),
                FixableInvariant::DanglingFk(Reference::Subject {
                    target: subject,
                    site: SubjectRefSite::TeacherSubjects(teacher_b),
                }),
            ]))
        );
    }

    #[test]
    fn empty_state_has_no_broken_invariants() {
        assert_eq!(
            broken_invariants(&InnerData::default()),
            Ok(BTreeSet::new())
        );
    }

    #[test]
    fn bootstrap_states_have_no_broken_invariants() {
        use collomatique_state::traits::Manager;
        use collomatique_testgen_colloscopes::rand::SeedableRng;
        use collomatique_testgen_colloscopes::{ChaCha8Rng, harness};

        // Every reference in a legitimately-built document resolves. Fixed seeds
        // keep the test deterministic (no time/randomness in test selection).
        for seed in 0..5 {
            let mut rng = ChaCha8Rng::seed_from_u64(seed);
            let (state, _) = harness::bootstrap(&mut rng);
            // The intentional dev-dep cycle (state-colloscopes ⇄ testgen) makes
            // rustc instantiate this crate twice in the lib-test build, so the
            // `InnerData` reachable through `testgen` is a distinct instance from
            // this module's. It resolves the checker as a method (on its own
            // instance) but cannot feed the local `broken_invariants` wrapper —
            // this clean sanity check asserts on the method directly.
            assert_eq!(
                state.get_data().get_inner_data().broken_invariants(),
                Ok(BTreeSet::new()),
                "bootstrap seed {seed} produced broken invariants",
            );
        }
    }

    #[test]
    fn logic_error_declaration_order_is_canonical() {
        // One value per variant, in declaration order; payloads are arbitrary
        // (the derived Ord compares the variant tag first).
        let period = unsafe { PeriodId::new(1) };
        let subject = unsafe { SubjectId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        let week = unsafe { WeekId::new(4) };
        let group_list = unsafe { GroupListId::new(5) };
        let all = [
            LogicError::DuplicatedId(42),
            LogicError::EmptyAssignmentsRow(period, subject),
            LogicError::EmptySlotsRow(subject),
            LogicError::SlotOrderingUnknownId(subject, slot),
            LogicError::SlotOrderingWrongSubject(subject, slot),
            LogicError::SlotOrderingDuplicate(slot),
            LogicError::OrphanSlot(slot),
            LogicError::EmptyWeeksRow(period),
            LogicError::WeekOrderingUnknownId(period, week),
            LogicError::WeekOrderingWrongPeriod(period, week),
            LogicError::WeekOrderingDuplicate(week),
            LogicError::OrphanWeek(week),
            LogicError::EmptyInterrogationRow(slot, week),
            LogicError::EmptyColloscopeGroupListRow(group_list),
        ];
        // Strict `<`, not is_sorted: equal adjacent values would be a
        // duplicated-variant bug.
        assert!(all.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn convergence_declaration_order_is_canonical() {
        let period = unsafe { PeriodId::new(1) };
        let subject = unsafe { SubjectId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        let week = unsafe { WeekId::new(4) };
        let group_list = unsafe { GroupListId::new(5) };
        let teacher = unsafe { TeacherId::new(6) };
        let student = unsafe { StudentId::new(7) };
        let slot_pairing_rule = unsafe { SlotPairingRuleId::new(8) };
        let group = 9u32;
        let start = SlotStart {
            weekday: chrono::Weekday::Mon.into(),
            start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(23, 0, 0).unwrap())
                .unwrap(),
        };
        let duration = collomatique_time::NonZeroMinutes::new(90).unwrap();
        let all = [
            Convergence::SlotTeacherDoesNotTeachSubject(slot, teacher, subject),
            Convergence::TeacherSubjectWithoutInterrogations(teacher, subject),
            Convergence::SlotForSubjectWithoutInterrogations(slot, subject),
            Convergence::SlotOverflowsDay {
                slot,
                start,
                duration,
            },
            Convergence::AssignmentForSubjectNotRunningOnPeriod(period, subject),
            Convergence::AssignedStudentNotPresentForPeriod {
                period,
                subject,
                student,
            },
            Convergence::InterrogationSlotNotRunningOnPeriod(slot, week),
            Convergence::InterrogationOnInactiveWeek(slot, week),
            Convergence::InterrogationGroupOutOfBounds(slot, week, group),
            Convergence::AssociationForSubjectWithoutInterrogations(period, subject),
            Convergence::AssociationForSubjectNotRunningOnPeriod(period, subject),
            Convergence::BalancingForSubjectWithoutInterrogations(subject),
            Convergence::PairedSlotsNotInSameSubject(slot_pairing_rule, slot, slot),
            Convergence::ColloscopeGroupListPrefilled(group_list),
            Convergence::ColloscopeStudentExcluded(group_list, student),
            Convergence::ColloscopeStudentGroupOutOfBounds(group_list, student, group),
        ];
        assert!(all.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn dangling_fk_sorts_before_convergence() {
        // The *largest* possible DanglingFk (last Reference kind, last site
        // variant, big id) still sorts before the *smallest* Convergence
        // (first variant, id 0): the variant tag dominates every payload.
        let biggest_dangling = FixableInvariant::DanglingFk(Reference::GroupList {
            target: unsafe { GroupListId::new(999) },
            site: GroupListRefSite::ColloscopeGroupListKey,
        });
        let smallest_convergence =
            FixableInvariant::Convergence(Convergence::SlotTeacherDoesNotTeachSubject(
                unsafe { SlotId::new(0) },
                unsafe { TeacherId::new(0) },
                unsafe { SubjectId::new(0) },
            ));
        assert!(biggest_dangling < smallest_convergence);
    }

    #[test]
    fn btreeset_first_picks_the_precise_fix() {
        // The §2.3 rationale: a row both dangling and convergence-broken —
        // min() must surface the row-removal fix.
        let dangling = FixableInvariant::DanglingFk(Reference::GroupList {
            target: unsafe { GroupListId::new(1) },
            site: GroupListRefSite::ColloscopeGroupListKey,
        });
        let convergence =
            FixableInvariant::Convergence(Convergence::ColloscopeGroupListPrefilled(unsafe {
                GroupListId::new(1)
            }));
        let mut set = BTreeSet::new();
        set.insert(convergence);
        set.insert(dangling);
        assert!(matches!(set.first(), Some(FixableInvariant::DanglingFk(_))));
    }

    // ---- Layer A: logic errors (the `Err` path) ----
    //
    // Each test forges *exactly* one broken row (or, for the collection tests,
    // a controlled few) and asserts exact set equality on the whole `Err(...)`.
    // Corruption reaches otherwise-unreachable states through pub map fields,
    // forged ids (`unsafe { Id::new(n) }`), and the `#[cfg(test)]` `forge_*`
    // hatches on `Slots` / `Colloscope` (the three empty-row variants have no
    // production surface — the canonicalizing setters drop empty writes).

    #[test]
    fn duplicated_id_is_reported() {
        // The same raw id used by a student and a teacher: two distinct entities
        // collide in the shared u64 namespace. Empty entities create no refs, so
        // the id collision is the only fault.
        let mut data = InnerData::default();
        data.params
            .students
            .student_map
            .insert(unsafe { StudentId::new(1) }, Student::default());
        data.params
            .teachers
            .teacher_map
            .insert(unsafe { TeacherId::new(1) }, Teacher::default());
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::DuplicatedId(1)]))
        );
    }

    #[test]
    fn duplicated_id_reported_once_per_raw_id() {
        // The same raw id shared by three entities still yields a single entry:
        // the `BTreeSet` dedups on the raw id, not the occurrence.
        let mut data = InnerData::default();
        data.params
            .students
            .student_map
            .insert(unsafe { StudentId::new(1) }, Student::default());
        data.params
            .teachers
            .teacher_map
            .insert(unsafe { TeacherId::new(1) }, Teacher::default());
        data.params.week_patterns.week_pattern_map.insert(
            unsafe { WeekPatternId::new(1) },
            WeekPattern {
                name: "P".into(),
                excluded_weeks: BTreeSet::new(),
            },
        );
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::DuplicatedId(1)]))
        );
    }

    #[test]
    fn empty_assignments_row() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let subject = unsafe { SubjectId::new(2) };
        data.params
            .assignments
            .map
            .insert((period, subject), BTreeSet::new());
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::EmptyAssignmentsRow(
                period, subject
            )]))
        );
    }

    #[test]
    fn empty_slots_row() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        data.params.slots.forge_ordering_row(subject, vec![]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::EmptySlotsRow(subject)]))
        );
    }

    #[test]
    fn empty_weeks_row() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        data.params.weeks.forge_ordering_row(period, vec![]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::EmptyWeeksRow(period)]))
        );
    }

    // ---- Ordering↔table mirror (slots). Each fixture plants a real slot via
    // the `insert_slot_at` compound mutator (keeps both containers in lockstep),
    // then re-forges the ordering row into the one desync under test. `find_slot`
    // failures and dangling teacher/subject refs never surface: the logic-error
    // sweep short-circuits `broken_invariants` before layer B runs.

    #[test]
    fn slot_ordering_unknown_id() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        let fake = unsafe { SlotId::new(4) };
        data.params
            .slots
            .insert_slot_at(slot, test_slot(subject, teacher), 0);
        // Re-forge the row to name a slot id that was never issued.
        data.params
            .slots
            .forge_ordering_row(subject, vec![slot, fake]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::SlotOrderingUnknownId(
                subject, fake
            )]))
        );
    }

    #[test]
    fn slot_ordering_wrong_subject() {
        let mut data = InnerData::default();
        let subject_a = unsafe { SubjectId::new(1) };
        let subject_b = unsafe { SubjectId::new(2) };
        let teacher = unsafe { TeacherId::new(3) };
        let s1 = unsafe { SlotId::new(4) };
        let s2 = unsafe { SlotId::new(5) };
        // Both slots name subject A; file s2 under subject B's row.
        data.params
            .slots
            .insert_slot_at(s1, test_slot(subject_a, teacher), 0);
        data.params
            .slots
            .insert_slot_at(s2, test_slot(subject_a, teacher), 1);
        data.params.slots.forge_ordering_row(subject_a, vec![s1]);
        data.params.slots.forge_ordering_row(subject_b, vec![s2]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::SlotOrderingWrongSubject(
                subject_b, s2
            )]))
        );
    }

    #[test]
    fn slot_ordering_duplicate() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        data.params
            .slots
            .insert_slot_at(slot, test_slot(subject, teacher), 0);
        data.params
            .slots
            .forge_ordering_row(subject, vec![slot, slot]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::SlotOrderingDuplicate(slot)]))
        );
    }

    #[test]
    fn orphan_slot() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        let s1 = unsafe { SlotId::new(3) };
        let s2 = unsafe { SlotId::new(4) };
        data.params
            .slots
            .insert_slot_at(s1, test_slot(subject, teacher), 0);
        data.params
            .slots
            .insert_slot_at(s2, test_slot(subject, teacher), 1);
        // Drop s2 from the ordering: it is left in the slot table, un-ordered.
        data.params.slots.forge_ordering_row(subject, vec![s1]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::OrphanSlot(s2)]))
        );
    }

    // ---- Ordering↔table mirror (weeks). The exact twin of the slots block:
    // `insert_week_at` plants a real week, `forge_ordering_row` splits the two
    // containers into the one desync under test.

    #[test]
    fn week_ordering_unknown_id() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let fake = unsafe { WeekId::new(3) };
        data.params
            .weeks
            .insert_week_at(week, period, 0, WeekDesc::default());
        data.params
            .weeks
            .forge_ordering_row(period, vec![week, fake]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::WeekOrderingUnknownId(
                period, fake
            )]))
        );
    }

    #[test]
    fn week_ordering_wrong_period() {
        let mut data = InnerData::default();
        let period_a = unsafe { PeriodId::new(1) };
        let period_b = unsafe { PeriodId::new(2) };
        let w1 = unsafe { WeekId::new(3) };
        let w2 = unsafe { WeekId::new(4) };
        // Both weeks name period A; file w2 under period B's row.
        data.params
            .weeks
            .insert_week_at(w1, period_a, 0, WeekDesc::default());
        data.params
            .weeks
            .insert_week_at(w2, period_a, 1, WeekDesc::default());
        data.params.weeks.forge_ordering_row(period_a, vec![w1]);
        data.params.weeks.forge_ordering_row(period_b, vec![w2]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::WeekOrderingWrongPeriod(
                period_b, w2
            )]))
        );
    }

    #[test]
    fn week_ordering_duplicate() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.params
            .weeks
            .insert_week_at(week, period, 0, WeekDesc::default());
        data.params
            .weeks
            .forge_ordering_row(period, vec![week, week]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::WeekOrderingDuplicate(week)]))
        );
    }

    #[test]
    fn orphan_week() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let w1 = unsafe { WeekId::new(2) };
        let w2 = unsafe { WeekId::new(3) };
        data.params
            .weeks
            .insert_week_at(w1, period, 0, WeekDesc::default());
        data.params
            .weeks
            .insert_week_at(w2, period, 1, WeekDesc::default());
        // Drop w2 from the ordering: it is left in the week table, un-ordered.
        data.params.weeks.forge_ordering_row(period, vec![w1]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::OrphanWeek(w2)]))
        );
    }

    #[test]
    fn empty_interrogation_row() {
        let mut data = InnerData::default();
        let slot = unsafe { SlotId::new(1) };
        let week = unsafe { WeekId::new(2) };
        data.colloscope
            .forge_interrogation_row(slot, week, BTreeSet::new());
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::EmptyInterrogationRow(
                slot, week
            )]))
        );
    }

    #[test]
    fn empty_colloscope_group_list_row() {
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        data.colloscope
            .forge_group_list_row(group_list, BTreeMap::new());
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::EmptyColloscopeGroupListRow(
                group_list
            )]))
        );
    }

    #[test]
    fn multiple_logic_errors_all_reported() {
        // A duplicate id, an empty assignments row, and an empty slots ordering
        // row in one state: all three surface together. (The degenerate pairing
        // rule that used to be the third leg is now unrepresentable after the
        // `PairingRule` seal, so an `EmptySlotsRow` — forged through the
        // test-only ordering hatch — stands in.)
        let mut data = InnerData::default();
        data.params
            .students
            .student_map
            .insert(unsafe { StudentId::new(1) }, Student::default());
        data.params
            .teachers
            .teacher_map
            .insert(unsafe { TeacherId::new(1) }, Teacher::default());
        let period = unsafe { PeriodId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        data.params
            .assignments
            .map
            .insert((period, subject), BTreeSet::new());
        let empty_row_subject = unsafe { SubjectId::new(5) };
        data.params
            .slots
            .forge_ordering_row(empty_row_subject, vec![]);
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([
                LogicError::DuplicatedId(1),
                LogicError::EmptyAssignmentsRow(period, subject),
                LogicError::EmptySlotsRow(empty_row_subject),
            ]))
        );
    }

    #[test]
    fn logic_error_short_circuits_dangling_sweep() {
        // The stage-3 dangling fixture: a student excluding a non-existent
        // period. On its own it is a fixable dangling reference.
        let mut data = InnerData::default();
        let student = unsafe { StudentId::new(1) };
        let period = unsafe { PeriodId::new(2) };
        data.params.students.student_map.insert(
            student,
            Student {
                excluded_periods: BTreeSet::from([period]),
                ..Default::default()
            },
        );
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::Period {
                    target: period,
                    site: PeriodRefSite::StudentExcludedPeriods(student),
                }
            )]))
        );

        // Add a logic error to the *same* state: the verdict flips wholesale to
        // `Err`, and the dangling reference no longer appears — the fixable sweep
        // never runs.
        let empty_period = unsafe { PeriodId::new(3) };
        let empty_subject = unsafe { SubjectId::new(4) };
        data.params
            .assignments
            .map
            .insert((empty_period, empty_subject), BTreeSet::new());
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::EmptyAssignmentsRow(
                empty_period,
                empty_subject
            )]))
        );
    }

    // ---- Layer C: convergence ----
    //
    // Each test forges a state that trips exactly the intended convergence
    // predicate(s) and asserts exact set equality on the whole `Ok(...)`. Where a
    // second entry is structurally unavoidable (any slot on an off subject also
    // implicates its teacher entry; the colloscope fixture's association trips
    // when its subject stops running), the two-element set is asserted with a
    // comment. The discipline pins (21–26) corrupt a reference *and* the
    // predicate behind it, showing layers B and C coexist and that a dangling
    // data-reading lookup skips the predicate.

    /// One period holding one week, built through the public constructors.
    fn test_periods(period: PeriodId, week: WeekId, desc: WeekDesc) -> (Periods, Weeks) {
        let periods = Periods::from_ordered_ids(None, vec![period]).unwrap();
        let weeks = Weeks::from_period_rows(vec![(period, vec![(week, desc)])]).unwrap();
        (periods, weeks)
    }

    /// A subject with interrogations disabled (the default has them enabled).
    fn subject_without_interrogations() -> Subject {
        Subject {
            parameters: SubjectParameters {
                interrogation_parameters: None,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    /// An automatic (non-prefilled) group list with `n` unnamed groups and no
    /// excluded students.
    fn automatic_group_list(n: usize) -> GroupList {
        GroupList::new(
            GroupListParameters {
                group_names: vec![None; n],
                ..Default::default()
            },
            GroupListFilling::Automatic {
                excluded_students: BTreeSet::new(),
            },
        )
        .expect("automatic filling is always consistent")
    }

    /// A copy of [test_slot] with the start time set to `h:m`.
    fn slot_at(subject_id: SubjectId, teacher_id: TeacherId, h: u32, m: u32) -> Slot {
        let mut slot = test_slot(subject_id, teacher_id);
        slot.start_time.start_time =
            WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(h, m, 0).unwrap()).unwrap();
        slot
    }

    /// Ids of a fully valid one-of-everything colloscope state. Only the ids the
    /// twist tests read are exposed; the group list and student are built into
    /// `data` but never referenced by id.
    struct ColloscopeFixture {
        data: InnerData,
        period: PeriodId,
        week: WeekId,
        subject: SubjectId,
        teacher: TeacherId,
        slot: SlotId,
    }

    /// A clean state with one period+week (active), one subject at position 0,
    /// a teacher teaching it, one slot, an automatic 2-group list and the
    /// `(period, subject)` association, plus one student. It holds no colloscope
    /// rows: tests add rows / twist one aspect through the returned ids. All raw
    /// ids are distinct so the duplicate-id logic-error check stays clean.
    fn colloscope_fixture() -> ColloscopeFixture {
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        let teacher = unsafe { TeacherId::new(4) };
        let slot = unsafe { SlotId::new(5) };
        let group_list = unsafe { GroupListId::new(6) };
        let student = unsafe { StudentId::new(7) };

        let mut data = InnerData::default();
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
        data.params.teachers.teacher_map.insert(
            teacher,
            Teacher {
                subjects: BTreeSet::from([subject]),
                ..Default::default()
            },
        );
        data.params.slots =
            Slots::from_subject_rows([(subject, vec![(slot, test_slot(subject, teacher))])])
                .unwrap();
        data.params
            .group_lists
            .group_list_map
            .insert(group_list, automatic_group_list(2));
        data.params
            .group_lists
            .subjects_associations
            .insert((period, subject), group_list);
        data.params
            .students
            .student_map
            .insert(student, Student::default());

        ColloscopeFixture {
            data,
            period,
            week,
            subject,
            teacher,
            slot,
        }
    }

    #[test]
    fn slot_teacher_does_not_teach_subject() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
        // Teacher registered but teaching nothing.
        data.params
            .teachers
            .teacher_map
            .insert(teacher, Teacher::default());
        data.params.slots =
            Slots::from_subject_rows([(subject, vec![(slot, test_slot(subject, teacher))])])
                .unwrap();
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::SlotTeacherDoesNotTeachSubject(slot, teacher, subject)
            )]))
        );
    }

    #[test]
    fn teacher_subject_without_interrogations() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, subject_without_interrogations())
            .unwrap();
        data.params.teachers.teacher_map.insert(
            teacher,
            Teacher {
                subjects: BTreeSet::from([subject]),
                ..Default::default()
            },
        );
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::TeacherSubjectWithoutInterrogations(teacher, subject)
            )]))
        );
    }

    #[test]
    fn slot_for_subject_without_interrogations() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, subject_without_interrogations())
            .unwrap();
        data.params.teachers.teacher_map.insert(
            teacher,
            Teacher {
                subjects: BTreeSet::from([subject]),
                ..Default::default()
            },
        );
        data.params.slots =
            Slots::from_subject_rows([(subject, vec![(slot, test_slot(subject, teacher))])])
                .unwrap();
        // Two entries are unavoidable: a slot on an off subject implicates the
        // teacher entry that teaches it as well as the per-slot check.
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([
                FixableInvariant::Convergence(Convergence::TeacherSubjectWithoutInterrogations(
                    teacher, subject
                )),
                FixableInvariant::Convergence(Convergence::SlotForSubjectWithoutInterrogations(
                    slot, subject
                )),
            ]))
        );
    }

    #[test]
    fn slot_overflows_day() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
        data.params.teachers.teacher_map.insert(
            teacher,
            Teacher {
                subjects: BTreeSet::from([subject]),
                ..Default::default()
            },
        );
        // 23:30 + the default 60-minute interrogation crosses midnight.
        data.params.slots =
            Slots::from_subject_rows([(subject, vec![(slot, slot_at(subject, teacher, 23, 30))])])
                .unwrap();
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::SlotOverflowsDay {
                    slot,
                    start: slot_at(subject, teacher, 23, 30).start_time,
                    duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
                }
            )]))
        );
    }

    #[test]
    fn slot_ending_exactly_at_midnight_is_fine() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
        data.params.teachers.teacher_map.insert(
            teacher,
            Teacher {
                subjects: BTreeSet::from([subject]),
                ..Default::default()
            },
        );
        // 23:00 + 60 minutes ends exactly at midnight (the 86 400-second wrap).
        data.params.slots =
            Slots::from_subject_rows([(subject, vec![(slot, slot_at(subject, teacher, 23, 0))])])
                .unwrap();
        assert_eq!(broken_invariants(&data), Ok(BTreeSet::new()));
    }

    #[test]
    fn assignment_for_subject_not_running_on_period() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        let student = unsafe { StudentId::new(4) };
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(
                0,
                subject,
                Subject {
                    excluded_periods: BTreeSet::from([period]),
                    ..Default::default()
                },
            )
            .unwrap();
        data.params
            .students
            .student_map
            .insert(student, Student::default());
        data.params
            .assignments
            .map
            .insert((period, subject), BTreeSet::from([student]));
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::AssignmentForSubjectNotRunningOnPeriod(period, subject)
            )]))
        );
    }

    #[test]
    fn assigned_student_not_present_for_period() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        let student = unsafe { StudentId::new(4) };
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
        data.params.students.student_map.insert(
            student,
            Student {
                excluded_periods: BTreeSet::from([period]),
                ..Default::default()
            },
        );
        data.params
            .assignments
            .map
            .insert((period, subject), BTreeSet::from([student]));
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::AssignedStudentNotPresentForPeriod {
                    period,
                    subject,
                    student,
                }
            )]))
        );
    }

    #[test]
    fn association_for_subject_without_interrogations() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        let group_list = unsafe { GroupListId::new(4) };
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, subject_without_interrogations())
            .unwrap();
        data.params
            .group_lists
            .group_list_map
            .insert(group_list, automatic_group_list(2));
        data.params
            .group_lists
            .subjects_associations
            .insert((period, subject), group_list);
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::AssociationForSubjectWithoutInterrogations(period, subject)
            )]))
        );
    }

    #[test]
    fn association_for_subject_not_running_on_period() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        let group_list = unsafe { GroupListId::new(4) };
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(
                0,
                subject,
                Subject {
                    excluded_periods: BTreeSet::from([period]),
                    ..Default::default()
                },
            )
            .unwrap();
        data.params
            .group_lists
            .group_list_map
            .insert(group_list, automatic_group_list(2));
        data.params
            .group_lists
            .subjects_associations
            .insert((period, subject), group_list);
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::AssociationForSubjectNotRunningOnPeriod(period, subject)
            )]))
        );
    }

    #[test]
    fn association_row_accumulates_both_breaks() {
        // Off subject that *also* excludes the period: both association
        // predicates fire (the old checker stopped at the first).
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        let group_list = unsafe { GroupListId::new(4) };
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(
                0,
                subject,
                Subject {
                    parameters: SubjectParameters {
                        interrogation_parameters: None,
                        ..Default::default()
                    },
                    excluded_periods: BTreeSet::from([period]),
                },
            )
            .unwrap();
        data.params
            .group_lists
            .group_list_map
            .insert(group_list, automatic_group_list(2));
        data.params
            .group_lists
            .subjects_associations
            .insert((period, subject), group_list);
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([
                FixableInvariant::Convergence(
                    Convergence::AssociationForSubjectWithoutInterrogations(period, subject)
                ),
                FixableInvariant::Convergence(
                    Convergence::AssociationForSubjectNotRunningOnPeriod(period, subject)
                ),
            ]))
        );
    }

    #[test]
    fn balancing_for_subject_without_interrogations() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, subject_without_interrogations())
            .unwrap();
        data.params
            .balancing
            .subjects
            .insert(subject, BalancingOptions::default());
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::BalancingForSubjectWithoutInterrogations(subject)
            )]))
        );
    }

    #[test]
    fn paired_slots_not_in_same_subject() {
        let mut data = InnerData::default();
        let subject_a = unsafe { SubjectId::new(1) };
        let subject_b = unsafe { SubjectId::new(2) };
        let teacher = unsafe { TeacherId::new(3) };
        let slot_a = unsafe { SlotId::new(4) };
        let slot_b = unsafe { SlotId::new(5) };
        let rule = unsafe { SlotPairingRuleId::new(6) };
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject_a, Subject::default())
            .unwrap();
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(1, subject_b, Subject::default())
            .unwrap();
        data.params.teachers.teacher_map.insert(
            teacher,
            Teacher {
                subjects: BTreeSet::from([subject_a, subject_b]),
                ..Default::default()
            },
        );
        data.params.slots = Slots::from_subject_rows([
            (subject_a, vec![(slot_a, test_slot(subject_a, teacher))]),
            (subject_b, vec![(slot_b, test_slot(subject_b, teacher))]),
        ])
        .unwrap();
        data.params.slot_pairings.slot_pairing_rule_map.insert(
            rule,
            SlotPairingRule::new(
                SlotRulePart {
                    slot_id: slot_a,
                    should_have: true,
                },
                SlotRulePart {
                    slot_id: slot_b,
                    should_have: false,
                },
                BTreeSet::new(),
                false,
            )
            .expect("distinct slots"),
        );
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::PairedSlotsNotInSameSubject(rule, slot_a, slot_b)
            )]))
        );
    }

    #[test]
    fn interrogation_slot_not_running_on_period() {
        let mut fx = colloscope_fixture();
        fx.data
            .params
            .subjects
            .ordered_subject_list
            .get_mut(&fx.subject)
            .unwrap()
            .excluded_periods
            .insert(fx.period);
        fx.data
            .colloscope
            .set_interrogation(fx.slot, fx.week, BTreeSet::from([0]));
        // Excluding the period breaks both the interrogation row and the
        // fixture's own association row.
        assert_eq!(
            broken_invariants(&fx.data),
            Ok(BTreeSet::from([
                FixableInvariant::Convergence(
                    Convergence::AssociationForSubjectNotRunningOnPeriod(fx.period, fx.subject)
                ),
                FixableInvariant::Convergence(Convergence::InterrogationSlotNotRunningOnPeriod(
                    fx.slot, fx.week
                )),
            ]))
        );
    }

    #[test]
    fn interrogation_on_inactive_week() {
        let mut fx = colloscope_fixture();
        (fx.data.params.periods, fx.data.params.weeks) =
            test_periods(fx.period, fx.week, WeekDesc::new(false));
        fx.data
            .colloscope
            .set_interrogation(fx.slot, fx.week, BTreeSet::from([0]));
        assert_eq!(
            broken_invariants(&fx.data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::InterrogationOnInactiveWeek(fx.slot, fx.week)
            )]))
        );
    }

    #[test]
    fn interrogation_on_pattern_excluded_week() {
        let mut fx = colloscope_fixture();
        let pattern = unsafe { WeekPatternId::new(8) };
        fx.data.params.week_patterns.week_pattern_map.insert(
            pattern,
            WeekPattern {
                name: "P".into(),
                excluded_weeks: BTreeSet::from([fx.week]),
            },
        );
        let mut slot_desc = test_slot(fx.subject, fx.teacher);
        slot_desc.week_pattern = Some(pattern);
        fx.data.params.slots =
            Slots::from_subject_rows([(fx.subject, vec![(fx.slot, slot_desc)])]).unwrap();
        fx.data
            .colloscope
            .set_interrogation(fx.slot, fx.week, BTreeSet::from([0]));
        assert_eq!(
            broken_invariants(&fx.data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::InterrogationOnInactiveWeek(fx.slot, fx.week)
            )]))
        );
    }

    #[test]
    fn interrogation_group_out_of_bounds() {
        // Group list has 2 groups: group 2 is out of range.
        let mut fx = colloscope_fixture();
        fx.data
            .colloscope
            .set_interrogation(fx.slot, fx.week, BTreeSet::from([2]));
        assert_eq!(
            broken_invariants(&fx.data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::InterrogationGroupOutOfBounds(fx.slot, fx.week, 2)
            )]))
        );

        // Groups 0 and 1 are in range.
        let mut fx = colloscope_fixture();
        fx.data
            .colloscope
            .set_interrogation(fx.slot, fx.week, BTreeSet::from([0, 1]));
        assert_eq!(broken_invariants(&fx.data), Ok(BTreeSet::new()));
    }

    #[test]
    fn missing_association_means_bound_zero() {
        // With no association the bound is 0, so even group 0 is out of range
        // (replicating the old `.unwrap_or(0)`).
        let mut fx = colloscope_fixture();
        fx.data
            .params
            .group_lists
            .subjects_associations
            .remove(&(fx.period, fx.subject));
        fx.data
            .colloscope
            .set_interrogation(fx.slot, fx.week, BTreeSet::from([0]));
        assert_eq!(
            broken_invariants(&fx.data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::InterrogationGroupOutOfBounds(fx.slot, fx.week, 0)
            )]))
        );
    }

    #[test]
    fn colloscope_group_list_prefilled() {
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        let student = unsafe { StudentId::new(2) };
        // A consistent prefilled list (one group, matching group name, no
        // duplicated student) with a colloscope placement row.
        data.params.group_lists.group_list_map.insert(
            group_list,
            GroupList::new(
                GroupListParameters {
                    group_names: vec![None],
                    ..Default::default()
                },
                GroupListFilling::Prefilled {
                    groups: vec![PrefilledGroup::default()],
                },
            )
            .expect("consistent prefilled list"),
        );
        data.params
            .students
            .student_map
            .insert(student, Student::default());
        data.colloscope
            .set_group_list(group_list, BTreeMap::from([(student, 0)]));
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::ColloscopeGroupListPrefilled(group_list)
            )]))
        );
    }

    #[test]
    fn colloscope_student_excluded() {
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        let student = unsafe { StudentId::new(2) };
        data.params.group_lists.group_list_map.insert(
            group_list,
            GroupList::new(
                GroupListParameters {
                    group_names: vec![None, None],
                    ..Default::default()
                },
                GroupListFilling::Automatic {
                    excluded_students: BTreeSet::from([student]),
                },
            )
            .expect("automatic filling is always consistent"),
        );
        data.params
            .students
            .student_map
            .insert(student, Student::default());
        data.colloscope
            .set_group_list(group_list, BTreeMap::from([(student, 0)]));
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::ColloscopeStudentExcluded(group_list, student)
            )]))
        );
    }

    #[test]
    fn colloscope_student_group_out_of_bounds() {
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        let student = unsafe { StudentId::new(2) };
        data.params
            .group_lists
            .group_list_map
            .insert(group_list, automatic_group_list(2));
        data.params
            .students
            .student_map
            .insert(student, Student::default());
        // 2 groups: group 5 is out of range.
        data.colloscope
            .set_group_list(group_list, BTreeMap::from([(student, 5)]));
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([FixableInvariant::Convergence(
                Convergence::ColloscopeStudentGroupOutOfBounds(group_list, student, 5)
            )]))
        );
    }

    #[test]
    fn interrogation_checks_skip_when_slot_dangles() {
        // Inactive week + a row on a *forged* slot id: the inactive-week and
        // bounds checks all need the slot to resolve, so they skip — only the
        // dangling slot surfaces.
        let mut fx = colloscope_fixture();
        (fx.data.params.periods, fx.data.params.weeks) =
            test_periods(fx.period, fx.week, WeekDesc::new(false));
        let forged_slot = unsafe { SlotId::new(99) };
        fx.data
            .colloscope
            .set_interrogation(forged_slot, fx.week, BTreeSet::from([0]));
        assert_eq!(
            broken_invariants(&fx.data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::Slot {
                    target: forged_slot,
                    site: SlotRefSite::ColloscopeInterrogation { week: fx.week },
                }
            )]))
        );
    }

    #[test]
    fn group_bounds_skip_when_association_group_list_dangles() {
        // The association points at a forged group list: the bound is
        // unresolvable, so the group-number check skips despite group 5 — only
        // the dangling group list surfaces.
        let mut fx = colloscope_fixture();
        let forged_gl = unsafe { GroupListId::new(99) };
        fx.data
            .params
            .group_lists
            .subjects_associations
            .insert((fx.period, fx.subject), forged_gl);
        fx.data
            .colloscope
            .set_interrogation(fx.slot, fx.week, BTreeSet::from([5]));
        assert_eq!(
            broken_invariants(&fx.data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::GroupList {
                    target: forged_gl,
                    site: GroupListRefSite::AssociationEntry {
                        period: fx.period,
                        subject: fx.subject,
                    },
                }
            )]))
        );
    }

    #[test]
    fn dangling_week_pattern_is_not_an_inactive_week() {
        // A dangling week pattern counts as "no exclusion", so the active week
        // stays active and no inactive-week break fires — only the dangling
        // pattern surfaces.
        let mut fx = colloscope_fixture();
        let forged_pattern = unsafe { WeekPatternId::new(99) };
        let mut slot_desc = test_slot(fx.subject, fx.teacher);
        slot_desc.week_pattern = Some(forged_pattern);
        fx.data.params.slots =
            Slots::from_subject_rows([(fx.subject, vec![(fx.slot, slot_desc)])]).unwrap();
        fx.data
            .colloscope
            .set_interrogation(fx.slot, fx.week, BTreeSet::from([0]));
        assert_eq!(
            broken_invariants(&fx.data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(
                Reference::WeekPattern {
                    target: forged_pattern,
                    site: WeekPatternRefSite::SlotWeekPattern(fx.slot),
                }
            )]))
        );
    }

    #[test]
    fn student_check_runs_when_assignment_subject_dangles() {
        // The assignments key's subject dangles (layer B), but the per-student
        // present-for-period check gates only on the *student*, which resolves —
        // so both layers report on the same row.
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let forged_subject = unsafe { SubjectId::new(3) };
        let student = unsafe { StudentId::new(4) };
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        data.params.students.student_map.insert(
            student,
            Student {
                excluded_periods: BTreeSet::from([period]),
                ..Default::default()
            },
        );
        data.params
            .assignments
            .map
            .insert((period, forged_subject), BTreeSet::from([student]));
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([
                FixableInvariant::DanglingFk(Reference::Subject {
                    target: forged_subject,
                    site: SubjectRefSite::AssignmentsKey { period },
                }),
                FixableInvariant::Convergence(Convergence::AssignedStudentNotPresentForPeriod {
                    period,
                    subject: forged_subject,
                    student,
                }),
            ]))
        );
    }

    #[test]
    fn slot_teacher_check_runs_when_subject_dangles() {
        // The slot's subject dangles (layer B), but the teacher-teaches check
        // reads only the *teacher* and compares the subject id — so it still
        // fires. Both layers report on the same slot.
        let mut data = InnerData::default();
        let forged_subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        data.params
            .teachers
            .teacher_map
            .insert(teacher, Teacher::default());
        data.params.slots = Slots::from_subject_rows([(
            forged_subject,
            vec![(slot, test_slot(forged_subject, teacher))],
        )])
        .unwrap();
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([
                FixableInvariant::DanglingFk(Reference::Subject {
                    target: forged_subject,
                    site: SubjectRefSite::SlotSubject(slot),
                }),
                FixableInvariant::Convergence(Convergence::SlotTeacherDoesNotTeachSubject(
                    slot,
                    teacher,
                    forged_subject,
                )),
            ]))
        );
    }

    #[test]
    fn logic_error_short_circuits_convergence() {
        // A convergence break (teacher not teaching its slot) plus a forged empty
        // assignments row: the logic error flips the verdict to `Err` and the
        // convergence sweep never runs.
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
        data.params
            .teachers
            .teacher_map
            .insert(teacher, Teacher::default());
        data.params.slots =
            Slots::from_subject_rows([(subject, vec![(slot, test_slot(subject, teacher))])])
                .unwrap();
        let empty_period = unsafe { PeriodId::new(4) };
        let empty_subject = unsafe { SubjectId::new(5) };
        data.params
            .assignments
            .map
            .insert((empty_period, empty_subject), BTreeSet::new());
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::EmptyAssignmentsRow(
                empty_period,
                empty_subject
            )]))
        );
    }

    #[test]
    fn colloscope_fixture_is_clean() {
        // Guards every fixture-based test above: the bare fixture plus an
        // in-bounds interrogation row has no broken invariants.
        let mut fx = colloscope_fixture();
        fx.data
            .colloscope
            .set_interrogation(fx.slot, fx.week, BTreeSet::from([0]));
        assert_eq!(broken_invariants(&fx.data), Ok(BTreeSet::new()));
    }

    // ---- Stage 7: per-site dangling coverage ----
    //
    // One single-corruption fixture per DanglingFk site not already exercised by
    // a stage-3/5 fixture, each pinning the exact new-checker output. (Until
    // step-5 R1.5 these fixtures also pinned the old checker's first error per
    // site — the operational proof of the legacy-bridge tables; that half
    // retired with the old checker.) `Period@WeekPeriodFk` became representable
    // when the force path dropped the `PeriodStillHasWeeks` guard, so it has a
    // fixture (`dangling_period_from_forced_removal_is_reported`).

    /// Asserts that `data` has exactly the one dangling reference `reference`.
    #[track_caller]
    fn assert_single_dangling_fk(data: &InnerData, reference: Reference) {
        assert_eq!(
            broken_invariants(data),
            Ok(BTreeSet::from([FixableInvariant::DanglingFk(reference)])),
        );
    }

    /// Registers a default subject at position 0 (a common scaffold below).
    fn register_subject(data: &mut InnerData, subject: SubjectId) {
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
    }

    #[test]
    fn dangling_period_in_subject_exclusions_is_reported() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let period = unsafe { PeriodId::new(2) };
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(
                0,
                subject,
                Subject {
                    excluded_periods: BTreeSet::from([period]),
                    ..Default::default()
                },
            )
            .unwrap();
        assert_single_dangling_fk(
            &data,
            Reference::Period {
                target: period,
                site: PeriodRefSite::SubjectExcludedPeriods(subject),
            },
        );
    }

    #[test]
    fn dangling_period_from_forced_removal_is_reported() {
        // A period holding one week, then force-removed. `force_apply_period`
        // `RemoveWithWeeks` drops the `PeriodStillHasWeeks` guard, so its only mutation
        // is `ordered_period_list.remove_at(position)`: the `week_map` entry and
        // the ordering row stay, leaving the week's `period_id` FK dangling.
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let mut data = InnerData::default();
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        data.params.periods.ordered_period_list.remove_at(0);
        assert_single_dangling_fk(
            &data,
            Reference::Period {
                target: period,
                site: PeriodRefSite::WeekPeriodFk(week),
            },
        );
    }

    #[test]
    fn dangling_period_in_pairing_rule_is_reported() {
        let mut data = InnerData::default();
        let subject_a = unsafe { SubjectId::new(1) };
        let subject_b = unsafe { SubjectId::new(2) };
        let rule = unsafe { PairingRuleId::new(3) };
        let period = unsafe { PeriodId::new(4) };
        register_subject(&mut data, subject_a);
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(1, subject_b, Subject::default())
            .unwrap();
        data.params.pairings.pairing_rule_map.insert(
            rule,
            PairingRule::new(
                RulePart {
                    subject_id: subject_a,
                    should_have: true,
                },
                RulePart {
                    subject_id: subject_b,
                    should_have: false,
                },
                BTreeSet::from([period]),
                false,
            )
            .expect("distinct subjects"),
        );
        assert_single_dangling_fk(
            &data,
            Reference::Period {
                target: period,
                site: PeriodRefSite::PairingRuleExcludedPeriods(rule),
            },
        );
    }

    #[test]
    fn dangling_period_in_slot_pairing_rule_is_reported() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let teacher = unsafe { TeacherId::new(2) };
        let slot_a = unsafe { SlotId::new(3) };
        let slot_b = unsafe { SlotId::new(4) };
        let rule = unsafe { SlotPairingRuleId::new(5) };
        let period = unsafe { PeriodId::new(6) };
        register_subject(&mut data, subject);
        data.params.teachers.teacher_map.insert(
            teacher,
            Teacher {
                subjects: BTreeSet::from([subject]),
                ..Default::default()
            },
        );
        data.params.slots = Slots::from_subject_rows([(
            subject,
            vec![
                (slot_a, test_slot(subject, teacher)),
                (slot_b, test_slot(subject, teacher)),
            ],
        )])
        .unwrap();
        data.params.slot_pairings.slot_pairing_rule_map.insert(
            rule,
            SlotPairingRule::new(
                SlotRulePart {
                    slot_id: slot_a,
                    should_have: true,
                },
                SlotRulePart {
                    slot_id: slot_b,
                    should_have: false,
                },
                BTreeSet::from([period]),
                false,
            )
            .expect("distinct slots"),
        );
        assert_single_dangling_fk(
            &data,
            Reference::Period {
                target: period,
                site: PeriodRefSite::SlotPairingRuleExcludedPeriods(rule),
            },
        );
    }

    #[test]
    fn dangling_period_in_association_is_reported() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let group_list = unsafe { GroupListId::new(2) };
        let period = unsafe { PeriodId::new(3) };
        register_subject(&mut data, subject);
        data.params
            .group_lists
            .group_list_map
            .insert(group_list, automatic_group_list(2));
        data.params
            .group_lists
            .subjects_associations
            .insert((period, subject), group_list);
        assert_single_dangling_fk(
            &data,
            Reference::Period {
                target: period,
                site: PeriodRefSite::AssociationEntry { subject },
            },
        );
    }

    #[test]
    fn dangling_period_in_assignments_key_is_reported() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let student = unsafe { StudentId::new(2) };
        let period = unsafe { PeriodId::new(3) };
        register_subject(&mut data, subject);
        data.params
            .students
            .student_map
            .insert(student, Student::default());
        data.params
            .assignments
            .map
            .insert((period, subject), BTreeSet::from([student]));
        assert_single_dangling_fk(
            &data,
            Reference::Period {
                target: period,
                site: PeriodRefSite::AssignmentsKey { subject },
            },
        );
    }

    #[test]
    fn dangling_week_in_interrogation_key_is_reported() {
        let mut fx = colloscope_fixture();
        let week = unsafe { WeekId::new(99) };
        fx.data
            .colloscope
            .set_interrogation(fx.slot, week, BTreeSet::from([0]));
        assert_single_dangling_fk(
            &fx.data,
            Reference::Week {
                target: week,
                site: WeekRefSite::ColloscopeInterrogation { slot: fx.slot },
            },
        );
    }

    #[test]
    fn dangling_subject_in_incompat_is_reported() {
        let mut data = InnerData::default();
        let incompat = unsafe { IncompatId::new(1) };
        let subject = unsafe { SubjectId::new(2) };
        data.params.incompats.incompat_map.insert(
            incompat,
            Incompatibility {
                subject_id: subject,
                name: String::new(),
                slots: vec![],
                minimum_free_slots: NonZeroU32::new(1).unwrap(),
                week_pattern_id: None,
            },
        );
        assert_single_dangling_fk(
            &data,
            Reference::Subject {
                target: subject,
                site: SubjectRefSite::IncompatSubject(incompat),
            },
        );
    }

    #[test]
    fn dangling_subject_in_pairing_antecedent_is_reported() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let dangling = unsafe { SubjectId::new(2) };
        let rule = unsafe { PairingRuleId::new(3) };
        register_subject(&mut data, subject);
        data.params.pairings.pairing_rule_map.insert(
            rule,
            PairingRule::new(
                RulePart {
                    subject_id: dangling,
                    should_have: true,
                },
                RulePart {
                    subject_id: subject,
                    should_have: false,
                },
                BTreeSet::new(),
                false,
            )
            .expect("distinct subjects"),
        );
        assert_single_dangling_fk(
            &data,
            Reference::Subject {
                target: dangling,
                site: SubjectRefSite::PairingRuleAntecedent(rule),
            },
        );
    }

    #[test]
    fn dangling_subject_in_pairing_consequent_is_reported() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let dangling = unsafe { SubjectId::new(2) };
        let rule = unsafe { PairingRuleId::new(3) };
        register_subject(&mut data, subject);
        data.params.pairings.pairing_rule_map.insert(
            rule,
            PairingRule::new(
                RulePart {
                    subject_id: subject,
                    should_have: true,
                },
                RulePart {
                    subject_id: dangling,
                    should_have: false,
                },
                BTreeSet::new(),
                false,
            )
            .expect("distinct subjects"),
        );
        assert_single_dangling_fk(
            &data,
            Reference::Subject {
                target: dangling,
                site: SubjectRefSite::PairingRuleConsequent(rule),
            },
        );
    }

    #[test]
    fn dangling_subject_in_balancing_is_reported() {
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        data.params
            .balancing
            .subjects
            .insert(subject, BalancingOptions::default());
        assert_single_dangling_fk(
            &data,
            Reference::Subject {
                target: subject,
                site: SubjectRefSite::BalancingSubjectKey,
            },
        );
    }

    #[test]
    fn dangling_subject_in_association_is_reported() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let group_list = unsafe { GroupListId::new(3) };
        let subject = unsafe { SubjectId::new(4) };
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        data.params
            .group_lists
            .group_list_map
            .insert(group_list, automatic_group_list(2));
        data.params
            .group_lists
            .subjects_associations
            .insert((period, subject), group_list);
        assert_single_dangling_fk(
            &data,
            Reference::Subject {
                target: subject,
                site: SubjectRefSite::AssociationEntry { period },
            },
        );
    }

    #[test]
    fn dangling_group_list_in_association_is_reported() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        let group_list = unsafe { GroupListId::new(4) };
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        register_subject(&mut data, subject);
        data.params
            .group_lists
            .subjects_associations
            .insert((period, subject), group_list);
        assert_single_dangling_fk(
            &data,
            Reference::GroupList {
                target: group_list,
                site: GroupListRefSite::AssociationEntry { period, subject },
            },
        );
    }

    #[test]
    fn dangling_student_in_prefilled_group_is_reported() {
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        let student = unsafe { StudentId::new(2) };
        // One named group and one prefilled group (counts match, no duplicate) so
        // the first old-checker failure is the dangling student, not a prefill
        // logic error.
        data.params.group_lists.group_list_map.insert(
            group_list,
            GroupList::new(
                GroupListParameters {
                    group_names: vec![None],
                    ..Default::default()
                },
                GroupListFilling::Prefilled {
                    groups: vec![PrefilledGroup {
                        students: BTreeSet::from([student]),
                    }],
                },
            )
            .expect("consistent prefilled list (student existence is not checked here)"),
        );
        assert_single_dangling_fk(
            &data,
            Reference::Student {
                target: student,
                site: StudentRefSite::GroupListPrefilledStudent(group_list),
            },
        );
    }

    #[test]
    fn dangling_student_in_excluded_set_is_reported() {
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        let student = unsafe { StudentId::new(2) };
        data.params.group_lists.group_list_map.insert(
            group_list,
            GroupList::new(
                GroupListParameters {
                    group_names: vec![None, None],
                    ..Default::default()
                },
                GroupListFilling::Automatic {
                    excluded_students: BTreeSet::from([student]),
                },
            )
            .expect("automatic filling is always consistent"),
        );
        assert_single_dangling_fk(
            &data,
            Reference::Student {
                target: student,
                site: StudentRefSite::GroupListExcludedStudent(group_list),
            },
        );
    }

    #[test]
    fn dangling_student_in_assignments_cell_is_reported() {
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        let student = unsafe { StudentId::new(4) };
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        register_subject(&mut data, subject);
        data.params
            .assignments
            .map
            .insert((period, subject), BTreeSet::from([student]));
        assert_single_dangling_fk(
            &data,
            Reference::Student {
                target: student,
                site: StudentRefSite::AssignmentsStudent { period, subject },
            },
        );
    }

    #[test]
    fn dangling_student_in_colloscope_group_list_is_reported() {
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        let student = unsafe { StudentId::new(2) };
        data.params
            .group_lists
            .group_list_map
            .insert(group_list, automatic_group_list(2));
        data.colloscope
            .set_group_list(group_list, BTreeMap::from([(student, 0)]));
        assert_single_dangling_fk(
            &data,
            Reference::Student {
                target: student,
                site: StudentRefSite::ColloscopeGroupListStudent(group_list),
            },
        );
    }

    #[test]
    fn dangling_week_pattern_in_incompat_is_reported() {
        let mut data = InnerData::default();
        let incompat = unsafe { IncompatId::new(1) };
        let subject = unsafe { SubjectId::new(2) };
        let pattern = unsafe { WeekPatternId::new(3) };
        register_subject(&mut data, subject);
        data.params.incompats.incompat_map.insert(
            incompat,
            Incompatibility {
                subject_id: subject,
                name: String::new(),
                slots: vec![],
                minimum_free_slots: NonZeroU32::new(1).unwrap(),
                week_pattern_id: Some(pattern),
            },
        );
        assert_single_dangling_fk(
            &data,
            Reference::WeekPattern {
                target: pattern,
                site: WeekPatternRefSite::IncompatWeekPattern(incompat),
            },
        );
    }

    // ---- Stage 7: compound states ----
    //
    // States with more than one corruption, pinning the checker's short-circuit
    // precedence (layer-A logic errors beat fixable breaks) and exact multi-entry
    // sets under multiplicity.

    #[test]
    fn compound_row_both_empty_and_not_running() {
        // One assignments row that is *both* empty (a layer-A logic error) and
        // on a subject that excludes the period (a convergence). The checker
        // short-circuits on the logic error.
        let mut data = InnerData::default();
        let period = unsafe { PeriodId::new(1) };
        let week = unsafe { WeekId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        (data.params.periods, data.params.weeks) = test_periods(period, week, WeekDesc::default());
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(
                0,
                subject,
                Subject {
                    excluded_periods: BTreeSet::from([period]),
                    ..Default::default()
                },
            )
            .unwrap();
        data.params
            .assignments
            .map
            .insert((period, subject), BTreeSet::new());
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::EmptyAssignmentsRow(
                period, subject
            )]))
        );
    }

    #[test]
    fn compound_logic_error_with_unrelated_fixable() {
        // A two-corruption state: an empty assignments row (layer-A logic error)
        // and, unrelated, a dangling subject in a teacher's `subjects`. The
        // checker short-circuits on the logic error and does not report the
        // dangle alongside it.
        let mut data = InnerData::default();
        let teacher = unsafe { TeacherId::new(1) };
        let dangling_subject = unsafe { SubjectId::new(99) };
        let period = unsafe { PeriodId::new(2) };
        let subject = unsafe { SubjectId::new(3) };
        data.params.teachers.teacher_map.insert(
            teacher,
            Teacher {
                subjects: BTreeSet::from([dangling_subject]),
                ..Default::default()
            },
        );
        data.params
            .assignments
            .map
            .insert((period, subject), BTreeSet::new());
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::EmptyAssignmentsRow(
                period, subject
            )]))
        );
    }

    #[test]
    fn compound_duplicate_id_with_dangling_ref() {
        // A raw id shared by a student and a teacher (logic error) *plus* a
        // dangling period in that student's exclusions. Layer A short-circuits
        // on the id collision.
        let mut data = InnerData::default();
        let dangling_period = unsafe { PeriodId::new(50) };
        data.params.students.student_map.insert(
            unsafe { StudentId::new(1) },
            Student {
                excluded_periods: BTreeSet::from([dangling_period]),
                ..Default::default()
            },
        );
        data.params
            .teachers
            .teacher_map
            .insert(unsafe { TeacherId::new(1) }, Teacher::default());
        assert_eq!(
            broken_invariants(&data),
            Err(BTreeSet::from([LogicError::DuplicatedId(1)]))
        );
    }

    #[test]
    fn compound_two_fixable_breaks() {
        // Two independent dangling references — a teacher in a slot and a
        // student in `settings` — so the `Ok` payload has two entries.
        let mut data = InnerData::default();
        let subject = unsafe { SubjectId::new(1) };
        let slot = unsafe { SlotId::new(2) };
        let teacher = unsafe { TeacherId::new(3) };
        let student = unsafe { StudentId::new(4) };
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
        data.params.slots =
            Slots::from_subject_rows([(subject, vec![(slot, test_slot(subject, teacher))])])
                .unwrap();
        data.params
            .settings
            .students
            .insert(student, Limits::default());
        assert_eq!(
            broken_invariants(&data),
            Ok(BTreeSet::from([
                FixableInvariant::DanglingFk(Reference::Teacher {
                    target: teacher,
                    site: TeacherRefSite::SlotTeacher(slot),
                }),
                FixableInvariant::DanglingFk(Reference::Student {
                    target: student,
                    site: StudentRefSite::SettingsStudentKey,
                }),
            ]))
        );
    }

    #[test]
    fn compound_convergence_with_dangling() {
        // A clean fixture twisted into a day-overflowing slot (a convergence)
        // with an added dangling student in `settings`. The checker reports
        // both, as `Ok`.
        let mut fx = colloscope_fixture();
        let dangling_student = unsafe { StudentId::new(99) };
        // 23:30 + the default 60-minute interrogation crosses midnight.
        fx.data.params.slots = Slots::from_subject_rows([(
            fx.subject,
            vec![(fx.slot, slot_at(fx.subject, fx.teacher, 23, 30))],
        )])
        .unwrap();
        fx.data
            .params
            .settings
            .students
            .insert(dangling_student, Limits::default());
        assert_eq!(
            broken_invariants(&fx.data),
            Ok(BTreeSet::from([
                FixableInvariant::DanglingFk(Reference::Student {
                    target: dangling_student,
                    site: StudentRefSite::SettingsStudentKey,
                }),
                FixableInvariant::Convergence(Convergence::SlotOverflowsDay {
                    slot: fx.slot,
                    start: slot_at(fx.subject, fx.teacher, 23, 30).start_time,
                    duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
                }),
            ]))
        );
    }
}
