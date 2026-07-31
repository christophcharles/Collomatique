//! Resolution map: one repair step per broken invariant.
//!
//! This is the colloscope side of the step-6 cascade. The engine
//! ([collomatique_state::apply_cascade]) applies a target op; when the
//! apply/check/rollback gate rejects it on broken invariants, the engine picks
//! the canonically-first one and asks this map to repair it, then retries.
//! Nothing here is exported: the map surfaces only through the [Fixable] impl.
//!
//! The whole job of an arm is one question — *can I remove, from the current
//! state, the thing the invariant complains about?* If yes, `Some(op)`; if no,
//! `None`. What the engine then does with `None` (convict the failing target,
//! or panic when a fix op produced the invariant) is the engine's business, and
//! no arm reasons about it.
//!
//! Five rules govern every arm below.
//!
//! 1. **Presence, never predicate.** An arm asks whether the material it would
//!    remove is *there*; it never re-evaluates the invariant's own condition,
//!    which may depend on the failing op's payload. So
//!    [Convergence::InterrogationGroupOutOfBounds] asks "is that group still in
//!    that cell?", never "is it out of bounds?" — after a group-list shrink is
//!    itself repaired, the count read back from the state can be above the
//!    offending group number while the group still has to go.
//! 2. **No `expect` on a state lookup — a miss is `None`.** The invariant set
//!    the engine hands the map was computed on this state *plus the op that
//!    just failed*, and that op was rolled back. A row named by a site may
//!    therefore not exist here at all. Every arm is a chain of lookups where
//!    any miss short-circuits. The only `expect`s in this file are on **sealed
//!    constructor** rebuilds, where the failure is provably impossible from the
//!    value alone.
//! 3. **The state is always valid at fix time**, so the ids a fix op names are
//!    alive and its prechecks pass. Every op that ever landed — target or fix —
//!    passed the full gate, and the entry document was validated on decode.
//!    This is what makes the row-clearing fixes legal even though the target of
//!    a dangling reference is "gone": it is *not* gone here. When
//!    `PeriodOp::RemoveWithWeeks(P)` fails, the data was rolled back and `P` is still in
//!    the table, so `SetRow(P, subject, ∅)` — whose precheck demands the period
//!    exist — applies cleanly. The hole appears only once the retried target
//!    finally lands, by which time every row that would have dangled is gone.
//! 4. **The presence test names the target**, not merely "some value is there".
//!    Wherever the offending reference sits in a field that could legally hold
//!    a *different*, live id, the arm compares against the target before
//!    acting; otherwise it destroys a perfectly valid reference. The criterion:
//!    an arm needs an explicit identity test **exactly when the target id does
//!    not appear in the op it emits**. `SetRow`, `SetSubject`, `SetStudent`,
//!    `SetInterrogation`, `SetGroupList` and `AssignToSubject` carry the target
//!    inside the op *when the target is the coordinate the op names* — and only
//!    then: a wrong target is then not expressible and a plain lookup is the
//!    whole test. `Remove(row)` and `Update(row, rebuilt)` name only the row,
//!    and the identity test is the only thing that ties them to the target.
//!    Element-removal rebuilds satisfy the criterion for free — the membership
//!    test *is* the identity test. Two arms emit one of the carrying ops on a
//!    target it does *not* name, and each carries its own test accordingly:
//!    [GroupListRefSite::AssociationEntry] (target = the assigned group list,
//!    op = `AssignToSubject(period, subject, None)`, which names the entry key
//!    only) has the explicit `*assigned != group_list` test, and
//!    [StudentRefSite::ColloscopeGroupListStudent] (target = the student, op =
//!    `SetGroupList(list, rebuilt)`, which names the list only) is the
//!    element-removal case, tested by its `contains_key(&student)` membership
//!    check. Read the criterion op-*instance*-wise, never op-*variant*-wise:
//!    taken as a list of variant names it would wave those two arms through.
//! 5. **Pin the shape you are about to change, not merely its existence.** An
//!    invariant names an offending *configuration*: a row together with the
//!    field values that make it offending. Since the failing op was rolled back
//!    before the map runs, testing only "the row is there" can find that row
//!    innocent and repair it anyway. The test pins only the fields the fix is
//!    about to destroy, never the whole predicate — pinning a field the
//!    legitimate cascade route is expected to have changed would reject that
//!    route. [Convergence::SlotOverflowsDay] is the case to remember: the arm
//!    tests `start` and deliberately does *not* test `duration`, because on the
//!    legitimate route (the subject's interrogation is lengthened) the live
//!    subject still holds the old duration while the live slot still holds the
//!    offending start.
//!
//! Some arms below cannot fire on today's code — their `Some` branch is
//! unreachable because a guard in another file forbids the route. They carry
//! their identity and shape tests anyway: those guards are not obliged to stay,
//! `Op::GlobalUpdate` can carry states nobody foresaw, and a test that is one
//! comparison cannot be wrong. Uniformity is also what makes rule 4's criterion
//! checkable by shape rather than by argument.
//!
//! The engine's no-op-fix panic is **not** a safety net for any of this: a fix
//! that lands as a perfect no-op is a crash in front of the user, not a repair.
//! Correctness lives in these arms.

