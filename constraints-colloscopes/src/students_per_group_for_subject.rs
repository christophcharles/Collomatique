use crate::ids::GroupNum;
use crate::native_extras::{
    MyBundle, V, extra_var, students_for_subject_period_group_list, subject_interrogation_params,
};
use crate::types::{ConstraintDesc, ReifiedVarName};
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::group_lists::GroupListFilling;

pub fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (&period, subject_map) in &env.group_lists.subjects_associations {
        for (&subject, &group_list) in subject_map {
            let Some(interrog_params) = subject_interrogation_params(env, subject) else {
                continue;
            };
            let min_students = interrog_params.students_per_group.start().get() as i64;
            let max_students = interrog_params.students_per_group.end().get() as i64;

            let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
                continue;
            };

            for group_index in 0..gl.params.group_names.len() as u32 {
                let group = GroupNum(group_index);

                bundle = add_reification(env, bundle, group_list, gl, group, subject, period);

                let students =
                    students_for_subject_period_group_list(env, group_list, subject, period);

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

                let group_has =
                    IntLinExpr::var(extra_var(ReifiedVarName::GroupHasStudentsForSubject {
                        group_list,
                        group,
                        subject,
                        period,
                    }));
                let min_constraint = count.clone().geq(&(min_students * group_has));
                bundle = bundle.with_constraint(
                    min_constraint,
                    ConstraintDesc::StudentsPerGroupForSubjectMin {
                        group_list,
                        group,
                        subject,
                        period,
                        min_students: min_students as u32,
                    },
                );

                let max_constraint = count.leq(&IntLinExpr::constant(max_students));
                bundle = bundle.with_constraint(
                    max_constraint,
                    ConstraintDesc::StudentsPerGroupForSubjectMax {
                        group_list,
                        group,
                        subject,
                        period,
                        max_students: max_students as u32,
                    },
                );
            }
        }
    }
    bundle
}

fn add_reification(
    env: &VarEnv,
    bundle: MyBundle,
    group_list: collomatique_state_colloscopes::ids::GroupListId,
    gl: &collomatique_state_colloscopes::group_lists::GroupList,
    group: GroupNum,
    subject: collomatique_state_colloscopes::ids::SubjectId,
    period: collomatique_state_colloscopes::ids::PeriodId,
) -> MyBundle {
    let var = ReifiedVarName::GroupHasStudentsForSubject {
        group_list,
        group,
        subject,
        period,
    };

    match &gl.filling {
        GroupListFilling::Prefilled { groups } => {
            let enrolled = env
                .assignments
                .period_map
                .get(&period)
                .and_then(|pa| pa.subject_map.get(&subject));
            let has_students = groups.get(group.0 as usize).is_some_and(|g| {
                g.students
                    .iter()
                    .any(|s| enrolled.is_some_and(|e| e.contains(s)))
            });
            if has_students {
                bundle
                    .and_reified(var, move || {
                        vec![IntLinExpr::constant(0).leq(&IntLinExpr::constant(0))]
                    })
                    .expect("no duplicate extras")
            } else {
                bundle
                    .and_reified(var, move || {
                        vec![IntLinExpr::constant(1).leq(&IntLinExpr::constant(0))]
                    })
                    .expect("no duplicate extras")
            }
        }
        GroupListFilling::Automatic { .. } => {
            let students = students_for_subject_period_group_list(env, group_list, subject, period);
            bundle
                .and_reified(var, move || {
                    let sum: IntLinExpr<V> = students
                        .iter()
                        .map(|&student| {
                            IntLinExpr::var(extra_var(ReifiedVarName::StudentInGroup {
                                student,
                                group_list,
                                group,
                            }))
                        })
                        .sum();
                    vec![sum.geq(&IntLinExpr::constant(1))]
                })
                .expect("no duplicate extras")
        }
    }
}
