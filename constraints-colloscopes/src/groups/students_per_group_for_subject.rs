use crate::extras::{
    MyBundle, V, extra_var, students_for_subject_period_group_list, subject_interrogation_params,
};
use crate::ids::GroupNum;
use crate::types::{ExtraVarName, ProgressiveConstraint, QualityConstraint};
use crate::vars::VarEnv;
use collomatique_ilp::int_linexpr::{IntConstraint, IntLinExpr};
use collomatique_state_colloscopes::group_lists::GroupListFilling;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for ((period, subject), &group_list) in env.group_lists.subjects_associations.iter() {
        let Some(interrog_params) = subject_interrogation_params(env, subject) else {
            continue;
        };
        let min_students = interrog_params.students_per_group.start().get();
        let max_students = interrog_params.students_per_group.end().get();

        let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
            continue;
        };

        for group in GroupNum::enumerate(env, group_list) {
            bundle = add_reification(env, bundle, group_list, gl, group, subject, period);

            let students = students_for_subject_period_group_list(env, gl, subject, period);

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

            let group_has = IntLinExpr::var(extra_var(ExtraVarName::GroupHasStudentsForSubject {
                group_list,
                group,
                subject,
                period,
            }));
            let min_constraint = count.clone().geq(&(i64::from(min_students) * group_has));
            bundle = bundle.with_constraint(
                min_constraint,
                ProgressiveConstraint::StudentsPerGroupForSubjectMin {
                    group_list,
                    group,
                    subject,
                    period,
                    min_students,
                }
                .into(),
            );

            let max_constraint = count.leq(&IntLinExpr::constant(i64::from(max_students)));
            bundle = bundle.with_constraint(
                max_constraint,
                QualityConstraint::StudentsPerGroupForSubjectMax {
                    group_list,
                    group,
                    subject,
                    period,
                    max_students,
                }
                .into(),
            );
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
    let var = ExtraVarName::GroupHasStudentsForSubject {
        group_list,
        group,
        subject,
        period,
    };

    match &gl.filling {
        GroupListFilling::Prefilled { groups } => {
            let enrolled = env.assignments.students(period, subject);
            let has_students = groups.get(group.index()).is_some_and(|g| {
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
                    .and_reified(var, move || vec![IntConstraint::infeasible()])
                    .expect("no duplicate extras")
            }
        }
        GroupListFilling::Automatic { .. } => {
            let students = students_for_subject_period_group_list(env, gl, subject, period);
            bundle
                .and_reified(var, move || {
                    let sum: IntLinExpr<V> = students
                        .iter()
                        .map(|&student| {
                            IntLinExpr::var(extra_var(ExtraVarName::StudentInGroup {
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
