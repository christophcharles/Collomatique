//! Attribution pins: which [Fix] does each invariant get repaired with?
//!
//! `innocent_tests.rs` asks the negative question — does an arm keep its hands
//! off a document that does not carry the offending shape? This module asks the
//! positive one, and pins the whole answer *by value*: state and invariant in,
//! the exact [Fix] out, rebuilt payload included, plus the op it translates to.
//!
//! **Why a live document is the right input.** The map runs on the state the
//! gate rolled back, so the material an invariant names is still fully alive
//! when an arm is asked about it (module docs, rule 3): when
//! `PeriodOp::Remove(P)` fails, `P` and its weeks are all still there,
//! which is what makes the repair expressible at all. Handing a valid document
//! an invariant that names live material is therefore the engine's own calling
//! convention, not a contrivance.
//!
//! **One test per [Fix] variant**, not one per invariant. The vocabulary's
//! granularity is one variant per *rendered meaning*, so invariants
//! deliberately collapse onto a shared variant — a slot whose subject died and
//! a slot whose teacher died both answer [Fix::DeleteSlot]. Each test below
//! asserts every invariant that is supposed to collapse onto its variant, which
//! is what turns the collapse from a comment into a pinned claim. The converse
//! is pinned too: [Fix::DeleteSlot] and [Fix::DeleteOverflowingSlot] translate
//! to the very same op, and only the variant tells them apart.
//!
//! Expected payloads are built with the fixture's **own builders**, never read
//! back out of the document: a rebuild that dropped a field would otherwise
//! agree with itself.

use std::collections::{BTreeMap, BTreeSet};

use collomatique_state::{FixOp, Fixable, InMemoryData};

use super::Fix;
use super::innocent_tests::{
    automatic_group_list, build_valid_document, dress_subject_in_pattern, make_slot, pairing_rule,
    plain_student, plain_subject, prefilled_group_list, slot_pairing_rule,
};
use crate::Data;
use crate::incompats::Incompatibility;
use crate::invariants::{Convergence, FixableInvariant};
use crate::ops::{
    AnnotatedAssignmentOp, AnnotatedBalancingOp, AnnotatedColloscopeOp, AnnotatedGroupListOp,
    AnnotatedIncompatOp, AnnotatedOp, AnnotatedPairingOp, AnnotatedSettingsOp, AnnotatedSlotOp,
    AnnotatedSlotPairingOp, AnnotatedStudentOp, AnnotatedSubjectOp, AnnotatedTeacherOp,
    AnnotatedWeekOp, AnnotatedWeekPatternOp, ColloscopeOp, Op,
};
use crate::refs::{
    GroupListRefSite, PeriodRefSite, Reference, SlotRefSite, StudentRefSite, SubjectRefSite,
    TeacherRefSite, WeekPatternRefSite, WeekRefSite,
};
use crate::teachers::Teacher;
use crate::week_patterns::WeekPattern;

fn dangling(reference: Reference) -> FixableInvariant {
    FixableInvariant::DanglingFk(reference)
}

fn convergence(convergence: Convergence) -> FixableInvariant {
    FixableInvariant::Convergence(convergence)
}

/// One attribution pin: on `data`, `invariant` is repaired by `expected`, and
/// `expected` performs that repair through `op`.
///
/// The two halves are asserted together because they are one claim split by
/// [FixOp]: the arm chooses the meaning, the translation turns it into the
/// change. The op assert comes second so that a variant which kept its name and
/// lost its payload still fails here.
#[track_caller]
fn assert_fix(data: &Data, invariant: FixableInvariant, expected: Fix, op: AnnotatedOp) {
    assert_eq!(
        data.fix_invariant(&invariant),
        Some(expected.clone()),
        "wrong repair for {invariant:?}",
    );
    assert_eq!(
        expected.to_annotated_op(),
        op,
        "wrong op for the repair of {invariant:?}",
    );
}

/// [assert_fix] for a variant several invariants share: every one of them must
/// answer the same fix, which is what sharing a variant *means*.
#[track_caller]
fn assert_shared_fix(
    data: &Data,
    invariants: impl IntoIterator<Item = FixableInvariant>,
    expected: Fix,
    op: AnnotatedOp,
) {
    for invariant in invariants {
        assert_fix(data, invariant, expected.clone(), op.clone());
    }
}

