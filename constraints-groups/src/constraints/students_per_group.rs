//! Group-size constraints (roadmap §2.3): per list and group, the number of
//! students placed there is at least `min` and at most `max`.
//!
//! The minimum is unconditional. The group count is exact
//! ([`VarEnv::group_count`]), so an empty group would leave the others
//! oversized: emptiness is not something the model must tolerate any more.

use crate::extras::{MyBundle, V, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{GroupListIdx, Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;

fn build_for_group(
    env: &VarEnv,
    bundle: MyBundle,
    list: GroupListIdx,
    group: u32,
    min_students: u32,
    max_students: u32,
) -> MyBundle {
    let count: IntLinExpr<V> = env
        .students(list)
        .iter()
        .map(|&student| {
            IntLinExpr::var(base_var(Var::StudentInGroup {
                list,
                student,
                group,
            }))
        })
        .sum();

    let min_constraint = count
        .clone()
        .geq(&IntLinExpr::constant(i64::from(min_students)));
    let bundle = bundle.with_constraint(
        min_constraint,
        ConstraintDesc::StudentsPerGroupMin {
            list,
            group,
            min_students,
        },
    );

    let max_constraint = count.leq(&IntLinExpr::constant(i64::from(max_students)));
    bundle.with_constraint(
        max_constraint,
        ConstraintDesc::StudentsPerGroupMax {
            list,
            group,
            max_students,
        },
    )
}

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for list in env.lists() {
        let min_students = env.min_size(list);
        let max_students = env.max_size(list);
        for group in 0..env.group_count(list) {
            bundle = build_for_group(env, bundle, list, group, min_students, max_students);
        }
    }
    bundle
}
