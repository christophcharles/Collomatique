//! Group-size constraints (roadmap §2.3): per list and group, the number of
//! students placed there is at least `min` when the group is non-empty, and
//! at most `max`.

use crate::extras::{MyBundle, V, extra_var};
use crate::types::{ConstraintDesc, ExtraVarName};
use crate::vars::{GroupListIdx, VarEnv};
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
            IntLinExpr::var(extra_var(ExtraVarName::StudentInGroup {
                list,
                student,
                group,
            }))
        })
        .sum();

    // The `GroupHasStudents` factor is what makes the minimum conditional:
    // an empty group has the indicator at 0, so the row degenerates to
    // `0 >= 0` instead of forbidding emptiness.
    let group_has = IntLinExpr::var(extra_var(ExtraVarName::GroupHasStudents { list, group }));
    let min_constraint = count.clone().geq(&(i64::from(min_students) * group_has));
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
        for group in 0..env.slot_count(list) {
            bundle = build_for_group(env, bundle, list, group, min_students, max_students);
        }
    }
    bundle
}
