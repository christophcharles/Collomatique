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
//! The checker ([crate::InnerData]`::broken_invariants`) lives here too: the
//! logic-error sweep (layer A, the `Err` path) and the dangling-reference sweep
//! (layer B) are implemented; the convergence layer lands in a later stage.

use std::collections::BTreeSet;

use thiserror::Error;

use crate::group_lists::GroupListFilling;
use crate::ids::{
    GroupListId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId, SubjectId,
    TeacherId, WeekId, WeekPatternId,
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

impl crate::InnerData {
    /// Returns every broken invariant of the document, deduplicated, in
    /// canonical order (the derived `Ord`s — see the module docs).
    ///
    /// `Ok` means the *code* is sound: the payload is what the *data* needs
    /// fixed (empty = fully valid). `Err` means a logic error — a state no
    /// legitimate elementary op can reach — and short-circuits: a logic error
    /// undermines the meaningfulness of the fixable sweep.
    ///
    /// Coverage so far: the logic-error sweep (layer A, the `Err` path) and the
    /// dangling-reference sweep (layer B, part of the `Ok` payload). The
    /// convergence layer lands next; until then a clean logic-error sweep gives
    /// an `Ok` holding only dangling references.
    pub fn broken_invariants(&self) -> Result<BTreeSet<FixableInvariant>, BTreeSet<LogicError>> {
        let logic_errors = self.logic_errors();
        if !logic_errors.is_empty() {
            return Err(logic_errors);
        }
        Ok(self.dangling_refs())
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

        // Duplicate raw ids across the shared `u64` namespace. The old check
        // (`InnerData::check_no_duplicate_ids`) is a bool; here each colliding
        // raw id is reported (an id reused three times still yields one entry —
        // the set dedups).
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
        for (subject, order) in self.params.slots.ordering_entries() {
            if order.is_empty() {
                errors.insert(LogicError::EmptySlotsRow(subject));
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

        // Prefilled group lists: the group count matches the names, and no
        // student appears twice. `check_duplicated_student` is vacuously true
        // for `Automatic`, but we only reach it inside the `Prefilled` arm —
        // reusing it keeps the predicate identical to the old checker's.
        for (id, group_list) in self.params.group_lists.group_list_map.iter() {
            if let GroupListFilling::Prefilled { groups } = &group_list.filling {
                if groups.len() != group_list.params.group_names.len() {
                    errors.insert(LogicError::PrefillGroupCountMismatch(id));
                }
                if !group_list.filling.check_duplicated_student() {
                    errors.insert(LogicError::DuplicatedStudentInPrefilledGroups(id));
                }
            }
        }

        // Parts-share-an-id predicates: a rule whose antecedent and consequent
        // name the same subject/slot is degenerate.
        for (id, rule) in self.params.pairings.pairing_rule_map.iter() {
            if rule.antecedent.subject_id == rule.consequent.subject_id {
                errors.insert(LogicError::PairingRulePartsShareSubject(id));
            }
        }
        for (id, rule) in self.params.slot_pairings.slot_pairing_rule_map.iter() {
            if rule.antecedent.slot_id == rule.consequent.slot_id {
                errors.insert(LogicError::SlotPairingRulePartsShareSlot(id));
            }
        }

        errors
    }

    /// Layer B: every registry edge ([Self::for_each_reference]) whose target
    /// id does not resolve, as [FixableInvariant::DanglingFk] entries.
    ///
    /// The eight existence sets are read from the entities' own tables (not the
    /// ordering sidecars), so the sweep stays sound on potentially inconsistent
    /// data. `Week@WeekPeriodFk` is type-guaranteed by the `Periods`
    /// encapsulation (weeks are keyed by their owning period): the arm is
    /// handled generically but never fires.
    fn dangling_refs(&self) -> BTreeSet<FixableInvariant> {
        let periods: BTreeSet<PeriodId> = self.params.periods.period_ids().collect();
        let weeks: BTreeSet<WeekId> = self.params.periods.week_ids().collect();
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InnerData;
    use crate::group_lists::{GroupList, GroupListFilling, GroupListParameters, PrefilledGroup};
    use crate::ids::Id;
    use crate::pairings::{PairingRule, RulePart};
    use crate::refs::{
        GroupListRefSite, PeriodRefSite, Reference, SlotRefSite, StudentRefSite, SubjectRefSite,
        TeacherRefSite, WeekPatternRefSite, WeekRefSite,
    };
    use crate::settings::Limits;
    use crate::slot_pairings::{SlotPairingRule, SlotRulePart};
    use crate::slots::{Slot, Slots};
    use crate::students::Student;
    use crate::subjects::Subject;
    use crate::teachers::Teacher;
    use crate::week_patterns::WeekPattern;
    use collomatique_time::{SlotStart, WholeMinuteTime};
    use std::collections::{BTreeMap, BTreeSet};

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
            data.broken_invariants(),
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
            data.broken_invariants(),
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
            data.broken_invariants(),
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
            data.broken_invariants(),
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
            data.broken_invariants(),
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
        // Register subject and teacher so only the week pattern dangles.
        data.params
            .subjects
            .ordered_subject_list
            .insert_at(0, subject, Subject::default())
            .unwrap();
        data.params
            .teachers
            .teacher_map
            .insert(teacher, Teacher::default());
        let mut slot_desc = test_slot(subject, teacher);
        slot_desc.week_pattern = Some(pattern);
        data.params.slots = Slots::from_subject_rows([(subject, vec![(slot, slot_desc)])]).unwrap();
        assert_eq!(
            data.broken_invariants(),
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
            SlotPairingRule {
                antecedent: SlotRulePart {
                    slot_id: slot_a,
                    should_have: true,
                },
                consequent: SlotRulePart {
                    slot_id: slot_b,
                    should_have: false,
                },
                excluded_periods: BTreeSet::new(),
                soft: false,
            },
        );
        assert_eq!(
            data.broken_invariants(),
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
            data.broken_invariants(),
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
            data.broken_invariants(),
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
            data.broken_invariants(),
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
            data.broken_invariants(),
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
            InnerData::default().broken_invariants(),
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
            data.broken_invariants(),
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
            data.broken_invariants(),
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
            data.broken_invariants(),
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
            data.broken_invariants(),
            Err(BTreeSet::from([LogicError::EmptySlotsRow(subject)]))
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
            data.broken_invariants(),
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
            data.broken_invariants(),
            Err(BTreeSet::from([LogicError::EmptyColloscopeGroupListRow(
                group_list
            )]))
        );
    }

    #[test]
    fn prefill_group_count_mismatch() {
        // One named group, zero prefilled groups: the count differs. Zero groups
        // ⇒ the duplicate-student predicate is vacuously clean, isolating the
        // variant. `group_names` must be set explicitly — the default is 16.
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        data.params.group_lists.group_list_map.insert(
            group_list,
            GroupList {
                params: GroupListParameters {
                    group_names: vec![None],
                    ..Default::default()
                },
                filling: GroupListFilling::Prefilled { groups: vec![] },
            },
        );
        assert_eq!(
            data.broken_invariants(),
            Err(BTreeSet::from([LogicError::PrefillGroupCountMismatch(
                group_list
            )]))
        );
    }

    #[test]
    fn duplicated_student_in_prefilled_groups() {
        // Two named groups, two prefilled groups (count matches), both holding
        // the same student. The student id dangles, but the `Err` path never
        // runs the dangling sweep — only the duplicate-student error surfaces.
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        let student = unsafe { StudentId::new(2) };
        data.params.group_lists.group_list_map.insert(
            group_list,
            GroupList {
                params: GroupListParameters {
                    group_names: vec![None, None],
                    ..Default::default()
                },
                filling: GroupListFilling::Prefilled {
                    groups: vec![
                        PrefilledGroup {
                            students: BTreeSet::from([student]),
                        },
                        PrefilledGroup {
                            students: BTreeSet::from([student]),
                        },
                    ],
                },
            },
        );
        assert_eq!(
            data.broken_invariants(),
            Err(BTreeSet::from([
                LogicError::DuplicatedStudentInPrefilledGroups(group_list)
            ]))
        );
    }

    #[test]
    fn pairing_rule_parts_share_subject() {
        let mut data = InnerData::default();
        let rule = unsafe { PairingRuleId::new(1) };
        let subject = unsafe { SubjectId::new(2) };
        data.params.pairings.pairing_rule_map.insert(
            rule,
            PairingRule {
                antecedent: RulePart {
                    subject_id: subject,
                    should_have: true,
                },
                consequent: RulePart {
                    subject_id: subject,
                    should_have: false,
                },
                excluded_periods: BTreeSet::new(),
                soft: false,
            },
        );
        assert_eq!(
            data.broken_invariants(),
            Err(BTreeSet::from([LogicError::PairingRulePartsShareSubject(
                rule
            )]))
        );
    }

    #[test]
    fn slot_pairing_rule_parts_share_slot() {
        let mut data = InnerData::default();
        let rule = unsafe { SlotPairingRuleId::new(1) };
        let slot = unsafe { SlotId::new(2) };
        data.params.slot_pairings.slot_pairing_rule_map.insert(
            rule,
            SlotPairingRule {
                antecedent: SlotRulePart {
                    slot_id: slot,
                    should_have: true,
                },
                consequent: SlotRulePart {
                    slot_id: slot,
                    should_have: false,
                },
                excluded_periods: BTreeSet::new(),
                soft: false,
            },
        );
        assert_eq!(
            data.broken_invariants(),
            Err(BTreeSet::from([LogicError::SlotPairingRulePartsShareSlot(
                rule
            )]))
        );
    }

    #[test]
    fn both_prefill_errors_on_one_list() {
        // Three named groups but only two prefilled groups (count mismatch), and
        // those two share a student (duplicate). Both predicates fire on the same
        // list — the exhaustive-collection contract the old first-error checker
        // could not honor.
        let mut data = InnerData::default();
        let group_list = unsafe { GroupListId::new(1) };
        let student = unsafe { StudentId::new(2) };
        data.params.group_lists.group_list_map.insert(
            group_list,
            GroupList {
                params: GroupListParameters {
                    group_names: vec![None, None, None],
                    ..Default::default()
                },
                filling: GroupListFilling::Prefilled {
                    groups: vec![
                        PrefilledGroup {
                            students: BTreeSet::from([student]),
                        },
                        PrefilledGroup {
                            students: BTreeSet::from([student]),
                        },
                    ],
                },
            },
        );
        assert_eq!(
            data.broken_invariants(),
            Err(BTreeSet::from([
                LogicError::PrefillGroupCountMismatch(group_list),
                LogicError::DuplicatedStudentInPrefilledGroups(group_list),
            ]))
        );
    }

    #[test]
    fn multiple_logic_errors_all_reported() {
        // A duplicate id, an empty assignments row, and a degenerate pairing rule
        // in one state: all three surface together.
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
        let rule = unsafe { PairingRuleId::new(4) };
        let shared = unsafe { SubjectId::new(5) };
        data.params.pairings.pairing_rule_map.insert(
            rule,
            PairingRule {
                antecedent: RulePart {
                    subject_id: shared,
                    should_have: true,
                },
                consequent: RulePart {
                    subject_id: shared,
                    should_have: false,
                },
                excluded_periods: BTreeSet::new(),
                soft: false,
            },
        );
        assert_eq!(
            data.broken_invariants(),
            Err(BTreeSet::from([
                LogicError::DuplicatedId(1),
                LogicError::EmptyAssignmentsRow(period, subject),
                LogicError::PairingRulePartsShareSubject(rule),
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
            data.broken_invariants(),
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
            data.broken_invariants(),
            Err(BTreeSet::from([LogicError::EmptyAssignmentsRow(
                empty_period,
                empty_subject
            )]))
        );
    }
}
