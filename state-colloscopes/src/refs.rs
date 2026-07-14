//! Reference registry: where every ID-based relationship lives, walkable once.
//!
//! A [RefSite] names *one place* a reference to some entity lives (a field, a
//! dense-mirror key, a colloscope key…). Its payload is the *referencing*
//! entity's coordinates — mirroring what the delete-blocking errors carry — not
//! the target id (the target is the visitor-callback argument / query argument).
//!
//! [RefVisitor] is called once per reference, in a fixed, documented order (see
//! [walk_params_refs] and, from commit 2 on, `InnerData::walk_refs`):
//!
//! 1. subjects (`OrderedTable` order) — `excluded_periods` (set order)
//! 2. teachers (id order) — `subjects`
//! 3. students (id order) — `excluded_periods`
//! 4. slots (`slot_map` id order) — `subject_id`, `teacher_id`, `week_pattern`
//! 5. incompats (id order) — `subject_id`, `week_pattern_id`
//! 6. pairings (id order) — `antecedent`, `consequent`, `excluded_periods`
//! 7. slot pairings (id order) — `antecedent`, `consequent`, `excluded_periods`
//! 8. group lists (id order) — filling students
//! 9. `settings.students` keys
//! 10. `balancing.subjects` keys
//! 11. week-pattern length coupling: week patterns (id order) × periods (table order)
//!
//! (Dense mirrors and the colloscope are walked after these, in commit 2.)
//!
//! ## Documented exclusions
//!
//! - `SubjectStillHasNonEmptySlotInColloscope` (update-only, and indirect via
//!   slot → subject): handled later as a wrapper (item 3), not a reference site.
//! - `slots.ordering` row *values*: a pure mirror of `slot_map` keys, covered by
//!   the structural no-orphan/count checks.
//! - colloscope group *indices*: not ids.

use crate::colloscope_params::Parameters;
use crate::group_lists::GroupListFilling;
use crate::ids::{
    GroupListId, IncompatId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekPatternId,
};

/// One place a reference to an entity lives.
///
/// The payload is the *referencing* entity's coordinates (mirroring the
/// delete-blocking error payloads); the *target* id is passed alongside to the
/// [RefVisitor] callback rather than stored here.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefSite {
    // --- entity fields ---
    /// `Subject::excluded_periods` → a period
    SubjectExcludedPeriods(SubjectId),
    /// `Student::excluded_periods` → a period
    StudentExcludedPeriods(StudentId),
    /// `PairingRule::excluded_periods` → a period
    PairingRuleExcludedPeriods(PairingRuleId),
    /// `SlotPairingRule::excluded_periods` → a period
    SlotPairingRuleExcludedPeriods(SlotPairingRuleId),
    /// `Teacher::subjects` → a subject
    TeacherSubjects(TeacherId),
    /// `Slot::subject_id` → a subject
    SlotSubject(SlotId),
    /// `Slot::teacher_id` → a teacher
    SlotTeacher(SlotId),
    /// `Slot::week_pattern` → a week pattern
    SlotWeekPattern(SlotId),
    /// `Incompatibility::subject_id` → a subject
    IncompatSubject(IncompatId),
    /// `Incompatibility::week_pattern_id` → a week pattern
    IncompatWeekPattern(IncompatId),
    /// `PairingRule::{antecedent,consequent}.subject_id` → a subject
    ///
    /// One variant for both parts (the block error does not distinguish them);
    /// emitted once **per occurrence** (antecedent then consequent), callers dedup.
    PairingRulePart(PairingRuleId),
    /// `SlotPairingRule::{antecedent,consequent}.slot_id` → a slot
    SlotPairingRulePart(SlotPairingRuleId),
    /// A prefilled group of the group list references a student
    GroupListPrefilledStudent(GroupListId),
    /// An automatic group list excludes a student
    GroupListExcludedStudent(GroupListId),
    /// `settings.students` has a per-student entry keyed by a student
    SettingsStudentKey,
    /// `balancing.subjects` has a per-subject entry keyed by a subject
    BalancingSubjectKey,
    /// Every week pattern spans every period (its length is coupled to the total
    /// number of weeks). `non_trivial` mirrors the delete-blocking predicate:
    /// `true` when removing this period's weeks from the pattern would change it
    /// (`!WeekPattern::can_remove_weeks`).
    WeekPatternLengthCoupling {
        week_pattern: WeekPatternId,
        non_trivial: bool,
    },
    // --- dense mirrors / junctions ---
    /// An `assignments.map` key `(period, subject)` — references *both* a period
    /// and a subject. `non_trivial` is `true` when the cell holds any student.
    AssignmentsKey {
        period: PeriodId,
        subject: SubjectId,
        non_trivial: bool,
    },
    /// A student assigned in an `assignments.map` cell `(period, subject)`
    AssignmentsStudent {
        period: PeriodId,
        subject: SubjectId,
    },
    /// A `group_lists.subjects_associations` entry — references a period, a
    /// subject and a group list at once.
    AssociationEntry {
        period: PeriodId,
        subject: SubjectId,
        group_list: GroupListId,
    },
    /// A `slots.ordering` key — references a subject. `non_trivial` is `true`
    /// when the subject's ordering row is non-empty.
    SlotsOrderingKey { non_trivial: bool },
    // --- colloscope ---
    /// A `colloscope.period_map` key — references a period. `non_trivial`
    /// mirrors `!ColloscopePeriod::is_empty()`.
    ColloscopePeriodKey { non_trivial: bool },
    /// A `ColloscopePeriod::slot_map` key — references a slot. `non_trivial`
    /// mirrors `!ColloscopeSlot::is_empty()`.
    ColloscopeSlotKey { period: PeriodId, non_trivial: bool },
    /// A `colloscope.group_lists` key — references a group list. `non_trivial`
    /// mirrors `!ColloscopeGroupList::is_empty()`.
    ColloscopeGroupListKey { non_trivial: bool },
    /// A student placed in a colloscope group list
    ColloscopeGroupListStudent(GroupListId),
}

