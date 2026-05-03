use crate::ids::{GlobalWeek, GroupNum};
use crate::types::{ConstraintDesc, ExtraVarName};
use collomatique_binding_colloscopes::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_ilp_modeler::bundle::ReifyError;
use collomatique_ilp_modeler::{IntConstraintBundle, Var as ModelerVar};
use collomatique_state_colloscopes::group_lists::GroupList;
use collomatique_state_colloscopes::ids::{GroupListId, Id, PeriodId, StudentId, SubjectId};
use collomatique_state_colloscopes::slots::Slot;
use std::collections::BTreeSet;

pub(crate) type V = ModelerVar<Var, ExtraVarName>;
pub(crate) type MyBundle = IntConstraintBundle<
    'static,
    Var,
    ExtraVarName,
    ConstraintDesc,
    VarEnv,
    ReifyError<Var, ExtraVarName>,
>;

pub(crate) fn base_var(v: Var) -> V {
    ModelerVar::Base(v)
}

pub(crate) fn extra_var(v: ExtraVarName) -> V {
    ModelerVar::Extra(v)
}

// ---- Helper functions reading from Parameters ----

pub(crate) fn week_to_period_id(env: &VarEnv, week: GlobalWeek) -> Option<(PeriodId, usize)> {
    collomatique_binding_colloscopes::tools::week_to_period_id(env, week.0)
}

pub(crate) fn group_list_for_interrogation(
    env: &VarEnv,
    subject: SubjectId,
    week: GlobalWeek,
) -> Option<GroupListId> {
    let (period_id, _) = week_to_period_id(env, week)?;
    let period_associations = env.group_lists.subjects_associations.get(&period_id)?;
    period_associations.get(&subject).copied()
}

pub(crate) fn groups_for_interrogation(
    env: &VarEnv,
    subject: SubjectId,
    week: GlobalWeek,
) -> Vec<GroupNum> {
    let Some(gl_id) = group_list_for_interrogation(env, subject, week) else {
        return vec![];
    };
    let Some(gl) = env.group_lists.group_list_map.get(&gl_id) else {
        return vec![];
    };
    groups_for_group_list(gl)
}

pub(crate) fn groups_for_group_list(gl: &GroupList) -> Vec<GroupNum> {
    (0..gl.params.group_names.len()).map(GroupNum).collect()
}

pub(crate) fn is_student_enrolled(
    env: &VarEnv,
    student: StudentId,
    subject: SubjectId,
    week: GlobalWeek,
) -> bool {
    let Some((period_id, _)) = week_to_period_id(env, week) else {
        return false;
    };
    env.assignments
        .period_map
        .get(&period_id)
        .and_then(|pa| pa.subject_map.get(&subject))
        .is_some_and(|students| students.contains(&student))
}

pub(crate) fn weeks_for_slot(
    env: &VarEnv,
    slot: &Slot,
    excluded_periods: &BTreeSet<PeriodId>,
) -> Vec<GlobalWeek> {
    collomatique_binding_colloscopes::tools::enumerate_weeks_for_slot(env, slot, excluded_periods)
        .into_iter()
        .map(GlobalWeek)
        .collect()
}

pub(crate) fn students_for_group_list(env: &VarEnv, gl: &GroupList) -> Vec<StudentId> {
    match &gl.filling {
        collomatique_state_colloscopes::group_lists::GroupListFilling::Automatic {
            excluded_students,
        } => env
            .students
            .student_map
            .keys()
            .filter(|s| !excluded_students.contains(s))
            .copied()
            .collect(),
        collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups } => {
            groups
                .iter()
                .flat_map(|g| g.students.iter().copied())
                .collect()
        }
    }
}

pub(crate) fn students_for_subject_period_group_list(
    env: &VarEnv,
    gl: &GroupList,
    subject: SubjectId,
    period: PeriodId,
) -> Vec<StudentId> {
    let enrolled = env
        .assignments
        .period_map
        .get(&period)
        .and_then(|pa| pa.subject_map.get(&subject));
    let Some(enrolled) = enrolled else {
        return vec![];
    };
    students_for_group_list(env, gl)
        .into_iter()
        .filter(|s| enrolled.contains(s))
        .collect()
}

pub(crate) fn subject_interrogation_params(
    env: &VarEnv,
    subject: SubjectId,
) -> Option<&collomatique_state_colloscopes::subjects::SubjectInterrogationParameters> {
    env.subjects
        .ordered_subject_list
        .iter()
        .find(|(id, _)| *id == subject)
        .and_then(|(_, s)| s.parameters.interrogation_parameters.as_ref())
}

