use crate::ids::{GlobalWeek, GroupNum};
use crate::types::{ConstraintDesc, ReifiedVarName};
use collomatique_binding_colloscopes::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::IntLinExpr;
use collomatique_ilp_modeler::bundle::ReifyError;
use collomatique_ilp_modeler::{IntConstraintBundle, Var as ModelerVar};
use collomatique_state_colloscopes::ids::{
    GroupListId, Id, PeriodId, SlotId, StudentId, SubjectId,
};

pub(crate) type V = ModelerVar<Var, ReifiedVarName>;
pub(crate) type MyBundle = IntConstraintBundle<
    'static,
    Var,
    ReifiedVarName,
    ConstraintDesc,
    VarEnv,
    ReifyError<Var, ReifiedVarName>,
>;

pub(crate) fn base_var(v: Var) -> V {
    ModelerVar::Base(v)
}

pub(crate) fn extra_var(v: ReifiedVarName) -> V {
    ModelerVar::Extra(v)
}

// ---- Helper functions reading from Parameters ----

fn week_to_period_id(env: &VarEnv, week: GlobalWeek) -> Option<(PeriodId, usize)> {
    collomatique_binding_colloscopes::tools::week_to_period_id(env, week.0 as usize)
}

fn slot_subject(env: &VarEnv, slot: SlotId) -> Option<SubjectId> {
    env.slots
        .find_slot_subject_and_position(slot)
        .map(|(subject_id, _)| subject_id)
}

fn group_list_for_interrogation(
    env: &VarEnv,
    slot: SlotId,
    week: GlobalWeek,
) -> Option<GroupListId> {
    let subject_id = slot_subject(env, slot)?;
    let (period_id, _) = week_to_period_id(env, week)?;
    let period_associations = env.group_lists.subjects_associations.get(&period_id)?;
    period_associations.get(&subject_id).copied()
}

fn groups_for_interrogation(env: &VarEnv, slot: SlotId, week: GlobalWeek) -> Vec<GroupNum> {
    let Some(gl_id) = group_list_for_interrogation(env, slot, week) else {
        return vec![];
    };
    groups_for_group_list(env, gl_id)
}

pub(crate) fn groups_for_group_list(env: &VarEnv, group_list: GroupListId) -> Vec<GroupNum> {
    let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
        return vec![];
    };
    (0..gl.params.group_names.len() as u32)
        .map(GroupNum)
        .collect()
}

pub(crate) fn is_group_list_prefilled(env: &VarEnv, group_list: GroupListId) -> bool {
    env.group_lists
        .group_list_map
        .get(&group_list)
        .is_some_and(|gl| gl.filling.is_prefilled())
}

pub(crate) fn prefilled_student_count(
    env: &VarEnv,
    group_list: GroupListId,
    group: GroupNum,
) -> usize {
    let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
        return 0;
    };
    match &gl.filling {
        collomatique_state_colloscopes::group_lists::GroupListFilling::Prefilled { groups } => {
            groups.get(group.0 as usize).map_or(0, |g| g.students.len())
        }
        _ => 0,
    }
}

fn student_prefilled_group(
    env: &VarEnv,
    student: StudentId,
    group_list: GroupListId,
) -> Option<GroupNum> {
    let gl = env.group_lists.group_list_map.get(&group_list)?;
    gl.filling
        .find_student_group(student)
        .map(|n| GroupNum(n as u32))
}

pub(crate) fn students_for_automatic_group_list(
    env: &VarEnv,
    group_list: GroupListId,
) -> Vec<StudentId> {
    let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
        return vec![];
    };
    let excluded = gl.filling.excluded_students();
    env.students
        .student_map
        .keys()
        .filter(|s| !excluded.contains(s))
        .copied()
        .collect()
}

fn is_student_enrolled(env: &VarEnv, student: StudentId, slot: SlotId, week: GlobalWeek) -> bool {
    let Some(subject_id) = slot_subject(env, slot) else {
        return false;
    };
    let Some((period_id, _)) = week_to_period_id(env, week) else {
        return false;
    };
    env.assignments
        .period_map
        .get(&period_id)
        .and_then(|pa| pa.subject_map.get(&subject_id))
        .is_some_and(|students| students.contains(&student))
}

fn all_slots(env: &VarEnv) -> Vec<SlotId> {
    env.slots
        .subject_map
        .values()
        .flat_map(|ss| ss.ordered_slots.iter().map(|(id, _)| *id))
        .collect()
}

fn weeks_for_slot(env: &VarEnv, slot: SlotId) -> Vec<GlobalWeek> {
    Var::enumerate_weeks_for_slot(env, &(slot.inner() as i32))
        .into_iter()
        .map(|w| GlobalWeek(w as u32))
        .collect()
}

pub(crate) fn all_group_lists(env: &VarEnv) -> Vec<GroupListId> {
    env.group_lists.group_list_map.keys().copied().collect()
}

fn all_students(env: &VarEnv) -> Vec<StudentId> {
    env.students.student_map.keys().copied().collect()
}

