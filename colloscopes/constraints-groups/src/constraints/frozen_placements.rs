//! `StudentInGroup(list, student, group) == 1` for each seat the caller asked
//! to hold fixed. Empty unless the generation was asked to keep the prefill.
//!
//! One plain equality row per seat, and not one per group of the list:
//! `student_in_one_group` already forces the student's other groups to 0 once
//! this one is 1.

use crate::extras::{MyBundle, V, base_var};
use crate::frozen::FrozenPlacements;
use crate::types::ConstraintDesc;
use crate::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;

pub(super) fn build(env: &VarEnv, frozen: &FrozenPlacements) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (list, student, group) in frozen.iter() {
        // A seat naming something this plan does not have is a caller bug:
        // the seats were read off a different plan than the one being built.
        // Refusing loudly beats pinning the wrong student — or, worse,
        // declaring a variable the rest of the model never mentions.
        assert!(
            env.lists().any(|l| l == list)
                && env.students(list).contains(&student)
                && group < env.group_count(list),
            "frozen placement {list:?}/{student:?}/{group} is not in this plan",
        );
        let seat: IntLinExpr<V> = IntLinExpr::var(base_var(Var::StudentInGroup {
            list,
            student,
            group,
        }));
        bundle = bundle.with_constraint(
            seat.eq(&IntLinExpr::constant(1)),
            ConstraintDesc::FrozenPlacement {
                list,
                student,
                group,
            },
        );
    }
    bundle
}

#[cfg(test)]
mod tests;
