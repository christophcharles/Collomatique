use crate::ids::GroupNum;
use crate::native_extras::{
    MyBundle, V, all_group_lists, extra_var, groups_for_group_list, students_for_group_list,
};
use crate::types::{ConstraintDesc, ReifiedVarName};
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::ids::GroupListId;

fn build_for_group(
    env: &VarEnv,
    bundle: MyBundle,
    group_list: GroupListId,
    group: GroupNum,
    min_students: i64,
    max_students: i64,
) -> MyBundle {
    let students = students_for_group_list(env, group_list);

    let count: IntLinExpr<V> = students
        .iter()
        .map(|&student| {
            IntLinExpr::var(extra_var(ReifiedVarName::StudentInGroup {
                student,
                group_list,
                group,
            }))
        })
        .sum();

    let group_has = IntLinExpr::var(extra_var(ReifiedVarName::GroupHasStudents {
        group_list,
        group,
    }));
    let min_constraint = count.clone().geq(&(min_students * group_has));
    let bundle = bundle.with_constraint(
        min_constraint,
        ConstraintDesc::StudentsPerGroupMin {
            group_list,
            group,
            min_students: min_students as u32,
        },
    );

    let max_constraint = count.leq(&IntLinExpr::constant(max_students));
    bundle.with_constraint(
        max_constraint,
        ConstraintDesc::StudentsPerGroupMax {
            group_list,
            group,
            max_students: max_students as u32,
        },
    )
}

pub fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for &group_list in all_group_lists(env).iter() {
        let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
            continue;
        };
        let min_students = gl.params.students_per_group.start().get() as i64;
        let max_students = gl.params.students_per_group.end().get() as i64;
        let groups = groups_for_group_list(env, group_list);
        for &group in &groups {
            bundle = build_for_group(env, bundle, group_list, group, min_students, max_students);
        }
    }
    bundle
}
