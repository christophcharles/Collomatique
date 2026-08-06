//! `Σ_g StudentInGroup(list, student, g) == 1` — every student of a spec
//! sits in exactly one group of its list.
//!
//! This is what the integer `Var::StudentGroup` used to say through its
//! domain. The base variable is now the assignment matrix itself, so the
//! property has to be stated as a constraint. One plain equality row per
//! (list, student): user constraints are never reified, so there is no
//! helper column behind it.
//!
//! The template grouping gets the same family over its own matrix: it is a
//! partition of the students like any list, only one that never leaves the
//! model.

use crate::extras::{MyBundle, V, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for list in env.lists() {
        for &student in env.students(list) {
            let sum: IntLinExpr<V> = (0..env.group_count(list))
                .map(|group| {
                    IntLinExpr::var(base_var(Var::StudentInGroup {
                        list,
                        student,
                        group,
                    }))
                })
                .sum();
            bundle = bundle.with_constraint(
                sum.eq(&IntLinExpr::constant(1)),
                ConstraintDesc::StudentInOneGroup { list, student },
            );
        }
    }
    if let Some(ghost) = env.ghost() {
        for &student in ghost.spec().students() {
            let sum: IntLinExpr<V> = (0..env.ghost_group_count())
                .map(|group| IntLinExpr::var(base_var(Var::StudentInGhostGroup { student, group })))
                .sum();
            bundle = bundle.with_constraint(
                sum.eq(&IntLinExpr::constant(1)),
                ConstraintDesc::GhostStudentInOneGroup { student },
            );
        }
    }
    bundle
}