// ---- Reified variable builders (lazy registration) ----

fn build_group_in_interrogation(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (&subject_id, subject_slots) in &env.slots.subject_map {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        for (slot_id, slot_data) in &subject_slots.ordered_slots {
            let slot = *slot_id;
            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                for group in groups_for_interrogation(env, subject_id, week) {
                    let var = ExtraVarName::GroupInInterrogation { slot, week, group };
                    bundle = bundle
                        .and_reified(var, move || {
                            let expr =
                                IntLinExpr::var(base_var(Var::GroupInInterrogationInternal {
                                    slot: slot.inner() as i32,
                                    week: week.0 as i32,
                                    group: group.0 as i32,
                                }));
                            vec![expr.eq(&IntLinExpr::constant(1))]
                        })
                        .expect("no duplicate extras");
                }
            }
        }
    }
    bundle
}

fn build_interrogation_has_groups(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (&subject_id, subject_slots) in &env.slots.subject_map {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        for (slot_id, slot_data) in &subject_slots.ordered_slots {
            let slot = *slot_id;
            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                let groups = groups_for_interrogation(env, subject_id, week);
                if groups.is_empty() {
                    continue;
                }
                let var = ExtraVarName::InterrogationHasGroups { slot, week };
                bundle = bundle
                    .and_reified(var, move || {
                        let sum: IntLinExpr<V> = groups
                            .iter()
                            .map(|&group| {
                                IntLinExpr::var(extra_var(ExtraVarName::GroupInInterrogation {
                                    slot,
                                    week,
                                    group,
                                }))
                            })
                            .sum();
                        vec![sum.geq(&IntLinExpr::constant(1))]
                    })
                    .expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_student_in_group(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for (&group_list, gl) in &env.group_lists.group_list_map {
        let students = students_for_group_list(env, gl);
        for group_index in 0..gl.params.group_names.len() {
            let group = GroupNum(group_index);
            for &student in &students {
                let var = ExtraVarName::StudentInGroup {
                    student,
                    group_list,
                    group,
                };
                bundle = bundle
                    .and_reified(var, move || {
                        let expr = IntLinExpr::var(base_var(Var::StudentGroup {
                            group_list: group_list.inner() as i32,
                            student: student.inner() as i32,
                        }));
                        let group_i64: i64 =
                            group.0.try_into().expect("group index should fit in i64");
                        vec![expr.eq(&IntLinExpr::constant(group_i64))]
                    })
                    .expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_group_has_students(env: &VarEnv) -> MyBundle {
    use collomatique_state_colloscopes::group_lists::GroupListFilling;

    let mut bundle = MyBundle::new();
    for (&group_list, gl) in &env.group_lists.group_list_map {
        for group_index in 0..gl.params.group_names.len() {
            let group = GroupNum(group_index);
            let var = ExtraVarName::GroupHasStudents { group_list, group };
            match &gl.filling {
                GroupListFilling::Prefilled { groups } => {
                    let has_students = groups
                        .get(group_index)
                        .is_some_and(|g| !g.students.is_empty());
                    if has_students {
                        bundle = bundle
                            .and_reified(var, move || {
                                vec![IntLinExpr::constant(0).leq(&IntLinExpr::constant(0))]
                            })
                            .expect("no duplicate extras");
                    } else {
                        bundle = bundle
                            .and_reified(var, move || {
                                vec![IntLinExpr::constant(1).leq(&IntLinExpr::constant(0))]
                            })
                            .expect("no duplicate extras");
                    }
                }
                GroupListFilling::Automatic { excluded_students } => {
                    let students: Vec<StudentId> = env
                        .students
                        .student_map
                        .keys()
                        .filter(|s| !excluded_students.contains(s))
                        .copied()
                        .collect();
                    bundle = bundle
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
                        .expect("no duplicate extras");
                }
            }
        }
    }
    bundle
}

fn build_student_at_interrogation_in_group(env: &VarEnv) -> MyBundle {
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
                let students = students_for_group_list(env, gl);
                for &student in &students {
                    for group_index in 0..gl.params.group_names.len() {
                        let group = GroupNum(group_index);
                        let var = ExtraVarName::StudentAtInterrogationInGroup {
                            student,
                            slot,
                            week,
                            group_list,
                            group,
                        };
                        bundle = bundle
                            .and_reified(var, move || {
                                let c1 = IntLinExpr::var(extra_var(ExtraVarName::StudentInGroup {
                                    student,
                                    group_list,
                                    group,
                                }))
                                .geq(&IntLinExpr::constant(1));
                                let c2 = IntLinExpr::var(extra_var(
                                    ExtraVarName::GroupInInterrogation { slot, week, group },
                                ))
                                .geq(&IntLinExpr::constant(1));
                                vec![c1, c2]
                            })
                            .expect("no duplicate extras");
                    }
                }
            }
        }
    }
    bundle
}

fn build_student_at_interrogation(env: &VarEnv) -> MyBundle {
    use collomatique_state_colloscopes::group_lists::GroupListFilling;

    let mut bundle = MyBundle::new();
    for (&subject_id, subject_slots) in &env.slots.subject_map {
        let Some(subject) = env.subjects.find_subject(subject_id) else {
            continue;
        };
        for (slot_id, slot_data) in &subject_slots.ordered_slots {
            let slot = *slot_id;
            for week in weeks_for_slot(env, slot_data, &subject.excluded_periods) {
                for &student in env.students.student_map.keys() {
                    let var = ExtraVarName::StudentAtInterrogation {
                        student,
                        slot,
                        week,
                    };

                    if !is_student_enrolled(env, student, subject_id, week) {
                        bundle = bundle
                            .and_reified(var, move || {
                                vec![IntLinExpr::constant(1).leq(&IntLinExpr::constant(0))]
                            })
                            .expect("no duplicate extras");
                        continue;
                    }

                    let Some(group_list) = group_list_for_interrogation(env, subject_id, week)
                    else {
                        bundle = bundle
                            .and_reified(var, move || {
                                vec![IntLinExpr::constant(0).geq(&IntLinExpr::constant(1))]
                            })
                            .expect("no duplicate extras");
                        continue;
                    };

                    let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
                        continue;
                    };

                    match &gl.filling {
                        GroupListFilling::Prefilled { groups } => {
                            let group = groups.iter().enumerate().find_map(|(i, g)| {
                                g.students.contains(&student).then(|| GroupNum(i))
                            });
                            match group {
                                Some(group) => {
                                    bundle = bundle
                                        .and_reified(var, move || {
                                            let expr = IntLinExpr::var(extra_var(
                                                ExtraVarName::GroupInInterrogation {
                                                    slot,
                                                    week,
                                                    group,
                                                },
                                            ));
                                            vec![expr.geq(&IntLinExpr::constant(1))]
                                        })
                                        .expect("no duplicate extras");
                                }
                                None => {
                                    bundle = bundle
                                        .and_reified(var, move || {
                                            vec![
                                                IntLinExpr::constant(0)
                                                    .geq(&IntLinExpr::constant(1)),
                                            ]
                                        })
                                        .expect("no duplicate extras");
                                }
                            }
                        }
                        GroupListFilling::Automatic { .. } => {
                            let group_count = gl.params.group_names.len();
                            bundle = bundle
                                .and_reified(var, move || {
                                    let sum: IntLinExpr<V> = (0..group_count)
                                        .map(|i| {
                                            let group = GroupNum(i);
                                            IntLinExpr::var(extra_var(
                                                ExtraVarName::StudentAtInterrogationInGroup {
                                                    student,
                                                    slot,
                                                    week,
                                                    group_list,
                                                    group,
                                                },
                                            ))
                                        })
                                        .sum();
                                    vec![sum.geq(&IntLinExpr::constant(1))]
                                })
                                .expect("no duplicate extras");
                        }
                    }
                }
            }
        }
    }
    bundle
}

// ---- Public API ----

pub fn build_native_extras(env: &VarEnv) -> MyBundle {
    let bundle = build_group_in_interrogation(env);
    let bundle = bundle
        .merge(build_interrogation_has_groups(env))
        .expect("no duplicate extras");
    let bundle = bundle
        .merge(build_student_in_group(env))
        .expect("no duplicate extras");
    let bundle = bundle
        .merge(build_group_has_students(env))
        .expect("no duplicate extras");
    let bundle = bundle
        .merge(build_student_at_interrogation_in_group(env))
        .expect("no duplicate extras");
    bundle
        .merge(build_student_at_interrogation(env))
        .expect("no duplicate extras")
}
