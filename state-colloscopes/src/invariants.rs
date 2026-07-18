//! Invariant vocabulary for the precise whole-model checker.
//!
//! This module defines *what can be broken*, in three kinds, classified
//! mechanically (plan `docs/plans/plan_step_2.md` §3):
//!
//! - [FixableInvariant::DanglingFk] — the edge is in the refs registry
//!   ([crate::InnerData::for_each_reference]) and its target id does not
//!   resolve. Fixed by removing or clearing the referencing data.
//! - [LogicError] — truth decidable from a row's *own value* (or, for
//!   [LogicError::DuplicatedId], a whole-document id-uniqueness property): no
//!   *other* entity's state can flip it, so no legitimate elementary op can
//!   produce it by side effect. Only buggy code (or a hand-forged file) can.
//!   Not fixable: consumers panic (cascade) or hard-error (decode).
//! - [Convergence] — a predicate over *existing* edges that legitimate ops can
//!   break indirectly (e.g. `UpdateSubject` turning interrogations off breaks
//!   every "subject has interrogations" referrer). The cascade resolves these
//!   lossily (clear the now-invalid data).
//!
//! `FixableInvariant = DanglingFk | Convergence` is the `Ok` payload of the
//! checker; `LogicError` is the `Err` payload and short-circuits (a logic error
//! undermines the meaningfulness of the fixable sweep).
//!
//! ## Canonical order
//!
//! The checker returns `BTreeSet`s; `Ord` is derived on every type here, so
//! **declaration order is the canonical order**. [FixableInvariant::DanglingFk]
//! is declared before [FixableInvariant::Convergence] so that when a row is
//! both dangling and convergence-broken, `min()` picks the precise row-removal
//! fix over the lossy one.
//!
//! The checker itself ([crate::InnerData]`::broken_invariants`) lands in later
//! stages; this module is vocabulary only.

use thiserror::Error;

use crate::ids::{
    GroupListId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId, SubjectId,
    TeacherId, WeekId,
};
use crate::refs::Reference;

/// A state no legitimate elementary op can reach: the *code* (or a hand-forged
/// file) is at fault, not the data. Truth is decidable from the row's own value
/// (or, for [LogicError::DuplicatedId], whole-document id uniqueness) — see the
/// module docs for the classification rule.
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
    /// A stored colloscope interrogation row with an empty group set (rows are
    /// canonical-absent: a row exists iff it holds an assigned group)
    #[error("colloscope interrogation row ({0:?}, {1:?}) is stored with an empty group set")]
    EmptyInterrogationRow(SlotId, WeekId),
    /// A stored colloscope group-list row with an empty placement map (a row
    /// exists iff it holds a placement)
    #[error("colloscope group-list row {0:?} is stored with an empty placement map")]
    EmptyColloscopeGroupListRow(GroupListId),
    /// A prefilled group list whose group count differs from `group_names.len()`
    #[error("prefilled group count does not match the group names of group list {0:?}")]
    PrefillGroupCountMismatch(GroupListId),
    /// A student placed in two prefilled groups of the same group list
    #[error("a student is placed in two prefilled groups of group list {0:?}")]
    DuplicatedStudentInPrefilledGroups(GroupListId),
    /// A pairing rule whose antecedent and consequent name the same subject
    #[error("pairing rule {0:?} has its antecedent and consequent on the same subject")]
    PairingRulePartsShareSubject(PairingRuleId),
    /// A slot pairing rule whose antecedent and consequent name the same slot
    #[error("slot pairing rule {0:?} has its antecedent and consequent on the same slot")]
    SlotPairingRulePartsShareSlot(SlotPairingRuleId),
}

