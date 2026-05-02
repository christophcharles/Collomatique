use crate::ids::{GlobalWeek, GroupNum};
use crate::types::{ConstraintDesc, ReifiedVarName};
use collomatique_binding_colloscopes::vars::{Var, VarEnv};
use collomatique_ilp::int_linexpr::{IntConstraint, IntLinExpr};
use collomatique_ilp_modeler::Var as ModelerVar;
use collomatique_ilp_modeler::bundle::{ConstraintBundle, IntConstraintBundle, ReifyError};
use collomatique_state_colloscopes::ids::{
    GroupListId, Id, PeriodId, SlotId, StudentId, SubjectId,
};

type V = ModelerVar<Var, ReifiedVarName>;
type Bundle<'m, Db> =
    ConstraintBundle<'m, Var, ReifiedVarName, ConstraintDesc, Db, ReifyError<Var, ReifiedVarName>>;
type IntBundle<'m, Db> = IntConstraintBundle<
    'm,
    Var,
    ReifiedVarName,
    ConstraintDesc,
    Db,
    ReifyError<Var, ReifiedVarName>,
>;

fn base_var(v: Var) -> V {
    ModelerVar::Base(v)
}

fn extra_var(v: ReifiedVarName) -> V {
    ModelerVar::Extra(v)
}

fn desc(name: &'static str) -> ConstraintDesc {
    ConstraintDesc::Native(name)
}

fn reify_one<'m, Db: Sync + 'm>(
    constraints: Vec<IntConstraint<V>>,
    name: &'static str,
    var: ReifiedVarName,
) -> IntBundle<'m, Db> {
    let items: Vec<_> = constraints.into_iter().map(|c| (c, desc(name))).collect();
    IntConstraintBundle::from_constraints(items)
        .reify(var)
        .expect("reification should not fail eagerly")
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

fn groups_for_group_list(env: &VarEnv, group_list: GroupListId) -> Vec<GroupNum> {
    let Some(gl) = env.group_lists.group_list_map.get(&group_list) else {
        return vec![];
    };
    (0..gl.params.group_names.len() as u32)
        .map(GroupNum)
        .collect()
}

fn is_group_list_prefilled(env: &VarEnv, group_list: GroupListId) -> bool {
    env.group_lists
        .group_list_map
        .get(&group_list)
        .is_some_and(|gl| gl.filling.is_prefilled())
}

fn prefilled_student_count(env: &VarEnv, group_list: GroupListId, group: GroupNum) -> usize {
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

fn students_for_automatic_group_list(env: &VarEnv, group_list: GroupListId) -> Vec<StudentId> {
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

fn all_group_lists(env: &VarEnv) -> Vec<GroupListId> {
    env.group_lists.group_list_map.keys().copied().collect()
}

fn all_students(env: &VarEnv) -> Vec<StudentId> {
    env.students.student_map.keys().copied().collect()
}

// ---- Reified variable builders ----

fn build_group_in_interrogation<'m, Db: Sync + 'm>(env: &VarEnv) -> IntBundle<'m, Db> {
    let mut bundle = IntBundle::new();
    for slot in all_slots(env) {
        for week in weeks_for_slot(env, slot) {
            for group in groups_for_interrogation(env, slot, week) {
                let var = ReifiedVarName::GroupInInterrogation { slot, week, group };
                let expr = IntLinExpr::var(base_var(Var::GroupInInterrogationInternal {
                    slot: slot.inner() as i32,
                    week: week.0 as i32,
                    group: group.0 as i32,
                }));
                let constraint = expr.eq(&IntLinExpr::constant(1));
                let sub = reify_one(vec![constraint], "group_in_interrogation", var);
                bundle = bundle.merge(sub).expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_interrogation_has_groups<'m, Db: Sync + 'm>(env: &VarEnv) -> IntBundle<'m, Db> {
    let mut bundle = IntBundle::new();
    for slot in all_slots(env) {
        for week in weeks_for_slot(env, slot) {
            let groups = groups_for_interrogation(env, slot, week);
            if groups.is_empty() {
                continue;
            }
            let var = ReifiedVarName::InterrogationHasGroups { slot, week };
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
            let constraint = sum.geq(&IntLinExpr::constant(1));
            let sub = reify_one(vec![constraint], "interrogation_has_groups", var);
            bundle = bundle.merge(sub).expect("no duplicate extras");
        }
    }
    bundle
}

fn build_student_in_group<'m, Db: Sync + 'm>(env: &VarEnv) -> IntBundle<'m, Db> {
    let mut bundle = IntBundle::new();
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
                let expr = IntLinExpr::var(base_var(Var::StudentGroup {
                    group_list: group_list.inner() as i32,
                    student: student.inner() as i32,
                }));
                let constraint = expr.eq(&IntLinExpr::constant(group.0 as i64));
                let sub = reify_one(vec![constraint], "student_in_group", var);
                bundle = bundle.merge(sub).expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_group_has_students<'m, Db: Sync + 'm>(env: &VarEnv) -> IntBundle<'m, Db> {
    let mut bundle = IntBundle::new();
    for &group_list in all_group_lists(env).iter() {
        let groups = groups_for_group_list(env, group_list);
        for &group in &groups {
            let var = ReifiedVarName::GroupHasStudents { group_list, group };
            if is_group_list_prefilled(env, group_list) {
                let count = prefilled_student_count(env, group_list, group);
                if count >= 1 {
                    // Trivially true — reify with tautological constraint
                    let constraint = IntLinExpr::constant(0).leq(&IntLinExpr::constant(0));
                    let sub = reify_one(vec![constraint], "group_has_students", var);
                    bundle = bundle.merge(sub).expect("no duplicate extras");
                } else {
                    // Trivially false
                    let constraint = IntLinExpr::constant(1).leq(&IntLinExpr::constant(0));
                    let sub = reify_one(vec![constraint], "group_has_students", var);
                    bundle = bundle.merge(sub).expect("no duplicate extras");
                }
            } else {
                let students = students_for_automatic_group_list(env, group_list);
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
                let constraint = sum.geq(&IntLinExpr::constant(1));
                let sub = reify_one(vec![constraint], "group_has_students", var);
                bundle = bundle.merge(sub).expect("no duplicate extras");
            }
        }
    }
    bundle
}

fn build_student_at_interrogation_in_group<'m, Db: Sync + 'm>(env: &VarEnv) -> IntBundle<'m, Db> {
    let mut bundle = IntBundle::new();
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
                    // AND of two >= 1 constraints (uses inequalities to reduce helper variables)
                    let c1 = IntLinExpr::var(extra_var(ReifiedVarName::StudentInGroup {
                        student,
                        group_list,
                        group,
                    }))
                    .geq(&IntLinExpr::constant(1));
                    let c2 = IntLinExpr::var(extra_var(ReifiedVarName::GroupInInterrogation {
                        slot,
                        week,
                        group,
                    }))
                    .geq(&IntLinExpr::constant(1));
                    let sub = reify_one(vec![c1, c2], "student_at_interrogation_in_group", var);
                    bundle = bundle.merge(sub).expect("no duplicate extras");
                }
            }
        }
    }
    bundle
}

