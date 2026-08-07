//! Regression tests for `build_model` against real `.collomatique` documents.
//!
//! Both fixtures are the same colloscope; they differ only in whether the last
//! period carries its group-list associations. The state layer accepts both (they
//! decode and pass all invariants), so the model builder must handle both too.

use collomatique_constraints_colloscopes::build_model;
use collomatique_storage::deserialize_data;

const WITH_ASSOC: &str = include_str!("fixtures/period_with_group_list_association.collomatique");
const WITHOUT_ASSOC: &str =
    include_str!("fixtures/period_without_group_list_association.collomatique");
const EXCLUDED_STUDENT: &str =
    include_str!("fixtures/excluded_student_in_automatic_group_list.collomatique");

#[test]
fn builds_with_group_list_association() {
    let (inner, _caveats) = deserialize_data(WITH_ASSOC).expect("fixture should decode");
    // Every period has a group-list association -> model builds. Passes today.
    let _ = build_model(&inner.params);
}

#[test]
fn builds_without_group_list_association() {
    // Regression: the last period has interrogated subjects (e.g. Divination slot 169,
    // cost 50) but no GroupListAssociations. `build_model` must NOT panic.
    //
    // FAILS today: misc/interrogation_cost.rs references the `InterrogationHasGroups`
    // extra for every cost-bearing slot/week without the
    // `groups_for_interrogation(..).is_empty()` guard that gates its declaration in
    // extras.rs, so the build panics with
    // `UndeclaredExtra(InterrogationHasGroups { slot: SlotId(169), week: GlobalWeek(24) })`.
    let (inner, _caveats) = deserialize_data(WITHOUT_ASSOC).expect("fixture should decode");
    let _ = build_model(&inner.params);
}

#[test]
fn builds_with_a_student_excluded_from_the_automatic_group_list() {
    // Regression: student 7 is assigned to subject 13 for period 0, and the group
    // list associated to that (period, subject) is group list 46, which is
    // `Automatic { excluded_students: [7, 11] }`. The state layer accepts this
    // (the document comes from a fuzz walk and passes `broken_invariants`), so
    // the builder must handle it too.
    //
    // FAILS today: `build_student_at_interrogation_in_group` (extras.rs) declares
    // the per-group variables only for `students_for_group_list`, which drops the
    // excluded students; `build_student_at_interrogation` then sums over those
    // variables for every *enrolled* student, without the exclusion check. So the
    // build panics with
    // `ExtraError(StudentAtInterrogation { student: 7, slot: 19, week: 0 },
    //  UndeclaredVariable(Extra(StudentAtInterrogationInGroup { .., group_list: 46, .. })))`
    // — the slot named is just the first one of subject 13 the builder reaches.
    // See docs/todos/fixme_excluded_student_extra.md.
    let (inner, _caveats) = deserialize_data(EXCLUDED_STUDENT).expect("fixture should decode");
    let _ = build_model(&inner.params);
}
