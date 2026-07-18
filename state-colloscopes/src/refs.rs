//! Reference registry: where every ID-based relationship lives, walkable once.
//!
//! A per-kind `*RefSite` enum (`PeriodRefSite`, `WeekRefSite`, …) names *one
//! place* a reference to that kind lives (a field, a dense-mirror key, a
//! colloscope key…). A site alone never locates a reference — it names the
//! *shape* of the referencing row minus the target; the target id is the other
//! half. **Site + target together are the complete coordinates of the id
//! occurrence.** [Reference] composes the two into one edge value.
//!
//! The payload of a site is the *key complement*: the referencing row's
//! coordinates minus the target. So no target is ever duplicated into a payload,
//! and nothing derivable from the key is carried (e.g. the association entry's
//! group-list *value* is dropped from the period/subject sides — it is looked up
//! from the `(period, subject)` key).
//!
//! The unit of account is the *id occurrence*, not the row. A sparse row keyed
//! on two ids (a colloscope interrogation `(slot, week)`, an assignments key
//! `(period, subject)`, an association entry) yields several distinct sites, one
//! per per-kind enum — the same split that gives a pairing rule's antecedent and
//! consequent distinct subject sites. Deleting either target resolves all of the
//! row's errors through the same op (remove the row); the canonical order
//! arbitrates that, it is not a reason to merge the sites.
//!
//! [RefVisitor] is called once per reference, in a fixed, documented order (see
//! [InnerData::walk_refs], which composes the family, dense-mirror and colloscope
//! walks below):
//!
//! 1. weeks (`week_map` id order) — `period_id`
//! 2. subjects (`OrderedTable` order) — `excluded_periods` (set order)
//! 3. teachers (id order) — `subjects`
//! 4. students (id order) — `excluded_periods`
//! 5. slots (`slot_map` id order) — `subject_id`, `teacher_id`, `week_pattern`
//! 6. incompats (id order) — `subject_id`, `week_pattern_id`
//! 7. pairings (id order) — `antecedent`, `consequent`, `excluded_periods`
//!    (antecedent and consequent yield distinct sites)
//! 8. slot pairings (id order) — `antecedent`, `consequent`, `excluded_periods`
//!    (antecedent and consequent yield distinct sites)
//! 9. group lists (id order) — filling students
//! 10. `settings.students` keys
//! 11. `balancing.subjects` keys
//! 12. week patterns (id order) — `excluded_weeks` → weeks (set order)
//! 13. sparse mirrors, in this order:
//!     a. `assignments.map` (per entry: key site to period then subject, then the
//!        assigned students), in `(period, subject)` key order
//!     b. `group_lists.subjects_associations` (per entry: period, subject, group
//!        list), in `(period, subject)` key order
//! 14. colloscope: interrogation rows keyed `(slot, week)` — each emits a slot
//!     key site then a week key site, in surface order (period → slot → week) —
//!     then `group_lists` rows (group-list key site, then that list's placed
//!     students → student)
//!
//! ## Documented exclusions
//!
//! - `SubjectStillHasNonEmptySlotInColloscope` (update-only, and indirect via
//!   slot → subject): handled later as a wrapper (item 3), not a reference site.
//! - `slots.ordering` keys *and* row values: a pure mirror of `slot_map` (a row
//!   exists iff the subject has ≥1 slot, canonical-absent, and the key matches
//!   each slot's `subject_id`), so the keys add nothing over the per-slot
//!   `SlotSubject` sites; the values are covered by the structural
//!   no-orphan/count checks.
//! - `periods.ordered_period_list` row *values*: a pure mirror of `week_map` /
//!   `Week::period_id`, covered by `check_periods_data_consistency`.
//! - *transitive* references are not materialized. A week pattern references a
//!   period only through the weeks it excludes; that edge is derivable from
//!   `WeekPatternExcludedWeek` (pattern → week) composed with `WeekPeriodFk`
//!   (week → period), and the cascade derives period blocking through week
//!   deletion — so no direct pattern → period site exists.
//! - colloscope group *indices*: not ids.

use collomatique_state::References;

use crate::InnerData;
use crate::colloscope_params::Parameters;
use crate::colloscopes::Colloscope;
use crate::group_lists::GroupListFilling;
use crate::ids::{
    GroupListId, IncompatId, NewId, PairingRuleId, PeriodId, SlotId, SlotPairingRuleId, StudentId,
    SubjectId, TeacherId, WeekId, WeekPatternId,
};