#[test]
fn a_week_of_a_dying_period_answers_delete_week() {
    let (data, doc) = build_valid_document();

    assert_fix(
        &data,
        dangling(Reference::Period {
            target: doc.period,
            site: PeriodRefSite::WeekPeriodFk(doc.week),
        }),
        Fix::DeleteWeek { week: doc.week },
        AnnotatedWeekOp::Remove(doc.week).into(),
    );
}

#[test]
fn a_subject_excluding_a_dying_period_answers_remove_subject_period_exclusion() {
    let (data, doc) = build_valid_document();
    // `Sport` excludes `other_period` and nothing else, so the rebuild empties
    // the set.
    let rebuilt = plain_subject("Sport", BTreeSet::new());

    assert_fix(
        &data,
        dangling(Reference::Period {
            target: doc.other_period,
            site: PeriodRefSite::SubjectExcludedPeriods(doc.excluded_subject),
        }),
        Fix::RemoveSubjectPeriodExclusion {
            subject: doc.excluded_subject,
            period: doc.other_period,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedSubjectOp::Update(doc.excluded_subject, rebuilt).into(),
    );
}

#[test]
fn a_student_excluding_a_dying_period_answers_remove_student_period_exclusion() {
    let (data, doc) = build_valid_document();
    let rebuilt = plain_student(BTreeSet::new());

    assert_fix(
        &data,
        dangling(Reference::Period {
            target: doc.other_period,
            site: PeriodRefSite::StudentExcludedPeriods(doc.excluded_student),
        }),
        Fix::RemoveStudentPeriodExclusion {
            student: doc.excluded_student,
            period: doc.other_period,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedStudentOp::Update(doc.excluded_student, rebuilt).into(),
    );
}

#[test]
fn a_pairing_rule_excluding_a_dying_period_answers_remove_pairing_rule_period_exclusion() {
    let (data, doc) = build_valid_document();
    // The two parts travel across the sealed rebuild untouched: same subjects,
    // same softness, one exclusion fewer.
    let rebuilt = pairing_rule(doc.subject, doc.other_subject, BTreeSet::new());

    assert_fix(
        &data,
        dangling(Reference::Period {
            target: doc.other_period,
            site: PeriodRefSite::PairingRuleExcludedPeriods(doc.pairing),
        }),
        Fix::RemovePairingRulePeriodExclusion {
            rule: doc.pairing,
            period: doc.other_period,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedPairingOp::Update(doc.pairing, rebuilt).into(),
    );
}

#[test]
fn a_slot_pairing_rule_excluding_a_dying_period_answers_remove_slot_pairing_rule_period_exclusion()
{
    let (data, doc) = build_valid_document();
    let rebuilt = slot_pairing_rule(doc.slot, doc.other_slot, BTreeSet::new());

    assert_fix(
        &data,
        dangling(Reference::Period {
            target: doc.other_period,
            site: PeriodRefSite::SlotPairingRuleExcludedPeriods(doc.slot_pairing),
        }),
        Fix::RemoveSlotPairingRulePeriodExclusion {
            rule: doc.slot_pairing,
            period: doc.other_period,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedSlotPairingOp::Update(doc.slot_pairing, rebuilt).into(),
    );
}

#[test]
fn every_doomed_assignments_row_answers_clear_assignment_row() {
    let (data, doc) = build_valid_document();

    assert_shared_fix(
        &data,
        [
            // The period dies…
            dangling(Reference::Period {
                target: doc.period,
                site: PeriodRefSite::AssignmentsKey {
                    subject: doc.subject,
                },
            }),
            // …the subject dies…
            dangling(Reference::Subject {
                target: doc.subject,
                site: SubjectRefSite::AssignmentsKey { period: doc.period },
            }),
            // …or the subject stops running on the period. One sentence.
            convergence(Convergence::AssignmentForSubjectNotRunningOnPeriod(
                doc.period,
                doc.subject,
            )),
        ],
        Fix::ClearAssignmentRow {
            period: doc.period,
            subject: doc.subject,
        },
        AnnotatedAssignmentOp::SetRow(doc.period, doc.subject, BTreeSet::new()).into(),
    );
}

#[test]
fn every_doomed_association_answers_unassign_group_list() {
    let (data, doc) = build_valid_document();

    assert_shared_fix(
        &data,
        [
            dangling(Reference::Period {
                target: doc.period,
                site: PeriodRefSite::AssociationEntry {
                    subject: doc.subject,
                },
            }),
            dangling(Reference::Subject {
                target: doc.subject,
                site: SubjectRefSite::AssociationEntry { period: doc.period },
            }),
            // The group list is the entry's *value*, and the fix names only the
            // key — the arm's identity test is what ties them together.
            dangling(Reference::GroupList {
                target: doc.group_list,
                site: GroupListRefSite::AssociationEntry {
                    period: doc.period,
                    subject: doc.subject,
                },
            }),
            convergence(Convergence::AssociationForSubjectWithoutInterrogations(
                doc.period,
                doc.subject,
            )),
            convergence(Convergence::AssociationForSubjectNotRunningOnPeriod(
                doc.period,
                doc.subject,
            )),
        ],
        Fix::UnassignGroupList {
            period: doc.period,
            subject: doc.subject,
        },
        AnnotatedGroupListOp::AssignToSubject(doc.period, doc.subject, None).into(),
    );
}

#[test]
fn a_week_pattern_excluding_a_dying_week_answers_remove_week_pattern_exclusion() {
    let (data, doc) = build_valid_document();
    let rebuilt = WeekPattern {
        name: "sauf la deuxième semaine".into(),
        excluded_weeks: BTreeSet::new(),
    };

    assert_fix(
        &data,
        dangling(Reference::Week {
            target: doc.other_week,
            site: WeekRefSite::WeekPatternExcludedWeek(doc.week_pattern),
        }),
        Fix::RemoveWeekPatternExclusion {
            pattern: doc.week_pattern,
            week: doc.other_week,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedWeekPatternOp::Update(doc.week_pattern, rebuilt).into(),
    );
}

#[test]
fn every_doomed_interrogation_cell_answers_clear_interrogation_cell() {
    let (data, doc) = build_valid_document();

    assert_shared_fix(
        &data,
        [
            dangling(Reference::Week {
                target: doc.week,
                site: WeekRefSite::ColloscopeInterrogation { slot: doc.slot },
            }),
            dangling(Reference::Slot {
                target: doc.slot,
                site: SlotRefSite::ColloscopeInterrogation { week: doc.week },
            }),
            convergence(Convergence::InterrogationSlotNotRunningOnPeriod(
                doc.slot, doc.week,
            )),
            convergence(Convergence::InterrogationOnInactiveWeek(doc.slot, doc.week)),
        ],
        Fix::ClearInterrogationCell {
            slot: doc.slot,
            week: doc.week,
        },
        AnnotatedColloscopeOp::SetInterrogation(doc.slot, doc.week, BTreeSet::new()).into(),
    );
}

#[test]
fn a_subject_a_teacher_can_no_longer_teach_answers_remove_teacher_subject() {
    let (data, doc) = build_valid_document();
    // `teacher` teaches `subject` and nothing else.
    let rebuilt = Teacher {
        desc: Default::default(),
        subjects: BTreeSet::new(),
    };

    assert_shared_fix(
        &data,
        [
            dangling(Reference::Subject {
                target: doc.subject,
                site: SubjectRefSite::TeacherSubjects(doc.teacher),
            }),
            convergence(Convergence::TeacherSubjectWithoutInterrogations(
                doc.teacher,
                doc.subject,
            )),
        ],
        Fix::RemoveTeacherSubject {
            teacher: doc.teacher,
            subject: doc.subject,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedTeacherOp::Update(doc.teacher, rebuilt).into(),
    );
}

#[test]
fn every_slot_that_cannot_stand_answers_delete_slot() {
    let (data, doc) = build_valid_document();

    assert_shared_fix(
        &data,
        [
            dangling(Reference::Subject {
                target: doc.subject,
                site: SubjectRefSite::SlotSubject(doc.slot),
            }),
            dangling(Reference::Teacher {
                target: doc.teacher,
                site: TeacherRefSite::SlotTeacher(doc.slot),
            }),
            convergence(Convergence::SlotTeacherDoesNotTeachSubject(
                doc.slot,
                doc.teacher,
                doc.subject,
            )),
            convergence(Convergence::SlotForSubjectWithoutInterrogations(
                doc.slot,
                doc.subject,
            )),
        ],
        Fix::DeleteSlot { slot: doc.slot },
        AnnotatedSlotOp::Remove(doc.slot).into(),
    );
}

#[test]
fn a_slot_running_past_midnight_answers_delete_overflowing_slot() {
    let (data, doc) = build_valid_document();
    // The arm compares the invariant's `start` against the live slot's and
    // deliberately ignores `duration` — so the start has to be the real one,
    // and the duration can be anything.
    let start = make_slot(doc.subject, doc.teacher, Some(doc.week_pattern), 8).start_time;

    assert_fix(
        &data,
        convergence(Convergence::SlotOverflowsDay {
            slot: doc.slot,
            start,
            duration: collomatique_time::NonZeroMinutes::new(60).unwrap(),
        }),
        // The same op as [Fix::DeleteSlot] above, and a different sentence:
        // this is the slot that would run into the next day.
        Fix::DeleteOverflowingSlot { slot: doc.slot },
        AnnotatedSlotOp::Remove(doc.slot).into(),
    );
}

#[test]
fn an_incompatibility_of_a_dying_subject_answers_delete_incompat() {
    let (data, doc) = build_valid_document();

    assert_fix(
        &data,
        dangling(Reference::Subject {
            target: doc.subject,
            site: SubjectRefSite::IncompatSubject(doc.incompat),
        }),
        Fix::DeleteIncompat {
            incompat: doc.incompat,
        },
        AnnotatedIncompatOp::Remove(doc.incompat).into(),
    );
}

#[test]
fn either_half_of_a_pairing_rule_dying_or_losing_interrogations_answers_delete_pairing_rule() {
    let (data, doc) = build_valid_document();

    assert_shared_fix(
        &data,
        [
            dangling(Reference::Subject {
                target: doc.subject,
                site: SubjectRefSite::PairingRuleAntecedent(doc.pairing),
            }),
            dangling(Reference::Subject {
                target: doc.other_subject,
                site: SubjectRefSite::PairingRuleConsequent(doc.pairing),
            }),
            convergence(
                Convergence::PairingRuleAntecedentForSubjectWithoutInterrogations(
                    doc.pairing,
                    doc.subject,
                ),
            ),
            convergence(
                Convergence::PairingRuleConsequentForSubjectWithoutInterrogations(
                    doc.pairing,
                    doc.other_subject,
                ),
            ),
        ],
        Fix::DeletePairingRule { rule: doc.pairing },
        AnnotatedPairingOp::Remove(doc.pairing).into(),
    );
}

#[test]
fn a_doomed_balancing_override_answers_clear_subject_balancing() {
    let (data, doc) = build_valid_document();

    assert_shared_fix(
        &data,
        [
            dangling(Reference::Subject {
                target: doc.subject,
                site: SubjectRefSite::BalancingSubjectKey,
            }),
            convergence(Convergence::BalancingForSubjectWithoutInterrogations(
                doc.subject,
            )),
        ],
        Fix::ClearSubjectBalancing {
            subject: doc.subject,
        },
        AnnotatedBalancingOp::SetSubject(doc.subject, None).into(),
    );
}

#[test]
fn a_dying_student_in_a_prefilled_list_answers_remove_student_from_group_list_prefill() {
    let (data, doc) = build_valid_document();
    // The bystander keeps her seat in the first group; the empty second group
    // is untouched.
    let rebuilt = prefilled_group_list(
        "Prérempli",
        vec![BTreeSet::from([doc.other_student]), BTreeSet::new()],
    );

    assert_fix(
        &data,
        dangling(Reference::Student {
            target: doc.student,
            site: StudentRefSite::GroupListPrefilledStudent(doc.prefilled_group_list),
        }),
        Fix::RemoveStudentFromGroupListPrefill {
            group_list: doc.prefilled_group_list,
            student: doc.student,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedGroupListOp::Update(doc.prefilled_group_list, rebuilt).into(),
    );
}

#[test]
fn a_dying_student_a_list_excludes_answers_remove_student_group_list_exclusion() {
    let (data, doc) = build_valid_document();
    let rebuilt = automatic_group_list("Exclusions", 2, BTreeSet::new());

    assert_fix(
        &data,
        dangling(Reference::Student {
            target: doc.excluded_student,
            site: StudentRefSite::GroupListExcludedStudent(doc.excluding_group_list),
        }),
        Fix::RemoveStudentGroupListExclusion {
            group_list: doc.excluding_group_list,
            student: doc.excluded_student,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedGroupListOp::Update(doc.excluding_group_list, rebuilt).into(),
    );
}

#[test]
fn a_dying_students_own_limits_answer_clear_student_settings() {
    let (data, doc) = build_valid_document();

    assert_fix(
        &data,
        dangling(Reference::Student {
            target: doc.student,
            site: StudentRefSite::SettingsStudentKey,
        }),
        Fix::ClearStudentSettings {
            student: doc.student,
        },
        AnnotatedSettingsOp::SetStudent(doc.student, None).into(),
    );
}

#[test]
fn a_student_who_must_leave_a_row_answers_remove_student_from_assignment_row() {
    let (data, doc) = build_valid_document();
    // The row holds all three students; only the named one leaves.
    let rebuilt = BTreeSet::from([doc.other_student, doc.excluded_student]);

    assert_shared_fix(
        &data,
        [
            dangling(Reference::Student {
                target: doc.student,
                site: StudentRefSite::AssignmentsStudent {
                    period: doc.period,
                    subject: doc.subject,
                },
            }),
            convergence(Convergence::AssignedStudentNotPresentForPeriod {
                period: doc.period,
                subject: doc.subject,
                student: doc.student,
            }),
        ],
        Fix::RemoveStudentFromAssignmentRow {
            period: doc.period,
            subject: doc.subject,
            student: doc.student,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedAssignmentOp::SetRow(doc.period, doc.subject, rebuilt).into(),
    );
}

#[test]
fn a_student_who_must_leave_a_colloscope_row_answers_remove_student_colloscope_placement() {
    let (data, doc) = build_valid_document();
    // The bystander keeps group 1.
    let rebuilt = BTreeMap::from([(doc.other_student, 1)]);

    assert_shared_fix(
        &data,
        [
            dangling(Reference::Student {
                target: doc.student,
                site: StudentRefSite::ColloscopeGroupListStudent(doc.group_list),
            }),
            convergence(Convergence::ColloscopeStudentExcluded(
                doc.group_list,
                doc.student,
            )),
            // The group number is the one the student actually holds — the arm
            // compares it before removing the placement.
            convergence(Convergence::ColloscopeStudentGroupOutOfBounds(
                doc.group_list,
                doc.student,
                0,
            )),
        ],
        Fix::RemoveStudentColloscopePlacement {
            group_list: doc.group_list,
            student: doc.student,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedColloscopeOp::SetGroupList(doc.group_list, rebuilt).into(),
    );
}

#[test]
fn a_slot_wearing_a_dying_pattern_answers_clear_slot_week_pattern() {
    let (data, doc) = build_valid_document();
    // Everything else about the slot survives: same subject, same teacher, same
    // start. Only the pattern goes — which is what makes it run every week.
    let rebuilt = make_slot(doc.subject, doc.teacher, None, 8);

    assert_fix(
        &data,
        dangling(Reference::WeekPattern {
            target: doc.week_pattern,
            site: WeekPatternRefSite::SlotWeekPattern(doc.slot),
        }),
        Fix::ClearSlotWeekPattern {
            slot: doc.slot,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedSlotOp::Update(doc.slot, rebuilt).into(),
    );
}

#[test]
fn an_incompatibility_wearing_a_dying_pattern_answers_clear_incompat_week_pattern() {
    let (data, doc) = build_valid_document();
    let rebuilt = Incompatibility {
        subject_id: doc.subject,
        name: "Sport".into(),
        slots: vec![],
        minimum_free_slots: std::num::NonZeroU32::new(2).unwrap(),
        week_pattern_id: None,
    };

    assert_fix(
        &data,
        dangling(Reference::WeekPattern {
            target: doc.week_pattern,
            site: WeekPatternRefSite::IncompatWeekPattern(doc.incompat),
        }),
        Fix::ClearIncompatWeekPattern {
            incompat: doc.incompat,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedIncompatOp::Update(doc.incompat, rebuilt).into(),
    );
}

#[test]
fn a_subject_wearing_a_dying_pattern_answers_clear_subject_week_pattern() {
    let (mut data, doc) = build_valid_document();
    // No fixture subject wears a pattern, so `Sport` is dressed in the live one
    // first; the repair is exactly that field going back to `None`, its excluded
    // period untouched.
    dress_subject_in_pattern(&mut data, doc.excluded_subject, doc.week_pattern);
    let rebuilt = plain_subject("Sport", BTreeSet::from([doc.other_period]));

    assert_fix(
        &data,
        dangling(Reference::WeekPattern {
            target: doc.week_pattern,
            site: WeekPatternRefSite::SubjectWeekPattern(doc.excluded_subject),
        }),
        Fix::ClearSubjectWeekPattern {
            subject: doc.excluded_subject,
            rebuilt: rebuilt.clone(),
        },
        AnnotatedSubjectOp::Update(doc.excluded_subject, rebuilt).into(),
    );
}

#[test]
fn every_broken_slot_pairing_rule_answers_delete_slot_pairing_rule() {
    let (data, doc) = build_valid_document();

    assert_shared_fix(
        &data,
        [
            dangling(Reference::Slot {
                target: doc.slot,
                site: SlotRefSite::SlotPairingRuleAntecedent(doc.slot_pairing),
            }),
            dangling(Reference::Slot {
                target: doc.other_slot,
                site: SlotRefSite::SlotPairingRuleConsequent(doc.slot_pairing),
            }),
            convergence(Convergence::PairedSlotsNotInSameSubject(
                doc.slot_pairing,
                doc.slot,
                doc.other_slot,
            )),
        ],
        Fix::DeleteSlotPairingRule {
            rule: doc.slot_pairing,
        },
        AnnotatedSlotPairingOp::Remove(doc.slot_pairing).into(),
    );
}

#[test]
fn a_doomed_colloscope_group_list_row_answers_clear_colloscope_group_list_row() {
    let (data, doc) = build_valid_document();

    assert_shared_fix(
        &data,
        [
            dangling(Reference::GroupList {
                target: doc.group_list,
                site: GroupListRefSite::ColloscopeGroupListKey,
            }),
            convergence(Convergence::ColloscopeGroupListPrefilled(doc.group_list)),
        ],
        Fix::ClearColloscopeGroupListRow {
            group_list: doc.group_list,
        },
        AnnotatedColloscopeOp::SetGroupList(doc.group_list, BTreeMap::new()).into(),
    );
}

#[test]
fn out_of_bounds_groups_answer_remove_groups_from_interrogation_cell() {
    let (mut data, doc) = build_valid_document();
    // The fixture's cells hold a single group, which would make the rebuild an
    // empty set — indistinguishable from a plain clear. A second group is added
    // here so the pin shows what the rebuild *keeps*. Both numbers are in
    // bounds for the list associated at `(period, subject)`, so the document
    // stays valid.
    let (annotated, _) = data.annotate(Op::Colloscope(ColloscopeOp::SetInterrogation(
        doc.slot,
        doc.week,
        BTreeSet::from([0, 1]),
    )));
    data.apply(&annotated)
        .expect("two in-bounds groups in one cell is a valid document");
    let rebuilt = BTreeSet::from([1]);

    assert_fix(
        &data,
        convergence(Convergence::InterrogationGroupsOutOfBounds(
            doc.slot,
            doc.week,
            BTreeSet::from([0]),
        )),
        Fix::RemoveGroupsFromInterrogationCell {
            slot: doc.slot,
            week: doc.week,
            groups: BTreeSet::from([0]),
            rebuilt: rebuilt.clone(),
        },
        AnnotatedColloscopeOp::SetInterrogation(doc.slot, doc.week, rebuilt).into(),
    );
}
