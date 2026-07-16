//! Reference registry: where every ID-based relationship lives, walkable once.
//!
//! A [RefSite] names *one place* a reference to some entity lives (a field, a
//! dense-mirror key, a colloscope key…). Its payload is the *referencing*
//! entity's coordinates — mirroring what the delete-blocking errors carry — not
//! the target id (the target is the visitor-callback argument / query argument).
//!
//! [RefVisitor] is called once per reference, in a fixed, documented order (see
//! [InnerData::walk_refs], which composes the family, dense-mirror and colloscope
//! walks below):
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
//! 12. dense mirrors, in this order:
//!     a. `assignments.map` (per entry: key site to period then subject, then the
//!        assigned students), in `(period, subject)` key order
//!     b. `group_lists.subjects_associations` (per entry: period, subject, group
//!        list), in `(period, subject)` key order
//!     c. `slots.ordering` keys → subject, in subject-id order
//! 13. colloscope: `period_map` entries (period key site, then that period's
//!     `slot_map` key sites), then `group_lists` entries (group-list key site,
//!     then that list's `groups_for_students` keys → student)
//!
//! ## Documented exclusions
//!
//! - `SubjectStillHasNonEmptySlotInColloscope` (update-only, and indirect via
//!   slot → subject): handled later as a wrapper (item 3), not a reference site.
//! - `slots.ordering` row *values*: a pure mirror of `slot_map` keys, covered by
//!   the structural no-orphan/count checks.
//! - colloscope group *indices*: not ids.

use collomatique_state::References;