/// One place a reference to a *period* lives. The payload is the referencing
/// row's coordinates minus the period; site + the target period are the complete
/// coordinates of the occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodRefSite {
    /// `Week::period_id` → the period the week belongs to
    WeekPeriodFk(WeekId),
    /// `Subject::excluded_periods` → a period
    SubjectExcludedPeriods(SubjectId),
    /// `Student::excluded_periods` → a period
    StudentExcludedPeriods(StudentId),
    /// `PairingRule::excluded_periods` → a period
    PairingRuleExcludedPeriods(PairingRuleId),
    /// `SlotPairingRule::excluded_periods` → a period
    SlotPairingRuleExcludedPeriods(SlotPairingRuleId),
    /// The period component of an `assignments.map` key — the target period plus
    /// `subject` form the full `(period, subject)` key. Rows are canonical-absent
    /// (a row exists iff it holds an assigned student), so a walked row is always
    /// non-trivial.
    AssignmentsKey { subject: SubjectId },
    /// The period component of a `group_lists.subjects_associations` key
    AssociationEntry { subject: SubjectId },
}

/// One place a reference to a *week* lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeekRefSite {
    /// `WeekPattern::excluded_weeks` → a week the pattern disables. This is the
    /// direct reference the per-week `NonTrivialWeekPattern` guard enforces on
    /// week removal.
    WeekPatternExcludedWeek(WeekPatternId),
    /// The week component of a colloscope interrogation row key — the target week
    /// plus `slot` form the full `(slot, week)` key. Rows are canonical-absent (a
    /// row exists iff it holds an assigned group), so a walked row is always
    /// non-trivial.
    ColloscopeInterrogation { slot: SlotId },
}

/// One place a reference to a *subject* lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubjectRefSite {
    /// `Teacher::subjects` → a subject
    TeacherSubjects(TeacherId),
    /// `Slot::subject_id` → a subject
    SlotSubject(SlotId),
    /// `Incompatibility::subject_id` → a subject
    IncompatSubject(IncompatId),
    /// `PairingRule::antecedent.subject_id` → a subject
    PairingRuleAntecedent(PairingRuleId),
    /// `PairingRule::consequent.subject_id` → a subject
    PairingRuleConsequent(PairingRuleId),
    /// `balancing.subjects` has a per-subject entry keyed by a subject
    BalancingSubjectKey,
    /// The subject component of an `assignments.map` key
    AssignmentsKey { period: PeriodId },
    /// The subject component of a `group_lists.subjects_associations` key
    AssociationEntry { period: PeriodId },
}

/// One place a reference to a *teacher* lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeacherRefSite {
    /// `Slot::teacher_id` → a teacher
    SlotTeacher(SlotId),
}

/// One place a reference to a *student* lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StudentRefSite {
    /// A prefilled group of the group list references a student
    GroupListPrefilledStudent(GroupListId),
    /// An automatic group list excludes a student
    GroupListExcludedStudent(GroupListId),
    /// `settings.students` has a per-student entry keyed by a student
    SettingsStudentKey,
    /// A student assigned in an `assignments.map` cell `(period, subject)` — here
    /// both key components are payload: neither is the target.
    AssignmentsStudent {
        period: PeriodId,
        subject: SubjectId,
    },
    /// A student placed in a colloscope group-list row
    ColloscopeGroupListStudent(GroupListId),
}

/// One place a reference to a *week pattern* lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeekPatternRefSite {
    /// `Slot::week_pattern` → a week pattern
    SlotWeekPattern(SlotId),
    /// `Incompatibility::week_pattern_id` → a week pattern
    IncompatWeekPattern(IncompatId),
}

/// One place a reference to a *slot* lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlotRefSite {
    /// `SlotPairingRule::antecedent.slot_id` → a slot
    SlotPairingRuleAntecedent(SlotPairingRuleId),
    /// `SlotPairingRule::consequent.slot_id` → a slot
    SlotPairingRuleConsequent(SlotPairingRuleId),
    /// The slot component of a colloscope interrogation row key — the target slot
    /// plus `week` form the full `(slot, week)` key. Rows are canonical-absent,
    /// so a walked row is always non-trivial.
    ColloscopeInterrogation { week: WeekId },
}