pub(crate) fn students_for_group_list(env: &VarEnv, group_list: GroupListId) -> Vec<StudentId> {
    let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
        return vec![];
    };
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
    group_list: GroupListId,
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
    students_for_group_list(env, group_list)
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
    for slot in all_slots(env) {
        for week in weeks_for_slot(env, slot) {
            for group in groups_for_interrogation(env, slot, week) {
                let var = ReifiedVarName::GroupInInterrogation { slot, week, group };
                bundle = bundle
                    .and_reified(var, move || {
                        let expr = IntLinExpr::var(base_var(Var::GroupInInterrogationInternal {
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
    bundle
}

fn build_interrogation_has_groups(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for slot in all_slots(env) {
        for week in weeks_for_slot(env, slot) {
            let groups = groups_for_interrogation(env, slot, week);
            if groups.is_empty() {
                continue;
            }
            let var = ReifiedVarName::InterrogationHasGroups { slot, week };
            bundle = bundle
                .and_reified(var, move || {
                    let sum: IntLinExpr<V> = groups
                        .iter()
                        .map(|&group| {
                            IntLinExpr::var(extra_var(ReifiedVarName::GroupInInterrogation {
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
    bundle
}

fn build_student_in_group(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for &group_list in all_group_lists(env).iter() {
        let student_ids = Var::compute_student_ids(env, &(group_list.inner() as i32));
        let groups = groups_for_group_list(env, group_list);
        for &student_id_i32 in &student_ids {
            let student = unsafe { StudentId::new(student_id_i32 as u64) };
            for &group in &groups {
                let var = ReifiedVarName::StudentInGroup {
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
                        vec![expr.eq(&IntLinExpr::constant(group.0 as i64))]
                    })
                    .expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_group_has_students(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for &group_list in all_group_lists(env).iter() {
        let groups = groups_for_group_list(env, group_list);
        for &group in &groups {
            let var = ReifiedVarName::GroupHasStudents { group_list, group };
            if is_group_list_prefilled(env, group_list) {
                let count = prefilled_student_count(env, group_list, group);
                if count >= 1 {
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
            } else {
                let students = students_for_automatic_group_list(env, group_list);
                bundle = bundle
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
                    .expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_student_at_interrogation_in_group(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for slot in all_slots(env) {
        for week in weeks_for_slot(env, slot) {
            let Some(group_list) = group_list_for_interrogation(env, slot, week) else {
                continue;
            };
            let student_ids = Var::compute_student_ids(env, &(group_list.inner() as i32));
            let groups = groups_for_group_list(env, group_list);
            for &student_id_i32 in &student_ids {
                let student = unsafe { StudentId::new(student_id_i32 as u64) };
                for &group in &groups {
                    let var = ReifiedVarName::StudentAtInterrogationInGroup {
                        student,
                        slot,
                        week,
                        group_list,
                        group,
                    };
                    bundle =
                        bundle
                            .and_reified(var, move || {
                                let c1 =
                                    IntLinExpr::var(extra_var(ReifiedVarName::StudentInGroup {
                                        student,
                                        group_list,
                                        group,
                                    }))
                                    .geq(&IntLinExpr::constant(1));
                                let c2 = IntLinExpr::var(extra_var(
                                    ReifiedVarName::GroupInInterrogation { slot, week, group },
                                ))
                                .geq(&IntLinExpr::constant(1));
                                vec![c1, c2]
                            })
                            .expect("no duplicate extras");
                }
            }
        }
    }
    bundle
}

fn build_student_at_interrogation(env: &VarEnv) -> MyBundle {
    let mut bundle = MyBundle::new();
    for slot in all_slots(env) {
        for week in weeks_for_slot(env, slot) {
            for &student in all_students(env).iter() {
                let var = ReifiedVarName::StudentAtInterrogation {
                    student,
                    slot,
                    week,
                };

                if !is_student_enrolled(env, student, slot, week) {
                    bundle = bundle
                        .and_reified(var, move || {
                            vec![IntLinExpr::constant(1).leq(&IntLinExpr::constant(0))]
                        })
                        .expect("no duplicate extras");
                    continue;
                }

                let Some(group_list) = group_list_for_interrogation(env, slot, week) else {
                    bundle = bundle
                        .and_reified(var, move || {
                            vec![IntLinExpr::constant(0).geq(&IntLinExpr::constant(1))]
                        })
                        .expect("no duplicate extras");
                    continue;
                };

                if is_group_list_prefilled(env, group_list) {
                    match student_prefilled_group(env, student, group_list) {
                        Some(group) => {
                            bundle = bundle
                                .and_reified(var, move || {
                                    let expr = IntLinExpr::var(extra_var(
                                        ReifiedVarName::GroupInInterrogation { slot, week, group },
                                    ));
                                    vec![expr.geq(&IntLinExpr::constant(1))]
                                })
                                .expect("no duplicate extras");
                        }
                        None => {
                            bundle = bundle
                                .and_reified(var, move || {
                                    vec![IntLinExpr::constant(0).geq(&IntLinExpr::constant(1))]
                                })
                                .expect("no duplicate extras");
                        }
                    }
                } else {
                    let groups = groups_for_group_list(env, group_list);
                    bundle = bundle
                        .and_reified(var, move || {
                            let sum: IntLinExpr<V> = groups
                                .iter()
                                .map(|&group| {
                                    IntLinExpr::var(extra_var(
                                        ReifiedVarName::StudentAtInterrogationInGroup {
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
