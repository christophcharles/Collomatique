use crate::extras::{MyBundle, V, base_var};
use crate::types::StructuralConstraint;
use crate::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::group_lists::GroupListFilling;
pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (group_list, gl) in env.group_lists.group_list_map.entries() {
        let GroupListFilling::Automatic { excluded_students } = &gl.filling else {
            continue;
        };
        for &student in env.students.student_map.keys() {
            if excluded_students.contains(&student) {
                continue;
            }
            let expr = IntLinExpr::<V>::var(base_var(Var::StudentGroup {
                group_list,
                student,
            }));
            let constraint = expr.geq(&IntLinExpr::constant(0));
            bundle = bundle.with_constraint(
                constraint,
                StructuralConstraint::StudentHasGroup {
                    student,
                    group_list,
                }
                .into(),
            );
        }
    }
    bundle
}
