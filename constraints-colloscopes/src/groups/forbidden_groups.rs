use crate::extras::{
    MyBundle, V, base_var, group_list_for_interrogation, groups_for_group_list,
    students_for_subject_period_group_list, week_to_period_id, weeks_for_slot,
};
use crate::ids::GroupNum;
use crate::types::StructuralConstraint;
use crate::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::group_lists::GroupListFilling;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for subject_id in env.slots.subjects_with_slots() {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        for (slot_id, slot_data) in env
            .slots
            .slots_for_subject(subject_id)
            .into_iter()
            .flatten()
        {
            let slot = *slot_id;
            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                let Some(group_list) = group_list_for_interrogation(env, subject_id, week) else {
                    continue;
                };
                let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
                    continue;
                };
                let (period, _) = week_to_period_id(env, week).unwrap();

                match gl.filling() {
                    GroupListFilling::Prefilled { groups } => {
                        let enrolled = env.assignments.students(period, subject_id);

                        for (group_index, prefilled_group) in groups.iter().enumerate() {
                            let group = GroupNum::new(env, group_list, group_index)
                                .expect("valid group index");
                            let has_enrolled = prefilled_group
                                .students
                                .iter()
                                .any(|s| enrolled.is_some_and(|e| e.contains(s)));

                            if !has_enrolled {
                                let expr =
                                    IntLinExpr::<V>::var(base_var(Var::GroupInInterrogation {
                                        slot,
                                        week,
                                        group,
                                    }));
                                bundle = bundle.with_constraint(
                                    expr.eq(&IntLinExpr::constant(0)),
                                    StructuralConstraint::ForbiddenGroup {
                                        group_list,
                                        group,
                                        slot,
                                        week,
                                        subject: subject_id,
                                    }
                                    .into(),
                                );
                            }
                        }
                    }
                    GroupListFilling::Automatic { .. } => {
                        let students =
                            students_for_subject_period_group_list(env, gl, subject_id, period);

                        for group in groups_for_group_list(env, group_list) {
                            let sum: IntLinExpr<V> = students
                                .iter()
                                .map(|&student| {
                                    IntLinExpr::var(base_var(Var::StudentInGroup {
                                        student,
                                        group_list,
                                        group,
                                    }))
                                })
                                .sum();

                            let gi = IntLinExpr::<V>::var(base_var(Var::GroupInInterrogation {
                                slot,
                                week,
                                group,
                            }));
                            bundle = bundle.with_constraint(
                                gi.leq(&sum),
                                StructuralConstraint::ForbiddenGroup {
                                    group_list,
                                    group,
                                    slot,
                                    week,
                                    subject: subject_id,
                                }
                                .into(),
                            );
                        }
                    }
                }
            }
        }
    }
    bundle
}
