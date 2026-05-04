use crate::ids::GroupNum;
use crate::native_extras::{
    MyBundle, V, extra_var, group_list_for_interrogation, groups_for_group_list,
    students_for_subject_period_group_list, week_to_period_id, weeks_for_slot,
};
use crate::types::{ConstraintDesc, ExtraVarName};
use collomatique_binding_colloscopes::vars::VarEnv;
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_state_colloscopes::group_lists::GroupListFilling;

pub(super) fn build(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (&subject_id, subject_slots) in &env.slots.subject_map {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        for (slot_id, slot_data) in &subject_slots.ordered_slots {
            let slot = *slot_id;
            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                let Some(group_list) = group_list_for_interrogation(env, subject_id, week) else {
                    continue;
                };
                let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
                    continue;
                };
                let (period, _) = week_to_period_id(env, week).unwrap();

                match &gl.filling {
                    GroupListFilling::Prefilled { groups } => {
                        let enrolled = env
                            .assignments
                            .period_map
                            .get(&period)
                            .and_then(|pa| pa.subject_map.get(&subject_id));

                        for (group_index, prefilled_group) in groups.iter().enumerate() {
                            let group = GroupNum(group_index);
                            let has_enrolled = prefilled_group
                                .students
                                .iter()
                                .any(|s| enrolled.is_some_and(|e| e.contains(s)));

                            if !has_enrolled {
                                let expr = IntLinExpr::<V>::var(extra_var(
                                    ExtraVarName::GroupInInterrogation { slot, week, group },
                                ));
                                bundle = bundle.with_constraint(
                                    expr.eq(&IntLinExpr::constant(0)),
                                    ConstraintDesc::ForbiddenGroup {
                                        group_list,
                                        group,
                                        slot,
                                        week,
                                        subject: subject_id,
                                    },
                                );
                            }
                        }
                    }
                    GroupListFilling::Automatic { .. } => {
                        let students =
                            students_for_subject_period_group_list(env, gl, subject_id, period);

                        for group in groups_for_group_list(gl) {
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

                            let gi = IntLinExpr::<V>::var(extra_var(
                                ExtraVarName::GroupInInterrogation { slot, week, group },
                            ));
                            bundle = bundle.with_constraint(
                                gi.leq(&sum),
                                ConstraintDesc::ForbiddenGroup {
                                    group_list,
                                    group,
                                    slot,
                                    week,
                                    subject: subject_id,
                                },
                            );
                        }
                    }
                }
            }
        }
    }
    bundle
}
