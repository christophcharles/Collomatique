//! Regression tests for `build_model` against `.collomatique` documents that
//! exercise slot-pairing rules over a period without a group-list association.
//!
//! Both fixtures derive from `examples/hogwarts.collomatique`. Subject 127
//! ("Potions - TP") has `groups_per_interrogation {min:4, max:4}` and owns the
//! null-week-pattern slots 156/157, so those slots are active on every
//! interrogation week. Each fixture removes the `(period 124, subject 127)`
//! `GroupListAssociations` row, leaving period 124's interrogation weeks
//! association-free for subject 127, and adds one slot-pairing rule over
//! 156/157. The state layer accepts both documents (they decode and pass all
//! invariants), so the model builder must handle both too.

use collomatique_constraints_colloscopes::{
    ConstraintDesc, ConstraintSource, ProgressiveConstraint, build_model,
};
use collomatique_storage::deserialize_data;

const SLOT_PAIRING_USED: &str =
    include_str!("fixtures/slot_pairing_over_period_without_association.collomatique");
const SLOT_PAIRING_NOT_USED: &str =
    include_str!("fixtures/slot_pairing_not_used_over_period_without_association.collomatique");

/// Defect A (panic): a `(should_have = true, false)` slot pairing over a subject
/// with `max_groups > 1` references the `InterrogationHasGroups` extra for every
/// common week (`pairings/slot.rs`, the `max_groups != 1` branch). That extra is
/// only declared where a group list is associated (`extras.rs`). For the removed
/// period, the extra is undeclared, so `build_model` must NOT panic.
///
/// FAILS today: `build_model` panics with
/// `UndeclaredExtra(InterrogationHasGroups { slot: SlotId(156), .. })`.
#[test]
fn slot_pairing_uses_group_extra_builds() {
    let (data, _caveats) = deserialize_data(SLOT_PAIRING_USED).expect("fixture should decode");
    let _ = build_model(&data.get_inner_data().params);
}

/// Defect B (soundness): a `(should_have = false, true)` slot pairing emits
/// `ant_count + con_count >= 1` for every common week
/// (`pairings/slot.rs`, the `(false, true)` branch). Over the association-free
/// period both group-count sums are empty, so the constraint degenerates to the
/// unsatisfiable `0 >= 1`, making the whole model infeasible.
///
/// The build itself succeeds (no `(true, false)` rule here), so we scan the
/// built model for a trivially-false `SlotPairingNotUsedImpliesUsed`
/// constraint instead of solving. `Constraint::trivially_eval()` returns
/// `Some(false)` for a variable-free unsatisfiable constraint. The match is
/// targeted at the pairing descriptor so it never trips over the deliberately
/// infeasible reified extras elsewhere in the model.
///
/// FAILS today: exactly such a `0 >= 1` constraint is present.
#[test]
fn slot_pairing_not_used_has_no_infeasible_constraint() {
    let (data, _caveats) = deserialize_data(SLOT_PAIRING_NOT_USED).expect("fixture should decode");
    let model = build_model(&data.get_inner_data().params);

    for (constraint, source) in model.problem().get_constraints() {
        if let ConstraintSource::User(ConstraintDesc::Level3(
            ProgressiveConstraint::SlotPairingNotUsedImpliesUsed { .. },
        )) = source
        {
            assert_ne!(
                constraint.trivially_eval(),
                Some(false),
                "spurious 0 >= 1 SlotPairingNotUsedImpliesUsed constraint over an \
                 association-free period"
            );
        }
    }
}