/// One place a reference to a *group list* lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GroupListRefSite {
    /// The group-list *value* of a `group_lists.subjects_associations` entry —
    /// the value isn't part of the key, so locating it needs the full
    /// `(period, subject)` key.
    AssociationEntry {
        period: PeriodId,
        subject: SubjectId,
    },
    /// A colloscope group-list row key — the target group list *is* the key, so
    /// there is no complement to carry. Rows are canonical-absent (a row exists
    /// iff it holds a placement).
    ColloscopeGroupListKey,
}

/// One full reference edge: a target id plus the site where it lives.
///
/// Kind-shaped — exactly one variant per target kind, no mixed variants. Site +
/// target together are the complete coordinates of the id occurrence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reference {
    /// A reference to a period.
    Period {
        target: PeriodId,
        site: PeriodRefSite,
    },
    /// A reference to a week.
    Week { target: WeekId, site: WeekRefSite },
    /// A reference to a subject.
    Subject {
        target: SubjectId,
        site: SubjectRefSite,
    },
    /// A reference to a teacher.
    Teacher {
        target: TeacherId,
        site: TeacherRefSite,
    },
    /// A reference to a student.
    Student {
        target: StudentId,
        site: StudentRefSite,
    },
    /// A reference to a week pattern.
    WeekPattern {
        target: WeekPatternId,
        site: WeekPatternRefSite,
    },
    /// A reference to a slot.
    Slot { target: SlotId, site: SlotRefSite },
    /// A reference to a group list.
    GroupList {
        target: GroupListId,
        site: GroupListRefSite,
    },
}

/// Visitor over every reference in a document.
///
/// One callback per referenced kind (`IncompatId`, `PairingRuleId` and
/// `SlotPairingRuleId` are never *referenced*, so they get no callback). Each
/// callback takes its own per-kind site type, so a filtering visitor matches
/// exhaustively over that kind's real cases only — no impossible arms, no
/// catchall. Every callback defaults to a no-op so a visitor implements only the
/// ones it cares about.
pub trait RefVisitor {
    /// A reference to `target` (a period) lives at `site`.
    fn period_ref(&mut self, _target: PeriodId, _site: PeriodRefSite) {}
    /// A reference to `target` (a week) lives at `site`.
    fn week_ref(&mut self, _target: WeekId, _site: WeekRefSite) {}
    /// A reference to `target` (a subject) lives at `site`.
    fn subject_ref(&mut self, _target: SubjectId, _site: SubjectRefSite) {}
    /// A reference to `target` (a teacher) lives at `site`.
    fn teacher_ref(&mut self, _target: TeacherId, _site: TeacherRefSite) {}
    /// A reference to `target` (a student) lives at `site`.
    fn student_ref(&mut self, _target: StudentId, _site: StudentRefSite) {}
    /// A reference to `target` (a week pattern) lives at `site`.
    fn week_pattern_ref(&mut self, _target: WeekPatternId, _site: WeekPatternRefSite) {}
    /// A reference to `target` (a slot) lives at `site`.
    fn slot_ref(&mut self, _target: SlotId, _site: SlotRefSite) {}
    /// A reference to `target` (a group list) lives at `site`.
    fn group_list_ref(&mut self, _target: GroupListId, _site: GroupListRefSite) {}
}

/// Walks the reference sites that live directly in [Parameters] entity families
/// (steps 1–12 of the module walk order), in the fixed documented order.
///
/// The dense-mirror and colloscope sites are walked separately;
/// [InnerData::walk_refs] composes all three.
pub(crate) fn walk_params_refs(params: &Parameters, v: &mut impl RefVisitor) {
    walk_weeks(params, v);
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
    walk_week_patterns(params, v);
}

// The per-entity family walkers below drive `References::for_each_ref` and map
// each yielded `NewId` to its per-kind site. `for_each_ref` visits fields in
// declaration order, which is exactly the documented within-entity walk order;
// every arm that cannot occur for that entity is `unreachable!`.
//
// The site is usually named by the referenced-id *kind* alone. When one entity
// yields the same kind from two fields that must map to *distinct* sites
// (antecedent vs consequent of a pairing rule), the walker sub-walks each field
// through its own `References` impl rather than matching off the whole value —
// see `walk_pairings` / `walk_slot_pairings`.

fn walk_weeks(params: &Parameters, v: &mut impl RefVisitor) {
    for (week_id, week) in params.periods.week_entries() {
        week.for_each_ref(&mut |id: NewId| match id {
            NewId::PeriodId(p) => v.period_ref(p, PeriodRefSite::WeekPeriodFk(week_id)),
            _ => unreachable!("Week only references its period"),
        });
    }
}