/// A predicate over *existing* edges that legitimate ops can break indirectly —
/// see the module docs for the classification rule. The step-6 cascade resolves
/// these lossily (clear the now-invalid data). Every predicate skips when a
/// prerequisite reference dangles: the [FixableInvariant::DanglingFk] entry
/// already reports that.
///
/// Declaration order is the canonical order (derived `Ord`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Error)]
pub enum Convergence {
    /// The slot's teacher's `subjects` set lacks the slot's subject
    #[error("the teacher of slot {0:?} does not teach the slot's subject")]
    SlotTeacherDoesNotTeachSubject(SlotId),
    /// A teacher references a subject whose interrogations are disabled
    #[error("teacher {0:?} references subject {1:?} which has interrogations disabled")]
    TeacherSubjectWithoutInterrogations(TeacherId, SubjectId),
    /// A slot on a subject whose interrogations are disabled
    #[error("slot {0:?} is on a subject with interrogations disabled")]
    SlotForSubjectWithoutInterrogations(SlotId),
    /// The slot's start time plus its subject's interrogation duration
    /// overflows the day
    #[error("slot {0:?} overflows its day given the subject's interrogation duration")]
    SlotOverflowsDay(SlotId),
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
    #[error("slot pairing rule {0:?} pairs slots of different subjects")]
    PairedSlotsNotInSameSubject(SlotPairingRuleId),
    /// An interrogation whose slot's subject excludes the week's period
    #[error("interrogation ({0:?}, {1:?}): the slot's subject does not run on the week's period")]
    InterrogationSlotNotRunningOnPeriod(SlotId, WeekId),
    /// An interrogation on a week the slot's week pattern deactivates
    #[error("interrogation ({0:?}, {1:?}) is on an inactive week")]
    InterrogationOnInactiveWeek(SlotId, WeekId),
    /// An interrogation assigning a group number ≥ the associated group list's
    /// group count
    #[error("interrogation ({0:?}, {1:?}) assigns an out-of-bounds group number")]
    InterrogationGroupOutOfBounds(SlotId, WeekId),
    /// A colloscope row for a prefilled group list
    #[error("colloscope holds a row for prefilled group list {0:?}")]
    ColloscopeGroupListPrefilled(GroupListId),
    /// A placed student who is in the automatic filling's excluded set
    #[error("colloscope group list {0:?} places excluded student {1:?}")]
    ColloscopeStudentExcluded(GroupListId, StudentId),
    /// A placed student with a group number ≥ the list's group count
    #[error("colloscope group list {0:?} places student {1:?} in an out-of-bounds group")]
    ColloscopeStudentGroupOutOfBounds(GroupListId, StudentId),
}

/// A broken invariant the *data* is responsible for — the `Ok` payload of the
/// checker. Fixed by removing or clearing the referencing data; the step-6
/// cascade's resolution map is total over this type, so every consumer matches
/// both variants exhaustively (no variant is "the panicking one").
///
/// [FixableInvariant::DanglingFk] is declared first so that when a row is both
/// dangling and convergence-broken, `BTreeSet::first()` picks the precise
/// row-removal fix over the lossy one (derived `Ord`, declaration order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Error)]
pub enum FixableInvariant {
    /// A reference whose target id does not resolve
    #[error("dangling reference: {0:?}")]
    DanglingFk(Reference),
    /// A broken convergence predicate
    #[error(transparent)]
    Convergence(Convergence),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Id;
    use crate::refs::{GroupListRefSite, Reference};
    use std::collections::BTreeSet;

    #[test]
    fn logic_error_declaration_order_is_canonical() {
        // One value per variant, in declaration order; payloads are arbitrary
        // (the derived Ord compares the variant tag first).
        let period = unsafe { PeriodId::new(1) };
        let subject = unsafe { SubjectId::new(2) };
        let slot = unsafe { SlotId::new(3) };
        let week = unsafe { WeekId::new(4) };
        let group_list = unsafe { GroupListId::new(5) };
        let pairing_rule = unsafe { PairingRuleId::new(6) };
        let slot_pairing_rule = unsafe { SlotPairingRuleId::new(7) };
        let all = [
            LogicError::DuplicatedId(42),
            LogicError::EmptyAssignmentsRow(period, subject),
            LogicError::EmptySlotsRow(subject),
            LogicError::EmptyInterrogationRow(slot, week),
            LogicError::EmptyColloscopeGroupListRow(group_list),
            LogicError::PrefillGroupCountMismatch(group_list),
            LogicError::DuplicatedStudentInPrefilledGroups(group_list),
            LogicError::PairingRulePartsShareSubject(pairing_rule),
            LogicError::SlotPairingRulePartsShareSlot(slot_pairing_rule),
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
        let all = [
            Convergence::SlotTeacherDoesNotTeachSubject(slot),
            Convergence::TeacherSubjectWithoutInterrogations(teacher, subject),
            Convergence::SlotForSubjectWithoutInterrogations(slot),
            Convergence::SlotOverflowsDay(slot),
            Convergence::AssignmentForSubjectNotRunningOnPeriod(period, subject),
            Convergence::AssignedStudentNotPresentForPeriod {
                period,
                subject,
                student,
            },
            Convergence::AssociationForSubjectWithoutInterrogations(period, subject),
            Convergence::AssociationForSubjectNotRunningOnPeriod(period, subject),
            Convergence::BalancingForSubjectWithoutInterrogations(subject),
            Convergence::PairedSlotsNotInSameSubject(slot_pairing_rule),
            Convergence::InterrogationSlotNotRunningOnPeriod(slot, week),
            Convergence::InterrogationOnInactiveWeek(slot, week),
            Convergence::InterrogationGroupOutOfBounds(slot, week),
            Convergence::ColloscopeGroupListPrefilled(group_list),
            Convergence::ColloscopeStudentExcluded(group_list, student),
            Convergence::ColloscopeStudentGroupOutOfBounds(group_list, student),
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
            FixableInvariant::Convergence(Convergence::SlotTeacherDoesNotTeachSubject(unsafe {
                SlotId::new(0)
            }));
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
}
