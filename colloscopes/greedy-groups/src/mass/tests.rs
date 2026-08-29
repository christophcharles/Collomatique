use super::*;
use crate::specs::tests::{plan_with_uses, student};

/// The masses are rationals with small denominators, but they are reached by
/// different routes here and in [`pair_mass`], so every comparison goes
/// through a tolerance rather than `assert_eq!`.
fn assert_close(got: f64, expected: f64) {
    assert!(
        (got - expected).abs() < 1e-9,
        "expected {expected}, got {got}",
    );
}

/// `N_s` for one student, 0 for a student the plan does not know.
fn n_uses_of(plan: &GenerationPlan, student: StudentId) -> usize {
    plan_n_uses(plan).get(&student).copied().unwrap_or(0)
}

#[test]
fn a_mass_needs_a_partner_and_a_use() {
    // The plain case: two uses of a list, a student taking part in three uses
    // overall, three partners in the group.
    assert_close(pair_mass(2, 3, 4), 2.0 / 9.0);
    // Alone in the group: nobody to put mass on.
    assert_close(pair_mass(2, 3, 1), 0.0);
    // A student whose every list serves nothing: placed, but scoring nothing.
    // The formula would divide by zero here, which is the reason for the
    // guard rather than a mere shortcut.
    assert_close(pair_mass(0, 0, 4), 0.0);
    assert_close(pair_mass(3, 0, 4), 0.0);
}

#[test]
fn n_uses_counts_kept_lists_too() {
    // Student 1 is in the spec (2 uses) and in a kept list serving 3 pairs;
    // student 2's kept list is inert; students 5 and 6 are known through kept
    // lists only.
    let plan = plan_with_uses(
        &[(&[1, 2, 3, 4], (2, 2), 2)],
        &[(&[&[1, 5]], 3), (&[&[2, 6]], 0)],
    );

    assert_eq!(n_uses_of(&plan, student(1)), 5);
    assert_eq!(n_uses_of(&plan, student(2)), 2);
    assert_eq!(n_uses_of(&plan, student(5)), 3);
    // An inert kept list weighs nothing and does not even make its members
    // part of the universe.
    assert_eq!(n_uses_of(&plan, student(6)), 0);
    // And a student the plan never mentions.
    assert_eq!(n_uses_of(&plan, student(99)), 0);
}