fn walk_subjects(params: &Parameters, v: &mut impl RefVisitor) {
    for (subject_id, subject) in params.subjects.ordered_subject_list.iter() {
        subject.for_each_ref(&mut |id: NewId| match id {
            NewId::PeriodId(p) => {
                v.period_ref(p, PeriodRefSite::SubjectExcludedPeriods(subject_id))
            }
            _ => unreachable!("Subject only references periods"),
        });
    }
}

fn walk_teachers(params: &Parameters, v: &mut impl RefVisitor) {
    for (teacher_id, teacher) in params.teachers.teacher_map.iter() {
        teacher.for_each_ref(&mut |id: NewId| match id {
            NewId::SubjectId(s) => v.subject_ref(s, SubjectRefSite::TeacherSubjects(teacher_id)),
            _ => unreachable!("Teacher only references subjects"),
        });
    }
}

fn walk_students(params: &Parameters, v: &mut impl RefVisitor) {
    for (student_id, student) in params.students.student_map.iter() {
        student.for_each_ref(&mut |id: NewId| match id {
            NewId::PeriodId(p) => {
                v.period_ref(p, PeriodRefSite::StudentExcludedPeriods(student_id))
            }
            _ => unreachable!("Student only references periods"),
        });
    }
}

fn walk_slots(params: &Parameters, v: &mut impl RefVisitor) {
    for (slot_id, slot) in params.slots.slot_entries() {
        slot.for_each_ref(&mut |id: NewId| match id {
            NewId::SubjectId(s) => v.subject_ref(s, SubjectRefSite::SlotSubject(slot_id)),
            NewId::TeacherId(t) => v.teacher_ref(t, TeacherRefSite::SlotTeacher(slot_id)),
            NewId::WeekPatternId(w) => {
                v.week_pattern_ref(w, WeekPatternRefSite::SlotWeekPattern(slot_id))
            }
            _ => unreachable!("Slot only references subject/teacher/week pattern"),
        });
    }
}

fn walk_incompats(params: &Parameters, v: &mut impl RefVisitor) {
    for (incompat_id, incompat) in params.incompats.incompat_map.iter() {
        incompat.for_each_ref(&mut |id: NewId| match id {
            NewId::SubjectId(s) => v.subject_ref(s, SubjectRefSite::IncompatSubject(incompat_id)),
            NewId::WeekPatternId(w) => {
                v.week_pattern_ref(w, WeekPatternRefSite::IncompatWeekPattern(incompat_id))
            }
            _ => unreachable!("Incompatibility only references subject/week pattern"),
        });
    }
}

fn walk_pairings(params: &Parameters, v: &mut impl RefVisitor) {
    // Antecedent and consequent must yield distinct sites, so each part is
    // sub-walked through its own `References` impl rather than matched off the
    // whole rule. Order — antecedent, consequent, excluded periods — matches the
    // `PairingRule` field declaration order the composed walk would produce.
    for (rule_id, rule) in params.pairings.pairing_rule_map.iter() {
        rule.antecedent.for_each_ref(&mut |id: NewId| match id {
            NewId::SubjectId(s) => v.subject_ref(s, SubjectRefSite::PairingRuleAntecedent(rule_id)),
            _ => unreachable!("RulePart only references a subject"),
        });
        rule.consequent.for_each_ref(&mut |id: NewId| match id {
            NewId::SubjectId(s) => v.subject_ref(s, SubjectRefSite::PairingRuleConsequent(rule_id)),
            _ => unreachable!("RulePart only references a subject"),
        });
        rule.excluded_periods
            .for_each_ref(&mut |id: NewId| match id {
                NewId::PeriodId(p) => {
                    v.period_ref(p, PeriodRefSite::PairingRuleExcludedPeriods(rule_id))
                }
                _ => unreachable!("excluded_periods only references periods"),
            });
    }
}