/// Visitor over every reference in a document.
///
/// One callback per referenced kind (`IncompatId`, `PairingRuleId` and
/// `SlotPairingRuleId` are never *referenced*, so they get no callback). Every
/// callback defaults to a no-op so a filtering visitor implements only the ones
/// it cares about.
pub trait RefVisitor {
    /// A reference to `target` (a period) lives at `site`.
    fn period_ref(&mut self, _target: PeriodId, _site: RefSite) {}
    /// A reference to `target` (a subject) lives at `site`.
    fn subject_ref(&mut self, _target: SubjectId, _site: RefSite) {}
    /// A reference to `target` (a teacher) lives at `site`.
    fn teacher_ref(&mut self, _target: TeacherId, _site: RefSite) {}
    /// A reference to `target` (a student) lives at `site`.
    fn student_ref(&mut self, _target: StudentId, _site: RefSite) {}
    /// A reference to `target` (a week pattern) lives at `site`.
    fn week_pattern_ref(&mut self, _target: WeekPatternId, _site: RefSite) {}
    /// A reference to `target` (a slot) lives at `site`.
    fn slot_ref(&mut self, _target: SlotId, _site: RefSite) {}
    /// A reference to `target` (a group list) lives at `site`.
    fn group_list_ref(&mut self, _target: GroupListId, _site: RefSite) {}
}

/// Walks the reference sites that live directly in [Parameters] entity families
/// (steps 1–11 of the module walk order), in the fixed documented order.
///
/// The dense-mirror and colloscope sites are walked separately (commit 2);
/// `InnerData::walk_refs` composes both.
pub(crate) fn walk_params_refs(params: &Parameters, v: &mut impl RefVisitor) {
    walk_subjects(params, v);
    walk_teachers(params, v);
    walk_students(params, v);
    walk_slots(params, v);
    walk_incompats(params, v);
    walk_pairings(params, v);
    walk_slot_pairings(params, v);
    walk_group_lists(params, v);
    walk_settings_keys(params, v);
    walk_balancing_keys(params, v);
    walk_week_pattern_coupling(params, v);
}

fn walk_subjects(params: &Parameters, v: &mut impl RefVisitor) {
    for (subject_id, subject) in params.subjects.ordered_subject_list.iter() {
        for &period_id in &subject.excluded_periods {
            v.period_ref(period_id, RefSite::SubjectExcludedPeriods(subject_id));
        }
    }
}

fn walk_teachers(params: &Parameters, v: &mut impl RefVisitor) {
    for (teacher_id, teacher) in params.teachers.teacher_map.iter() {
        for &subject_id in &teacher.subjects {
            v.subject_ref(subject_id, RefSite::TeacherSubjects(teacher_id));
        }
    }
}

fn walk_students(params: &Parameters, v: &mut impl RefVisitor) {
    for (student_id, student) in params.students.student_map.iter() {
        for &period_id in &student.excluded_periods {
            v.period_ref(period_id, RefSite::StudentExcludedPeriods(student_id));
        }
    }
}

