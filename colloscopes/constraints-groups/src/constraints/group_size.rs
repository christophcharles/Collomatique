//! Group sizes, pinned: per list and group, the number of students placed
//! there is *exactly* the balanced target of
//! [`balanced_targets`](crate::targets).
//!
//! One equality row replaces the min/max pair the model used to carry. The
//! sizes were never really free: the group count is exact
//! ([`VarEnv::group_count`]), the targets sum to the student count and stay
//! inside the spec's range, so the balanced shape was always among the
//! optimal ones. Fixing it is what the collision objective needs — a pair's
//! mass in a group is `1 / (size − 1)`, so a *variable* size would make the
//! objective non-linear.

use crate::extras::{MyBundle, V, base_var};
use crate::types::ConstraintDesc;
use crate::vars::{GroupListIdx, Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;

fn build_for_group(
    env: &VarEnv,
    bundle: MyBundle,
    list: GroupListIdx,
    group: u32,
    target: u32,
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

    bundle.with_constraint(
        count.eq(&IntLinExpr::constant(i64::from(target))),
        ConstraintDesc::GroupSize {
            list,
            group,
            target,
        },
    )
}

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for list in env.lists() {
        for (group, &target) in env.targets(list).iter().enumerate() {
            bundle = build_for_group(env, bundle, list, group as u32, target);
        }
    }
    bundle
}