fn walk_slot_pairings(params: &Parameters, v: &mut impl RefVisitor) {
    // Antecedent and consequent must yield distinct sites, so each part is
    // sub-walked through its own `References` impl (as in `walk_pairings`).
    for (rule_id, rule) in params.slot_pairings.slot_pairing_rule_map.iter() {
        rule.antecedent.for_each_ref(&mut |id: NewId| match id {
            NewId::SlotId(s) => v.slot_ref(s, SlotRefSite::SlotPairingRuleAntecedent(rule_id)),
            _ => unreachable!("SlotRulePart only references a slot"),
        });
        rule.consequent.for_each_ref(&mut |id: NewId| match id {
            NewId::SlotId(s) => v.slot_ref(s, SlotRefSite::SlotPairingRuleConsequent(rule_id)),
            _ => unreachable!("SlotRulePart only references a slot"),
        });
        rule.excluded_periods
            .for_each_ref(&mut |id: NewId| match id {
                NewId::PeriodId(p) => {
                    v.period_ref(p, PeriodRefSite::SlotPairingRuleExcludedPeriods(rule_id))
                }
                _ => unreachable!("excluded_periods only references periods"),
            });
    }
}

fn walk_group_lists(params: &Parameters, v: &mut impl RefVisitor) {
    for (gl_id, gl) in params.group_lists.group_list_map.iter() {
        // A group list value is exactly one filling variant, so pre-matching once
        // picks the site constructor soundly (the two student sites are
        // distinguished by variant, not by which student is referenced).
        let site = match &gl.filling {
            GroupListFilling::Prefilled { .. } => StudentRefSite::GroupListPrefilledStudent(gl_id),
            GroupListFilling::Automatic { .. } => StudentRefSite::GroupListExcludedStudent(gl_id),
        };
        gl.for_each_ref(&mut |id: NewId| match id {
            NewId::StudentId(s) => v.student_ref(s, site),
            _ => unreachable!("GroupList only references students"),
        });
    }
}

fn walk_settings_keys(params: &Parameters, v: &mut impl RefVisitor) {
    for student_id in params.settings.students.keys() {
        v.student_ref(student_id, StudentRefSite::SettingsStudentKey);
    }
}

fn walk_balancing_keys(params: &Parameters, v: &mut impl RefVisitor) {
    for subject_id in params.balancing.subjects.keys() {
        v.subject_ref(subject_id, SubjectRefSite::BalancingSubjectKey);
    }
}

fn walk_week_patterns(params: &Parameters, v: &mut impl RefVisitor) {
    for (wp_id, wp) in params.week_patterns.week_pattern_map.iter() {
        wp.for_each_ref(&mut |id: NewId| match id {
            NewId::WeekId(w) => v.week_ref(w, WeekRefSite::WeekPatternExcludedWeek(wp_id)),
            _ => unreachable!("Week pattern only references the weeks it excludes"),
        });
    }
}

/// Walks the sparse assignments mirror (step 13a): each stored `(period,
/// subject)` row references *both* a period and a subject, then each assigned
/// student. The period and subject sites each carry the *other* key component as
/// payload. Rows are canonical-absent, so a walked row is always non-trivial.
fn walk_assignments(params: &Parameters, v: &mut impl RefVisitor) {
    for ((period, subject), students) in params.assignments.map.iter() {
        v.period_ref(period, PeriodRefSite::AssignmentsKey { subject });
        v.subject_ref(subject, SubjectRefSite::AssignmentsKey { period });
        for &student_id in students {
            v.student_ref(
                student_id,
                StudentRefSite::AssignmentsStudent { period, subject },
            );
        }
    }
}

/// Walks the subject/group-list association mirror (step 13b): each entry
/// references a period, a subject and a group list at once. The period and
/// subject sites carry the other key component; the group-list site (its value
/// is not part of the key) carries the full `(period, subject)` key.
fn walk_associations(params: &Parameters, v: &mut impl RefVisitor) {
    for ((period, subject), group_list) in params.group_lists.subjects_associations.iter() {
        v.period_ref(period, PeriodRefSite::AssociationEntry { subject });
        v.subject_ref(subject, SubjectRefSite::AssociationEntry { period });
        v.group_list_ref(
            *group_list,
            GroupListRefSite::AssociationEntry { period, subject },
        );
    }
}