#[cfg(test)]
mod innocent_tests;

use std::collections::{BTreeMap, BTreeSet};

use collomatique_state::Fixable;

use crate::Data;
use crate::group_lists::{GroupList, GroupListFilling};
use crate::ids::{
    GroupListId, PeriodId, SlotId, StudentId, SubjectId, TeacherId, WeekId, WeekPatternId,
};
use crate::invariants::{Convergence, FixableInvariant};
use crate::ops::{
    AnnotatedAssignmentOp, AnnotatedBalancingOp, AnnotatedColloscopeOp, AnnotatedGroupListOp,
    AnnotatedIncompatOp, AnnotatedOp, AnnotatedPairingOp, AnnotatedSettingsOp, AnnotatedSlotOp,
    AnnotatedSlotPairingOp, AnnotatedStudentOp, AnnotatedSubjectOp, AnnotatedTeacherOp,
    AnnotatedWeekOp, AnnotatedWeekPatternOp,
};
use crate::pairings::PairingRule;
use crate::refs::{
    GroupListRefSite, PeriodRefSite, Reference, SlotRefSite, StudentRefSite, SubjectRefSite,
    TeacherRefSite, WeekPatternRefSite, WeekRefSite,
};
use crate::slot_pairings::SlotPairingRule;

impl Fixable for Data {
    fn fix_invariant(&self, invariant: &FixableInvariant) -> Option<AnnotatedOp> {
        match invariant {
            FixableInvariant::DanglingFk(reference) => self.fix_dangling(*reference),
            FixableInvariant::Convergence(convergence) => self.fix_convergence(convergence),
        }
    }
}

impl Data {
    /// Every op this map emits is deletive, and the deletive ops' annotated
    /// forms are payload-identical to their plain forms — so the arms construct
    /// [AnnotatedOp] values directly. No `annotate` call, no issued id (with
    /// only `&self` the map cannot reach the id issuer at all).
    fn fix_dangling(&self, reference: Reference) -> Option<AnnotatedOp> {
        match reference {
            Reference::Period { target, site } => self.fix_period_ref(target, site),
            Reference::Week { target, site } => self.fix_week_ref(target, site),
            Reference::Subject { target, site } => self.fix_subject_ref(target, site),
            Reference::Teacher { target, site } => self.fix_teacher_ref(target, site),
            Reference::Student { target, site } => self.fix_student_ref(target, site),
            Reference::WeekPattern { target, site } => self.fix_week_pattern_ref(target, site),
            Reference::Slot { target, site } => self.fix_slot_ref(target, site),
            Reference::GroupList { target, site } => self.fix_group_list_ref(target, site),
        }
    }

