use crate::ids::GroupNum;
use crate::native_extras::{MyBundle, V, extra_var, students_for_group_list};
use crate::types::{ConstraintDesc, ExtraVarName};
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::group_lists::GroupList;
use collomatique_state_colloscopes::ids::GroupListId;

fn build_for_group(
    env: &VarEnv,
    bundle: MyBundle,
    group_list: GroupListId,
    gl: &GroupList,
    group: GroupNum,
    min_students: u32,
    max_students: u32,
) -> MyBundle {
    let students = students_for_group_list(env, gl);

    let count: IntLinExpr<V> = students
        .iter()
        .map(|&student| {
            IntLinExpr::var(extra_var(ExtraVarName::StudentInGroup {
                student,
                group_list,
                group,
            }))
        })
        .sum();

    let group_has = IntLinExpr::var(extra_var(ExtraVarName::GroupHasStudents {
        group_list,
        group,
    }));
    let min_constraint = count.clone().geq(&(i64::from(min_students) * group_has));
    let bundle = bundle.with_constraint(
        min_constraint,
        ConstraintDesc::StudentsPerGroupMin {
            group_list,
            group,
            min_students,
        },
    );

    let max_constraint = count.leq(&IntLinExpr::constant(i64::from(max_students)));
    bundle.with_constraint(
        max_constraint,
        ConstraintDesc::StudentsPerGroupMax {
            group_list,
            group,
            max_students,
        },
    )
}

pub fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (&group_list, gl) in &env.group_lists.group_list_map {
        let min_students = gl.params.students_per_group.start().get();
        let max_students = gl.params.students_per_group.end().get();
        for group_index in 0..gl.params.group_names.len() {
            let group = GroupNum(group_index);
            bundle = build_for_group(
                env,
                bundle,
                group_list,
                gl,
                group,
                min_students,
                max_students,
            );
        }
    }
    bundle
}
