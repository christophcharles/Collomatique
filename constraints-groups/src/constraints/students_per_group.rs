//! Group-size constraints (roadmap §2.3): per list and group, the number of
//! students placed there is at most `max`.

use crate::extras::{MyBundle, V, extra_var};
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{GroupListIdx, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;

fn build_for_group(
    env: &VarEnv,
    bundle: MyBundle,
    list: GroupListIdx,
    group: u32,
    max_students: u32,
) -> MyBundle {
    let count: IntLinExpr<V> = env
        .students(list)
        .iter()
        .map(|&student| {
            IntLinExpr::var(extra_var(ExtraVarName::StudentInGroup {
                list,
                student,
                group,
            }))
        })
        .sum();

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
        let max_students = env.max_size(list);
        for group in 0..env.slot_count(list) {
            bundle = build_for_group(env, bundle, list, group, max_students);
        }
    }
    bundle
}