    /// A week belongs to its period and cannot survive without it; every other
    /// site holds the period in a set or in a row key, so only the reference
    /// goes.
    fn fix_period_ref(&self, period: PeriodId, site: PeriodRefSite) -> Option<AnnotatedOp> {
        let params = &self.inner_data.params;
        match site {
            PeriodRefSite::WeekPeriodFk(week_id) => {
                let week = params.weeks.find_week(week_id)?;
                if week.period_id != period {
                    return None;
                }
                Some(AnnotatedWeekOp::Remove(week_id).into())
            }
            PeriodRefSite::SubjectExcludedPeriods(subject_id) => {
                let subject = params.subjects.find_subject(subject_id)?;
                if !subject.excluded_periods.contains(&period) {
                    return None;
                }
                let mut rebuilt = subject.clone();
                rebuilt.excluded_periods.remove(&period);
                Some(AnnotatedSubjectOp::Update(subject_id, rebuilt).into())
            }
            PeriodRefSite::StudentExcludedPeriods(student_id) => {
                let student = params.students.student_map.get(&student_id)?;
                if !student.excluded_periods.contains(&period) {
                    return None;
                }
                let mut rebuilt = student.clone();
                rebuilt.excluded_periods.remove(&period);
                Some(AnnotatedStudentOp::Update(student_id, rebuilt).into())
            }
            PeriodRefSite::PairingRuleExcludedPeriods(rule_id) => {
                let rule = params.pairings.pairing_rule_map.get(&rule_id)?;
                if !rule.excluded_periods().contains(&period) {
                    return None;
                }
                // Sealed value: `into_parts` is the door for callers that
                // rebuild. The two parts are moved across untouched, and
                // `PairingRule::new`'s only failure is the two parts naming one
                // subject — so the `expect` is honest (rule 2's exception).
                let (antecedent, consequent, mut excluded_periods, soft) =
                    rule.clone().into_parts();
                excluded_periods.remove(&period);
                let rebuilt = PairingRule::new(antecedent, consequent, excluded_periods, soft)
                    .expect("removing an excluded period cannot make the parts share a subject");
                Some(AnnotatedPairingOp::Update(rule_id, rebuilt).into())
            }
            PeriodRefSite::SlotPairingRuleExcludedPeriods(rule_id) => {
                let rule = params.slot_pairings.slot_pairing_rule_map.get(&rule_id)?;
                if !rule.excluded_periods().contains(&period) {
                    return None;
                }
                let (antecedent, consequent, mut excluded_periods, soft) =
                    rule.clone().into_parts();
                excluded_periods.remove(&period);
                let rebuilt = SlotPairingRule::new(antecedent, consequent, excluded_periods, soft)
                    .expect("removing an excluded period cannot make the parts share a slot");
                Some(AnnotatedSlotPairingOp::Update(rule_id, rebuilt).into())
            }
            PeriodRefSite::AssignmentsKey { subject } => {
                if params.assignments.students(period, subject).is_none() {
                    return None;
                }
                // Canonical-absent: an emptied row is removed outright, so this
                // is always a real change.
                Some(AnnotatedAssignmentOp::SetRow(period, subject, BTreeSet::new()).into())
            }
            PeriodRefSite::AssociationEntry { subject } => {
                if !params
                    .group_lists
                    .subjects_associations
                    .contains(&(period, subject))
                {
                    return None;
                }
                // Unassigning leaves the group list itself in place, possibly
                // referenced by nothing — a legal state the invariants never
                // complain about. Removing it would be destruction the
                // invariant did not ask for.
                Some(AnnotatedGroupListOp::AssignToSubject(period, subject, None).into())
            }
        }
    }

    fn fix_week_ref(&self, week: WeekId, site: WeekRefSite) -> Option<AnnotatedOp> {
        let params = &self.inner_data.params;
        match site {
            WeekRefSite::WeekPatternExcludedWeek(pattern_id) => {
                let pattern = params.week_patterns.week_pattern_map.get(&pattern_id)?;
                if !pattern.excluded_weeks.contains(&week) {
                    return None;
                }
                let mut rebuilt = pattern.clone();
                rebuilt.excluded_weeks.remove(&week);
                Some(AnnotatedWeekPatternOp::Update(pattern_id, rebuilt).into())
            }
            WeekRefSite::ColloscopeInterrogation { slot } => {
                if self
                    .inner_data
                    .colloscope
                    .interrogation(slot, week)
                    .is_none()
                {
                    return None;
                }
                Some(AnnotatedColloscopeOp::SetInterrogation(slot, week, BTreeSet::new()).into())
            }
        }
    }