fn build_student_at_interrogation<'m, Db: Sync + 'm>(env: &VarEnv) -> IntBundle<'m, Db> {
    let mut bundle = IntBundle::new();
    for slot in all_slots(env) {
        for week in weeks_for_slot(env, slot) {
            for &student in all_students(env).iter() {
                let var = ReifiedVarName::StudentAtInterrogation {
                    student,
                    slot,
                    week,
                };

                if !is_student_enrolled(env, student, slot, week) {
                    // Not enrolled — infeasible (reified var forced to 0)
                    let constraint = IntLinExpr::constant(1).leq(&IntLinExpr::constant(0));
                    let sub = reify_one(vec![constraint], "student_at_interrogation", var);
                    bundle = bundle.merge(sub).expect("no duplicate extras");
                    continue;
                }

                let Some(group_list) = group_list_for_interrogation(env, slot, week) else {
                    // No group list — constraint is 0 >= 1, always false
                    let constraint = IntLinExpr::constant(0).geq(&IntLinExpr::constant(1));
                    let sub = reify_one(vec![constraint], "student_at_interrogation", var);
                    bundle = bundle.merge(sub).expect("no duplicate extras");
                    continue;
                };

                if is_group_list_prefilled(env, group_list) {
                    // Prefilled: student must be in their predetermined group
                    match student_prefilled_group(env, student, group_list) {
                        Some(group) => {
                            let expr =
                                IntLinExpr::var(extra_var(ReifiedVarName::GroupInInterrogation {
                                    slot,
                                    week,
                                    group,
                                }));
                            let constraint = expr.geq(&IntLinExpr::constant(1));
                            let sub = reify_one(vec![constraint], "student_at_interrogation", var);
                            bundle = bundle.merge(sub).expect("no duplicate extras");
                        }
                        None => {
                            // Student not in any prefilled group — use GroupNum(-1) equivalent
                            // which makes the constraint always false
                            let constraint = IntLinExpr::constant(0).geq(&IntLinExpr::constant(1));
                            let sub = reify_one(vec![constraint], "student_at_interrogation", var);
                            bundle = bundle.merge(sub).expect("no duplicate extras");
                        }
                    }
                } else {
                    // Automatic: sum over groups of StudentAtInterrogationInGroup >= 1
                    let groups = groups_for_group_list(env, group_list);
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
                    let constraint = sum.geq(&IntLinExpr::constant(1));
                    let sub = reify_one(vec![constraint], "student_at_interrogation", var);
                    bundle = bundle.merge(sub).expect("no duplicate extras");
                }
            }
        }
    }
    bundle
}

// ---- Public API ----

pub fn build_native_extras_bundle<'m, Db: Sync + 'm>(env: &VarEnv) -> Bundle<'m, Db> {
    let mut bundle = IntBundle::new();

    bundle = bundle
        .merge(build_group_in_interrogation(env))
        .expect("no duplicate extras");
    bundle = bundle
        .merge(build_interrogation_has_groups(env))
        .expect("no duplicate extras");
    bundle = bundle
        .merge(build_student_in_group(env))
        .expect("no duplicate extras");
    bundle = bundle
        .merge(build_group_has_students(env))
        .expect("no duplicate extras");
    bundle = bundle
        .merge(build_student_at_interrogation_in_group(env))
        .expect("no duplicate extras");
    bundle = bundle
        .merge(build_student_at_interrogation(env))
        .expect("no duplicate extras");

    bundle.into_general()
}