fn walk_slots(params: &Parameters, v: &mut impl RefVisitor) {
    for (slot_id, slot) in params.slots.slot_entries() {
        v.subject_ref(slot.subject_id, RefSite::SlotSubject(slot_id));
        v.teacher_ref(slot.teacher_id, RefSite::SlotTeacher(slot_id));
        if let Some(week_pattern) = slot.week_pattern {
            v.week_pattern_ref(week_pattern, RefSite::SlotWeekPattern(slot_id));
        }
    }
}

fn walk_incompats(params: &Parameters, v: &mut impl RefVisitor) {
    for (incompat_id, incompat) in params.incompats.incompat_map.iter() {
        v.subject_ref(incompat.subject_id, RefSite::IncompatSubject(incompat_id));
        if let Some(week_pattern) = incompat.week_pattern_id {
            v.week_pattern_ref(week_pattern, RefSite::IncompatWeekPattern(incompat_id));
        }
    }
}

fn walk_pairings(params: &Parameters, v: &mut impl RefVisitor) {
    for (rule_id, rule) in params.pairings.pairing_rule_map.iter() {
        v.subject_ref(
            rule.antecedent.subject_id,
            RefSite::PairingRulePart(rule_id),
        );
        v.subject_ref(
            rule.consequent.subject_id,
            RefSite::PairingRulePart(rule_id),
        );
        for &period_id in &rule.excluded_periods {
            v.period_ref(period_id, RefSite::PairingRuleExcludedPeriods(rule_id));
        }
    }
}

fn walk_slot_pairings(params: &Parameters, v: &mut impl RefVisitor) {
    for (rule_id, rule) in params.slot_pairings.slot_pairing_rule_map.iter() {
        v.slot_ref(
            rule.antecedent.slot_id,
            RefSite::SlotPairingRulePart(rule_id),
        );
        v.slot_ref(
            rule.consequent.slot_id,
            RefSite::SlotPairingRulePart(rule_id),
        );
        for &period_id in &rule.excluded_periods {
            v.period_ref(period_id, RefSite::SlotPairingRuleExcludedPeriods(rule_id));
        }
    }
}

fn walk_group_lists(params: &Parameters, v: &mut impl RefVisitor) {
    for (gl_id, gl) in params.group_lists.group_list_map.iter() {
        match &gl.filling {
            GroupListFilling::Prefilled { groups } => {
                for group in groups {
                    for &student_id in &group.students {
                        v.student_ref(student_id, RefSite::GroupListPrefilledStudent(gl_id));
                    }
                }
            }
            GroupListFilling::Automatic { excluded_students } => {
                for &student_id in excluded_students {
                    v.student_ref(student_id, RefSite::GroupListExcludedStudent(gl_id));
                }
            }
        }
    }
}

fn walk_settings_keys(params: &Parameters, v: &mut impl RefVisitor) {
    for student_id in params.settings.students.keys() {
        v.student_ref(student_id, RefSite::SettingsStudentKey);
    }
}

fn walk_balancing_keys(params: &Parameters, v: &mut impl RefVisitor) {
    for subject_id in params.balancing.subjects.keys() {
        v.subject_ref(subject_id, RefSite::BalancingSubjectKey);
    }
}

fn walk_week_pattern_coupling(params: &Parameters, v: &mut impl RefVisitor) {
    // Cumulative first-week offset per period, in period order, matching
    // `Periods::find_period_position_and_first_week`.
    let mut spans = Vec::new();
    let mut first_week = 0usize;
    for (period_id, weeks) in params.periods.ordered_period_list.iter() {
        spans.push((period_id, first_week, weeks.len()));
        first_week += weeks.len();
    }
    for (wp_id, wp) in params.week_patterns.week_pattern_map.iter() {
        for &(period_id, first_week, week_count) in &spans {
            // Removing zero weeks is always trivial (and `can_remove_weeks`
            // would assert on an empty span); otherwise mirror the exact
            // delete-blocking predicate (periods.rs Remove path).
            let non_trivial = week_count != 0 && !wp.can_remove_weeks(first_week, week_count);
            v.period_ref(
                period_id,
                RefSite::WeekPatternLengthCoupling {
                    week_pattern: wp_id,
                    non_trivial,
                },
            );
        }
    }
}

/// TEMPORARY test hook (removed in commit 2 when `InnerData::walk_refs` lands).
///
/// Lets the integration pin test drive [walk_params_refs] before the public
/// `walk_refs` entry point exists.
#[doc(hidden)]
pub fn walk_params_refs_for_tests(params: &Parameters, v: &mut impl RefVisitor) {
    walk_params_refs(params, v);
}