    /// `Slot::subject_id`, `Incompatibility::subject_id` and
    /// `RulePart::subject_id` are all bare mandatory fields: the reference
    /// cannot leave on its own, so the whole row goes. The two pairing-rule
    /// parts get **separate arms** even though both emit the same `Remove`,
    /// because a shared arm testing neither part would delete a rule whose two
    /// parts are both live.
    fn fix_subject_ref(&self, subject: SubjectId, site: SubjectRefSite) -> Option<AnnotatedOp> {
        let params = &self.inner_data.params;
        match site {
            SubjectRefSite::TeacherSubjects(teacher_id) => {
                let teacher = params.teachers.teacher_map.get(&teacher_id)?;
                if !teacher.subjects.contains(&subject) {
                    return None;
                }
                let mut rebuilt = teacher.clone();
                rebuilt.subjects.remove(&subject);
                Some(AnnotatedTeacherOp::Update(teacher_id, rebuilt).into())
            }
            SubjectRefSite::SlotSubject(slot_id) => {
                let slot = params.slots.find_slot(slot_id)?;
                if slot.subject_id != subject {
                    return None;
                }
                Some(AnnotatedSlotOp::Remove(slot_id).into())
            }
            SubjectRefSite::IncompatSubject(incompat_id) => {
                let incompat = params.incompats.incompat_map.get(&incompat_id)?;
                if incompat.subject_id != subject {
                    return None;
                }
                Some(AnnotatedIncompatOp::Remove(incompat_id).into())
            }
            SubjectRefSite::PairingRuleAntecedent(rule_id) => {
                let rule = params.pairings.pairing_rule_map.get(&rule_id)?;
                if rule.antecedent().subject_id != subject {
                    return None;
                }
                Some(AnnotatedPairingOp::Remove(rule_id).into())
            }
            SubjectRefSite::PairingRuleConsequent(rule_id) => {
                let rule = params.pairings.pairing_rule_map.get(&rule_id)?;
                if rule.consequent().subject_id != subject {
                    return None;
                }
                Some(AnnotatedPairingOp::Remove(rule_id).into())
            }
            SubjectRefSite::BalancingSubjectKey => {
                if !params.balancing.subjects.contains(&subject) {
                    return None;
                }
                // The subject falls back to the global balancing options.
                Some(AnnotatedBalancingOp::SetSubject(subject, None).into())
            }
            SubjectRefSite::AssignmentsKey { period } => {
                if params.assignments.students(period, subject).is_none() {
                    return None;
                }
                Some(AnnotatedAssignmentOp::SetRow(period, subject, BTreeSet::new()).into())
            }
            SubjectRefSite::AssociationEntry { period } => {
                if !params
                    .group_lists
                    .subjects_associations
                    .contains(&(period, subject))
                {
                    return None;
                }
                Some(AnnotatedGroupListOp::AssignToSubject(period, subject, None).into())
            }
        }
    }

    /// `Slot::teacher_id` is mandatory, so there is no teacher-less slot to
    /// fall back to, and naming a substitute teacher would be inventing data:
    /// the slot goes. The identity test is load-bearing here rather than
    /// defensive — a slot's teacher *is* freely editable, so `SlotOp::Update`
    /// naming a dead teacher lands, and without the test this arm would delete
    /// a slot whose live teacher is perfectly valid.
    fn fix_teacher_ref(&self, teacher: TeacherId, site: TeacherRefSite) -> Option<AnnotatedOp> {
        let params = &self.inner_data.params;
        match site {
            TeacherRefSite::SlotTeacher(slot_id) => {
                let slot = params.slots.find_slot(slot_id)?;
                if slot.teacher_id != teacher {
                    return None;
                }
                Some(AnnotatedSlotOp::Remove(slot_id).into())
            }
        }
    }

