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

#[test]
fn builds_with_group_list_association() {
    let (data, _caveats) = deserialize_data(WITH_ASSOC).expect("fixture should decode");
    // Every period has a group-list association -> model builds. Passes today.
    let _ = build_model(&data.get_inner_data().params);
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
    let (data, _caveats) = deserialize_data(WITHOUT_ASSOC).expect("fixture should decode");
    let _ = build_model(&data.get_inner_data().params);
}