/// Walks the colloscope (step 14): each interrogation row references both a
/// slot and a week (two-sided, like the assignments mirror) — each site carries
/// the other key component — then each group-list row references its list (the
/// target *is* the key, so no payload) and every placed student. Rows are
/// canonical-absent, so a walked row is always non-trivial.
fn walk_colloscope(colloscope: &Colloscope, v: &mut impl RefVisitor) {
    for ((slot_id, week_id), _assigned_groups) in colloscope.iter() {
        v.slot_ref(
            slot_id,
            SlotRefSite::ColloscopeInterrogation { week: week_id },
        );
        v.week_ref(
            week_id,
            WeekRefSite::ColloscopeInterrogation { slot: slot_id },
        );
    }
    for (gl_id, groups_for_students) in colloscope.group_lists_iter() {
        v.group_list_ref(gl_id, GroupListRefSite::ColloscopeGroupListKey);
        for student_id in groups_for_students.keys() {
            v.student_ref(
                *student_id,
                StudentRefSite::ColloscopeGroupListStudent(gl_id),
            );
        }
    }
}

impl InnerData {
    /// Walks every reference in the document, in the documented fixed order (see
    /// the module docs): first the [Parameters] entity families (steps 1–12),
    /// then the dense mirrors (step 13), then the colloscope (step 14).
    pub fn walk_refs(&self, v: &mut impl RefVisitor) {
        walk_params_refs(&self.params, v);
        walk_assignments(&self.params, v);
        walk_associations(&self.params, v);
        walk_colloscope(&self.colloscope, v);
    }

    /// Walks every reference as a full [Reference] edge, in the same fixed order
    /// as [InnerData::walk_refs]. For consumers that want the whole info in one
    /// place (heterogeneous collections, generic dangling-ref reports) rather
    /// than a per-kind callback.
    ///
    /// The name parallels the `References::for_each_ref` derive method — same
    /// verb, but at the document level (a full edge) instead of the entity level
    /// (a raw `NewId`).
    pub fn for_each_reference(&self, f: &mut impl FnMut(Reference)) {
        struct Funnel<'a, F>(&'a mut F);
        impl<F: FnMut(Reference)> RefVisitor for Funnel<'_, F> {
            fn period_ref(&mut self, target: PeriodId, site: PeriodRefSite) {
                (self.0)(Reference::Period { target, site });
            }
            fn week_ref(&mut self, target: WeekId, site: WeekRefSite) {
                (self.0)(Reference::Week { target, site });
            }
            fn subject_ref(&mut self, target: SubjectId, site: SubjectRefSite) {
                (self.0)(Reference::Subject { target, site });
            }
            fn teacher_ref(&mut self, target: TeacherId, site: TeacherRefSite) {
                (self.0)(Reference::Teacher { target, site });
            }
            fn student_ref(&mut self, target: StudentId, site: StudentRefSite) {
                (self.0)(Reference::Student { target, site });
            }
            fn week_pattern_ref(&mut self, target: WeekPatternId, site: WeekPatternRefSite) {
                (self.0)(Reference::WeekPattern { target, site });
            }
            fn slot_ref(&mut self, target: SlotId, site: SlotRefSite) {
                (self.0)(Reference::Slot { target, site });
            }
            fn group_list_ref(&mut self, target: GroupListId, site: GroupListRefSite) {
                (self.0)(Reference::GroupList { target, site });
            }
        }
        self.walk_refs(&mut Funnel(f));
    }
}

/// Generates a `references_to_*` reverse lookup: every per-kind site whose target
/// is the given id, in walk order. Each shares one filtering [RefVisitor].
macro_rules! references_to_impl {
    ($(#[$m:meta])* $fn_name:ident, $id_ty:ty, $site_ty:ty, $callback:ident) => {
        $(#[$m])*
        pub fn $fn_name(&self, id: $id_ty) -> Vec<$site_ty> {
            struct Filter {
                target: $id_ty,
                sites: Vec<$site_ty>,
            }
            impl RefVisitor for Filter {
                fn $callback(&mut self, target: $id_ty, site: $site_ty) {
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
        references_to_period, PeriodId, PeriodRefSite, period_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given week, in walk order.
        references_to_week, WeekId, WeekRefSite, week_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given subject, in walk order.
        references_to_subject, SubjectId, SubjectRefSite, subject_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given teacher, in walk order.
        references_to_teacher, TeacherId, TeacherRefSite, teacher_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given student, in walk order.
        references_to_student, StudentId, StudentRefSite, student_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given week pattern, in walk order.
        references_to_week_pattern, WeekPatternId, WeekPatternRefSite, week_pattern_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given slot, in walk order.
        references_to_slot, SlotId, SlotRefSite, slot_ref
    );
    references_to_impl!(
        /// Every reference site targeting the given group list, in walk order.
        references_to_group_list, GroupListId, GroupListRefSite, group_list_ref
    );
}
