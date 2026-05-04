use crate::native_extras::{MyBundle, V, extra_var, groups_for_group_list};
use crate::types::{ConstraintDesc, ExtraVarName};
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::group_lists::GroupListFilling;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (&group_list, gl) in &env.group_lists.group_list_map {
        let GroupListFilling::Automatic { .. } = &gl.filling else {
            continue;
        };
        let groups = groups_for_group_list(gl);
        if groups.len() < 2 {
            continue;
        }
        for &group in &groups[..groups.len() - 1] {
            let current = IntLinExpr::<V>::var(extra_var(ExtraVarName::GroupHasStudents {
                group_list,
                group,
            }));
            let next = IntLinExpr::<V>::var(extra_var(ExtraVarName::GroupHasStudents {
                group_list,
                group: group.next(),
            }));
            bundle = bundle.with_constraint(
                current.geq(&next),
                ConstraintDesc::GroupFilledByAscendingOrder { group_list, group },
            );
        }
    }
    bundle
}