    fn fix_student_ref(&self, student: StudentId, site: StudentRefSite) -> Option<AnnotatedOp> {
        let params = &self.inner_data.params;
        match site {
            StudentRefSite::GroupListPrefilledStudent(group_list_id) => {
                let group_list = params.group_lists.group_list_map.get(&group_list_id)?;
                // `contains_student` is `false` for an automatic filling, so
                // this one test also short-circuits when the live list is of
                // the other kind (which is what a `GroupListOp::Update` that
                // flipped the variant leaves behind).
                if !group_list.filling().contains_student(student) {
                    return None;
                }
                let (list_params, mut filling) = group_list.clone().into_parts();
                filling.remove_student(student);
                let rebuilt = GroupList::new(list_params, filling).expect(
                    "removing a member changes neither the group count \
                     nor introduces a duplicate",
                );
                Some(AnnotatedGroupListOp::Update(group_list_id, rebuilt).into())
            }
            StudentRefSite::GroupListExcludedStudent(group_list_id) => {
                let group_list = params.group_lists.group_list_map.get(&group_list_id)?;
                // Matching the variant is the other half of the presence test:
                // a prefilled filling excludes nobody.
                let GroupListFilling::Automatic { excluded_students } = group_list.filling() else {
                    return None;
                };
                if !excluded_students.contains(&student) {
                    return None;
                }
                let mut excluded_students = excluded_students.clone();
                excluded_students.remove(&student);
                let rebuilt = GroupList::new(
                    group_list.params().clone(),
                    GroupListFilling::Automatic { excluded_students },
                )
                .expect("`GroupList::new` validates the prefilled branch only");
                Some(AnnotatedGroupListOp::Update(group_list_id, rebuilt).into())
            }
            StudentRefSite::SettingsStudentKey => {
                if !params.settings.students.contains(&student) {
                    return None;
                }
                // The student falls back to the global limits.
                Some(AnnotatedSettingsOp::SetStudent(student, None).into())
            }
            StudentRefSite::AssignmentsStudent { period, subject } => {
                let row = params.assignments.students(period, subject)?;
                if !row.contains(&student) {
                    return None;
                }
                let mut rebuilt = row.clone();
                rebuilt.remove(&student);
                Some(AnnotatedAssignmentOp::SetRow(period, subject, rebuilt).into())
            }
            StudentRefSite::ColloscopeGroupListStudent(group_list_id) => {
                // An absent row, and a row that does not place this student,
                // are both `None`: `SetGroupList` against a missing row would
                // be a perfect no-op. Where the row is there it is non-empty
                // (canonical-absent), so removing a placement — even the last
                // one, which clears the row — is always a real change.
                let placements = self.inner_data.colloscope.group_list(group_list_id)?;
                if !placements.contains_key(&student) {
                    return None;
                }
                let mut rebuilt = placements.clone();
                rebuilt.remove(&student);
                Some(AnnotatedColloscopeOp::SetGroupList(group_list_id, rebuilt).into())
            }
        }
    }

    /// Both sites hold the pattern in an `Option` whose `None` is a legal,
    /// documented value meaning "every week", so the reference can go alone and
    /// the row stays. This is the map's one deliberate divergence from the
    /// legacy cleaning, which deleted the slot and the incompatibility. No
    /// invariant can break either way: `None` deactivates nothing, so clearing
    /// it can only remove instances of `InterrogationOnInactiveWeek`, and no
    /// convergence predicate mentions an incompatibility at all.
    fn fix_week_pattern_ref(
        &self,
        week_pattern: WeekPatternId,
        site: WeekPatternRefSite,
    ) -> Option<AnnotatedOp> {
        let params = &self.inner_data.params;
        match site {
            WeekPatternRefSite::SlotWeekPattern(slot_id) => {
                let slot = params.slots.find_slot(slot_id)?;
                if slot.week_pattern != Some(week_pattern) {
                    return None;
                }
                let mut rebuilt = slot.clone();
                rebuilt.week_pattern = None;
                Some(AnnotatedSlotOp::Update(slot_id, rebuilt).into())
            }
            WeekPatternRefSite::IncompatWeekPattern(incompat_id) => {
                let incompat = params.incompats.incompat_map.get(&incompat_id)?;
                if incompat.week_pattern_id != Some(week_pattern) {
                    return None;
                }
                let mut rebuilt = incompat.clone();
                rebuilt.week_pattern_id = None;
                Some(AnnotatedIncompatOp::Update(incompat_id, rebuilt).into())
            }
        }
    }

    /// `SlotRulePart::slot_id` is a bare id, so a half-rule cannot exist and
    /// the rule goes. As for the pairing rules, the two parts get separate arms
    /// so that each tests its own part against the target.
    fn fix_slot_ref(&self, slot: SlotId, site: SlotRefSite) -> Option<AnnotatedOp> {
        let params = &self.inner_data.params;
        match site {
            SlotRefSite::SlotPairingRuleAntecedent(rule_id) => {
                let rule = params.slot_pairings.slot_pairing_rule_map.get(&rule_id)?;
                if rule.antecedent().slot_id != slot {
                    return None;
                }
                Some(AnnotatedSlotPairingOp::Remove(rule_id).into())
            }
            SlotRefSite::SlotPairingRuleConsequent(rule_id) => {
                let rule = params.slot_pairings.slot_pairing_rule_map.get(&rule_id)?;
                if rule.consequent().slot_id != slot {
                    return None;
                }
                Some(AnnotatedSlotPairingOp::Remove(rule_id).into())
            }
            SlotRefSite::ColloscopeInterrogation { week } => {
                // The slot is half the row key, so clearing is forced. A
                // present row is non-empty, so this is always a real change.
                if self
                    .inner_data
                    .colloscope
                    .interrogation(slot, week)
                    .is_none()
                {
                    return None;
                }
                Some(AnnotatedColloscopeOp::SetInterrogation(slot, week, BTreeSet::new()).into())
            }
        }
    }