use crate::InnerData;
use crate::colloscope_params::Parameters;
use crate::colloscopes::Colloscope;
use crate::group_lists::GroupListFilling;
use crate::ids::{
    GroupListId, IncompatId, NewId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
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
/// The dense-mirror and colloscope sites are walked separately;
/// [InnerData::walk_refs] composes all three.
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

// The per-entity family walkers below drive `References::for_each_ref` and map
// each yielded `NewId` to its [RefSite]. Within a single entity value the
// referenced-id *kind* names the site uniquely (verified against the relationship
// inventory), so the `match` on `NewId` is exhaustive over the kinds that entity
// can reference; every other arm is `unreachable!`. `for_each_ref` visits fields
// in declaration order, which is exactly the documented within-entity walk order.

fn walk_subjects(params: &Parameters, v: &mut impl RefVisitor) {
    for (subject_id, subject) in params.subjects.ordered_subject_list.iter() {
        subject.for_each_ref(&mut |id: NewId| match id {
            NewId::PeriodId(p) => v.period_ref(p, RefSite::SubjectExcludedPeriods(subject_id)),
            _ => unreachable!("Subject only references periods"),
        });
    }
}

fn walk_teachers(params: &Parameters, v: &mut impl RefVisitor) {
    for (teacher_id, teacher) in params.teachers.teacher_map.iter() {
        teacher.for_each_ref(&mut |id: NewId| match id {
            NewId::SubjectId(s) => v.subject_ref(s, RefSite::TeacherSubjects(teacher_id)),
            _ => unreachable!("Teacher only references subjects"),
        });
    }
}

fn walk_students(params: &Parameters, v: &mut impl RefVisitor) {
    for (student_id, student) in params.students.student_map.iter() {
        student.for_each_ref(&mut |id: NewId| match id {
            NewId::PeriodId(p) => v.period_ref(p, RefSite::StudentExcludedPeriods(student_id)),
            _ => unreachable!("Student only references periods"),
        });
    }
}

fn walk_slots(params: &Parameters, v: &mut impl RefVisitor) {
    for (slot_id, slot) in params.slots.slot_entries() {
        slot.for_each_ref(&mut |id: NewId| match id {
            NewId::SubjectId(s) => v.subject_ref(s, RefSite::SlotSubject(slot_id)),
            NewId::TeacherId(t) => v.teacher_ref(t, RefSite::SlotTeacher(slot_id)),
            NewId::WeekPatternId(w) => v.week_pattern_ref(w, RefSite::SlotWeekPattern(slot_id)),
            _ => unreachable!("Slot only references subject/teacher/week pattern"),
        });
    }
}

fn walk_incompats(params: &Parameters, v: &mut impl RefVisitor) {
    for (incompat_id, incompat) in params.incompats.incompat_map.iter() {
        incompat.for_each_ref(&mut |id: NewId| match id {
            NewId::SubjectId(s) => v.subject_ref(s, RefSite::IncompatSubject(incompat_id)),
            NewId::WeekPatternId(w) => {
                v.week_pattern_ref(w, RefSite::IncompatWeekPattern(incompat_id))
            }
            _ => unreachable!("Incompatibility only references subject/week pattern"),
        });
    }
}

fn walk_pairings(params: &Parameters, v: &mut impl RefVisitor) {
    // Both parts share `RefSite::PairingRulePart` (the block error does not
    // distinguish them); `for_each_ref` visits antecedent then consequent then
    // the excluded periods, matching the documented order.
    for (rule_id, rule) in params.pairings.pairing_rule_map.iter() {
        rule.for_each_ref(&mut |id: NewId| match id {
            NewId::SubjectId(s) => v.subject_ref(s, RefSite::PairingRulePart(rule_id)),
            NewId::PeriodId(p) => v.period_ref(p, RefSite::PairingRuleExcludedPeriods(rule_id)),
            _ => unreachable!("PairingRule only references subject/period"),
        });
    }
}

fn walk_slot_pairings(params: &Parameters, v: &mut impl RefVisitor) {
    for (rule_id, rule) in params.slot_pairings.slot_pairing_rule_map.iter() {
        rule.for_each_ref(&mut |id: NewId| match id {
            NewId::SlotId(s) => v.slot_ref(s, RefSite::SlotPairingRulePart(rule_id)),
            NewId::PeriodId(p) => v.period_ref(p, RefSite::SlotPairingRuleExcludedPeriods(rule_id)),
            _ => unreachable!("SlotPairingRule only references slot/period"),
        });
    }
}

fn walk_group_lists(params: &Parameters, v: &mut impl RefVisitor) {
    for (gl_id, gl) in params.group_lists.group_list_map.iter() {
        // A group list value is exactly one filling variant, so pre-matching once
        // picks the site constructor soundly (the two student sites are
        // distinguished by variant, not by which student is referenced).
        let site = match &gl.filling {
            GroupListFilling::Prefilled { .. } => RefSite::GroupListPrefilledStudent(gl_id),
            GroupListFilling::Automatic { .. } => RefSite::GroupListExcludedStudent(gl_id),
        };
        gl.for_each_ref(&mut |id: NewId| match id {
            NewId::StudentId(s) => v.student_ref(s, site),
            _ => unreachable!("GroupList only references students"),
        });
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

/// Walks the sparse assignments mirror (step 12a): each stored `(period,
/// subject)` row references *both* a period and a subject, then each assigned
/// student. Rows are canonical-absent, so a walked row is always non-trivial.
fn walk_assignments(params: &Parameters, v: &mut impl RefVisitor) {
    for ((period, subject), students) in params.assignments.map.iter() {
        let site = RefSite::AssignmentsKey {
            period,
            subject,
            non_trivial: !students.is_empty(),
        };
        v.period_ref(period, site);
        v.subject_ref(subject, site);
        for &student_id in students {
            v.student_ref(student_id, RefSite::AssignmentsStudent { period, subject });
        }
    }
}

/// Walks the subject/group-list association mirror (step 12b): each entry
/// references a period, a subject and a group list at once.
fn walk_associations(params: &Parameters, v: &mut impl RefVisitor) {
    for ((period, subject), group_list) in params.group_lists.subjects_associations.iter() {
        let site = RefSite::AssociationEntry {
            period,
            subject,
            group_list: *group_list,
        };
        v.period_ref(period, site);
        v.subject_ref(subject, site);
        v.group_list_ref(*group_list, site);
    }
}

/// Walks the slots ordering mirror keys (step 12c): each key references a subject.
fn walk_slots_ordering_keys(params: &Parameters, v: &mut impl RefVisitor) {
    for (subject_id, row) in params.slots.ordering_entries() {
        v.subject_ref(
            subject_id,
            RefSite::SlotsOrderingKey {
                non_trivial: !row.is_empty(),
            },
        );
    }
}

/// Walks the colloscope (step 13): period keys (then their slot keys), then
/// group-list keys (then their placed students).
fn walk_colloscope(colloscope: &Colloscope, v: &mut impl RefVisitor) {
    for (period_id, collo_period) in colloscope.period_map.iter() {
        v.period_ref(
            *period_id,
            RefSite::ColloscopePeriodKey {
                non_trivial: !collo_period.is_empty(),
            },
        );
        for (slot_id, collo_slot) in collo_period.slot_map.iter() {
            v.slot_ref(
                *slot_id,
                RefSite::ColloscopeSlotKey {
                    period: *period_id,
                    non_trivial: !collo_slot.is_empty(),
                },
            );
        }
    }
    for (gl_id, collo_gl) in colloscope.group_lists.iter() {
        v.group_list_ref(
            *gl_id,
            RefSite::ColloscopeGroupListKey {
                non_trivial: !collo_gl.is_empty(),
            },
        );
        for student_id in collo_gl.groups_for_students.keys() {
            v.student_ref(*student_id, RefSite::ColloscopeGroupListStudent(*gl_id));
        }
    }
}

impl InnerData {
    /// Walks every reference in the document, in the documented fixed order (see
    /// the module docs): first the [Parameters] entity families (steps 1–11),
    /// then the dense mirrors (step 12), then the colloscope (step 13).
    pub fn walk_refs(&self, v: &mut impl RefVisitor) {
        walk_params_refs(&self.params, v);
        walk_assignments(&self.params, v);
        walk_associations(&self.params, v);
        walk_slots_ordering_keys(&self.params, v);
        walk_colloscope(&self.colloscope, v);
    }
}

/// Generates a `references_to_*` reverse lookup: every [RefSite] whose target is
/// the given id, in walk order. Each shares one filtering [RefVisitor].
macro_rules! references_to_impl {
    ($(#[$m:meta])* $fn_name:ident, $id_ty:ty, $callback:ident) => {
        $(#[$m])*
        pub fn $fn_name(&self, id: $id_ty) -> Vec<RefSite> {
            struct Filter {
                target: $id_ty,
                sites: Vec<RefSite>,
            }
            impl RefVisitor for Filter {
                fn $callback(&mut self, target: $id_ty, site: RefSite) {
                    if target == self.target {
                        self.sites.push(site);
                    }
                }
            }
            let mut f = Filter {
                target: id,
                sites: Vec::new(),
            };
            self.walk_refs(&mut f);
            f.sites
        }
    };
}

impl InnerData {
    references_to_impl!(
        /// Every reference site targeting the given period, in walk order.
        references_to_period, PeriodId, period_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given subject, in walk order.
        references_to_subject, SubjectId, subject_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given teacher, in walk order.
        references_to_teacher, TeacherId, teacher_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given student, in walk order.
        references_to_student, StudentId, student_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given week pattern, in walk order.
        references_to_week_pattern, WeekPatternId, week_pattern_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given slot, in walk order.
        references_to_slot, SlotId, slot_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given group list, in walk order.
        references_to_group_list, GroupListId, group_list_ref
    );
}