    fn fix_group_list_ref(
        &self,
        group_list: GroupListId,
        site: GroupListRefSite,
    ) -> Option<AnnotatedOp> {
        let params = &self.inner_data.params;
        match site {
            GroupListRefSite::AssociationEntry { period, subject } => {
                // Here the reference is the entry's *value*, not part of its
                // key, so the identity test is what ties the fix to the target.
                let assigned = params
                    .group_lists
                    .subjects_associations
                    .get(&(period, subject))?;
                if *assigned != group_list {
                    return None;
                }
                Some(AnnotatedGroupListOp::AssignToSubject(period, subject, None).into())
            }
            GroupListRefSite::ColloscopeGroupListKey => {
                // The group list *is* the row key, so clearing is forced.
                if self.inner_data.colloscope.group_list(group_list).is_none() {
                    return None;
                }
                Some(AnnotatedColloscopeOp::SetGroupList(group_list, BTreeMap::new()).into())
            }
        }
    }

    /// The convergence fixes clear the now-invalid data — lossy by nature. The
    /// repairs going the other way are all excluded: granting a teacher a
    /// subject, enabling interrogations, shortening a duration or moving a
    /// slot's start each *invent* data, and none of them shrinks the document,
    /// which is what the cascade's termination proof rests on.
    fn fix_convergence(&self, convergence: &Convergence) -> Option<AnnotatedOp> {
        let params = &self.inner_data.params;
        let colloscope = &self.inner_data.colloscope;
        match convergence {
            Convergence::SlotTeacherDoesNotTeachSubject(slot_id, teacher, subject) => {
                // The teacher comparison is the load-bearing one: the reachable
                // route is `SlotOp::Update` rewriting the teacher to one who
                // does not teach the subject, over a state that was fine.
                let slot = params.slots.find_slot(*slot_id)?;
                if slot.teacher_id != *teacher || slot.subject_id != *subject {
                    return None;
                }
                Some(AnnotatedSlotOp::Remove(*slot_id).into())
            }
            Convergence::TeacherSubjectWithoutInterrogations(teacher_id, subject) => {
                // `Teacher::subjects` is a set: one element can leave and the
                // teacher stays valid, so only the element goes.
                let teacher = params.teachers.teacher_map.get(teacher_id)?;
                if !teacher.subjects.contains(subject) {
                    return None;
                }
                let mut rebuilt = teacher.clone();
                rebuilt.subjects.remove(subject);
                Some(AnnotatedTeacherOp::Update(*teacher_id, rebuilt).into())
            }
            Convergence::SlotForSubjectWithoutInterrogations(slot_id, subject) => {
                // This arm's `Some` branch is structurally shadowed: any state
                // where this fires also breaks something declared earlier
                // (`SlotTeacherDoesNotTeachSubject`,
                // `TeacherSubjectWithoutInterrogations`, or a `SlotTeacher`
                // dangle), and the engine picks only the canonical first. The
                // arm is written in full anyway — see the module docs.
                let slot = params.slots.find_slot(*slot_id)?;
                if slot.subject_id != *subject {
                    return None;
                }
                Some(AnnotatedSlotOp::Remove(*slot_id).into())
            }
            Convergence::SlotOverflowsDay {
                slot: slot_id,
                start,
                // Deliberately not tested: on the legitimate route the subject's
                // interrogation was lengthened, and the live subject still holds
                // the *old* duration (module docs, rule 5).
                duration: _,
            } => {
                let slot = params.slots.find_slot(*slot_id)?;
                if slot.start_time != *start {
                    return None;
                }
                Some(AnnotatedSlotOp::Remove(*slot_id).into())
            }
            Convergence::AssignmentForSubjectNotRunningOnPeriod(period, subject) => {
                // Coordinate-shaped: the invariant names a coordinate, the op
                // carries that same coordinate, and the fix removes the whole
                // thing at it — so the presence of the row *is* the offending
                // shape, and there is no field left to compare.
                if params.assignments.students(*period, *subject).is_none() {
                    return None;
                }
                Some(AnnotatedAssignmentOp::SetRow(*period, *subject, BTreeSet::new()).into())
            }
            Convergence::AssignedStudentNotPresentForPeriod {
                period,
                subject,
                student,
            } => {
                let row = params.assignments.students(*period, *subject)?;
                if !row.contains(student) {
                    return None;
                }
                let mut rebuilt = row.clone();
                rebuilt.remove(student);
                Some(AnnotatedAssignmentOp::SetRow(*period, *subject, rebuilt).into())
            }
            // Both name the same offending configuration — an association entry
            // at that coordinate — and both clear it. When they fire together
            // the canonical pick takes the first and the second goes with it.
            Convergence::AssociationForSubjectWithoutInterrogations(period, subject)
            | Convergence::AssociationForSubjectNotRunningOnPeriod(period, subject) => {
                if !params
                    .group_lists
                    .subjects_associations
                    .contains(&(*period, *subject))
                {
                    return None;
                }
                Some(AnnotatedGroupListOp::AssignToSubject(*period, *subject, None).into())
            }
            Convergence::BalancingForSubjectWithoutInterrogations(subject) => {
                if !params.balancing.subjects.contains(subject) {
                    return None;
                }
                Some(AnnotatedBalancingOp::SetSubject(*subject, None).into())
            }
            Convergence::PairedSlotsNotInSameSubject(rule_id, antecedent_slot, consequent_slot) => {
                // Which of the two slots is "wrong" is undecidable, and a rule
                // is sealed with two mandatory parts, so a part cannot leave
                // alone: the rule goes.
                let rule = params.slot_pairings.slot_pairing_rule_map.get(rule_id)?;
                if rule.antecedent().slot_id != *antecedent_slot
                    || rule.consequent().slot_id != *consequent_slot
                {
                    return None;
                }
                Some(AnnotatedSlotPairingOp::Remove(*rule_id).into())
            }
            // Same coordinate, same clearing op, same test (as above for the
            // two association variants).
            Convergence::InterrogationSlotNotRunningOnPeriod(slot, week)
            | Convergence::InterrogationOnInactiveWeek(slot, week) => {
                if colloscope.interrogation(*slot, *week).is_none() {
                    return None;
                }
                Some(AnnotatedColloscopeOp::SetInterrogation(*slot, *week, BTreeSet::new()).into())
            }
            Convergence::InterrogationGroupOutOfBounds(slot, week, group) => {
                // Presence, not predicate: the bound is never re-checked, since
                // a repaired group-list shrink legitimately needs this trim even
                // though the group reads as in-bounds again.
                let cell = colloscope.interrogation(*slot, *week)?;
                if !cell.contains(group) {
                    return None;
                }
                let mut rebuilt = cell.clone();
                rebuilt.remove(group);
                Some(AnnotatedColloscopeOp::SetInterrogation(*slot, *week, rebuilt).into())
            }
            Convergence::ColloscopeGroupListPrefilled(group_list) => {
                // Presence is the whole test, and prefilled-ness is
                // deliberately *not* read from the state: the offending
                // configuration has two routes, and on the one where the op
                // flips the list to prefilled, the pre-op row is a real,
                // innocent row this arm must clear. For a prefilled list there
                // is no single element to blame — the whole row is the
                // offending thing.
                if colloscope.group_list(*group_list).is_none() {
                    return None;
                }
                Some(AnnotatedColloscopeOp::SetGroupList(*group_list, BTreeMap::new()).into())
            }
            Convergence::ColloscopeStudentExcluded(group_list, student) => {
                // The filling's excluded set is likewise not read: adding a
                // student to it must clean the placement, while placing an
                // already-excluded student must be rejected. The presence test
                // gives both.
                let placements = colloscope.group_list(*group_list)?;
                if !placements.contains_key(student) {
                    return None;
                }
                let mut rebuilt = placements.clone();
                rebuilt.remove(student);
                Some(AnnotatedColloscopeOp::SetGroupList(*group_list, rebuilt).into())
            }
            Convergence::ColloscopeStudentGroupOutOfBounds(group_list, student, group) => {
                let placements = colloscope.group_list(*group_list)?;
                if placements.get(student) != Some(group) {
                    return None;
                }
                let mut rebuilt = placements.clone();
                rebuilt.remove(student);
                Some(AnnotatedColloscopeOp::SetGroupList(*group_list, rebuilt).into())
            }
        }
    }
}
